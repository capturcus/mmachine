use parking_lot::Mutex;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::mpsc::Sender;

use crate::bits::{MValue, BITNESS};
use crate::bus::Bus;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

const REGISTERS_NUM: usize = 4;
pub const RAM_SIZE: usize = 1 << BITNESS;
const STACK_SIZE: usize = 1 << (BITNESS - 4);

pub enum ControlCable {
    Halt,
    MemoryAddressIn,
    RamIn,
    RamOut,
    IntructionRegisterOut,
    IntructionRegisterIn,
    AddMul,
    SubDiv,
    AluOut,
    CounterEnable,
    CounterOut,
    CounterIn,
    InputOut,
    OutputIn,
    StackIn,
    StackOut,
    RegBase,
}

use crate::ControlCable::*;

pub const CONTROL_CABLES_SIZE: usize =
    std::mem::variant_count::<ControlCable>() + REGISTERS_NUM * 2 - 1;

pub type ControlCables = [AtomicBool; CONTROL_CABLES_SIZE];

trait ControlCablesExt {
    fn reset(&self);
    fn load(&self, c: ControlCable) -> bool;
    fn store(&self, val: bool, c: ControlCable);
}

impl ControlCablesExt for ControlCables {
    fn reset(&self) {
        for i in 0..CONTROL_CABLES_SIZE {
            self[i].store(false, SeqCst);
        }
    }
    fn load(&self, c: ControlCable) -> bool {
        self[c as usize].load(SeqCst)
    }

    fn store(&self, val: bool, c: ControlCable) {
        self[c as usize].store(val, SeqCst);
    }
}

pub struct CpuComponentArgs<'a> {
    pub cables: &'a ControlCables,
    pub bus: Arc<Bus>,
    pub rx: mpsc::Receiver<()>,
    pub finished: &'a AtomicUsize,
    pub clock_tx: Sender<()>,
}

pub trait CpuComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables);
}

pub fn start_cpu_component<
    'a,
    T: CpuComponent + std::marker::Send + std::marker::Sync + ?Sized + 'static,
>(
    args: CpuComponentArgs<'a>,
    component: Arc<T>,
    scope: &'a std::thread::Scope<'a, '_>,
) {
    scope.spawn(move || loop {
        match args.rx.recv() {
            Ok(_) => {
                component.step(args.bus.clone(), &args.cables);
                args.finished.fetch_add(1, SeqCst);
                args.clock_tx.send(()).unwrap();
            }
            Err(_) => {
                return;
            }
        }
    });
}

pub struct ProgramCounterComponent {
    pub program_counter: MValue,
}

impl CpuComponent for ProgramCounterComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables) {
        if cables.load(CounterEnable) {
            self.program_counter
                .set(&MValue::from_u32(self.program_counter.as_u32() + 1));
        }
        if cables.load(CounterIn) {
            bus.read_into(&self.program_counter);
        }
        if cables.load(CounterOut) {
            bus.write_from(&self.program_counter);
        }
    }
}

pub struct RegisterComponent {
    pub reg_num: usize,
    pub value: MValue,
    pub alu_tx: Arc<Mutex<mpsc::Sender<(usize, MValue)>>>,
    pub sent_to_alu: Arc<AtomicUsize>,
}

fn reg_in(reg_num: usize) -> usize {
    RegBase as usize + 2 * reg_num
}

fn reg_out(reg_num: usize) -> usize {
    RegBase as usize + 2 * reg_num + 1
}

impl<'a> CpuComponent for RegisterComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables) {
        if cables[reg_in(self.reg_num)].load(SeqCst) {
            bus.read_into(&self.value);
            {
                let lock = self.alu_tx.lock();
                lock.send((self.reg_num, self.value.clone())).unwrap();
                self.sent_to_alu.fetch_add(1, SeqCst);
            }
        }
        if cables[reg_out(self.reg_num)].load(SeqCst) {
            bus.write_from(&self.value);
        }
        println!("register {} is now {}", self.reg_num, self.value.as_u32());
    }
}

pub struct AluComponent {
    pub reg_a: MValue,
    pub reg_b: MValue,
}

impl AluComponent {
    pub fn run(&self, reg_rx: mpsc::Receiver<(usize, MValue)>, alu_clock_tx: mpsc::Sender<()>) {
        loop {
            let (reg_num, mvalue) = reg_rx.recv().unwrap();
            if reg_num == 0 {
                self.reg_a.set(&mvalue);
            }
            if reg_num == 1 {
                self.reg_b.set(&mvalue);
            }
            alu_clock_tx.send(()).unwrap();
        }
    }
}

impl CpuComponent for AluComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables) {
        if cables.load(AluOut) {
            let ret = self.reg_a.clone();
            if cables.load(AddMul) {
                // multiplication or division
                if cables.load(SubDiv) {
                    // division
                    ret.div(&self.reg_b);
                } else {
                    // multiplication
                    ret.mul(&self.reg_b);
                }
            } else {
                // addition or subtraction
                if cables.load(SubDiv) {
                    // subtraction
                    ret.sub(&self.reg_b);
                } else {
                    // addition
                    ret.add(&self.reg_b);
                }
            }
            bus.write_from(&ret);
        }
    }
}

pub struct ClockComponent<'a> {
    pub clock_rx: Receiver<()>,
    pub txs: Vec<Sender<()>>,
    pub finished: &'a AtomicUsize,
    pub alu_clock_rx: Receiver<()>,
    pub sent_to_alu: Arc<AtomicUsize>,
    pub cables: &'a ControlCables,
}

impl<'a> ClockComponent<'a> {
    pub fn run(&self) {
        loop {
            if self.cables.load(Halt) {
                println!("clock: halt");
                break;
            }
            println!("\n### new clock cycle ###");
            for t in &self.txs {
                t.send(()).unwrap();
            }
            loop {
                self.clock_rx.recv().unwrap();
                let amount_finished = self.finished.load(SeqCst);
                if amount_finished == self.txs.len() {
                    self.finished.store(0, SeqCst);
                    break;
                }
            }
            for _ in 0..self.sent_to_alu.load(SeqCst) {
                self.alu_clock_rx.recv().unwrap();
            }
            self.sent_to_alu.store(0, SeqCst);
        }
    }
}

pub struct ControlComponent {
    test_instruction: Vec<Vec<usize>>,
    microcode_counter: AtomicUsize,
}

impl ControlComponent {
    pub fn new() -> Self {
        let mut ret = ControlComponent {
            test_instruction: Vec::new(),
            microcode_counter: AtomicUsize::new(0),
        };
        ret.test_instruction = vec![
            vec![MemoryAddressIn as usize, reg_out(0)],
            vec![RamIn as usize, reg_out(1)],
            vec![MemoryAddressIn as usize, reg_out(2)],
            vec![MemoryAddressIn as usize, reg_out(0)],
            vec![RamOut as usize, reg_in(3)],
            vec![Halt as usize],
        ];
        ret
    }
}

impl CpuComponent for ControlComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables) {
        cables.reset();
        let current_microcodes = &self.test_instruction[self.microcode_counter.load(SeqCst)];
        for m in current_microcodes {
            cables[*m].store(true, SeqCst);
        }
        self.microcode_counter.fetch_add(1, SeqCst);
    }
}

pub struct RamComponent {
    pub memory: Box<[MValue; RAM_SIZE]>,
    pub memory_address_register: MValue,
    pub ram_register: MValue,
}

impl CpuComponent for RamComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables) {
        if cables.load(MemoryAddressIn) {
            bus.read_into(&self.memory_address_register);
            let memory_index = self.memory_address_register.as_u32() as usize;
            self.ram_register.set(&self.memory[memory_index]);
        }
        if cables.load(RamIn) {
            bus.read_into(&self.ram_register);
            let memory_index = self.memory_address_register.as_u32() as usize;
            self.memory[memory_index].set(&self.ram_register);
        }
        if cables.load(RamOut) {
            let memory_index = self.memory_address_register.as_u32() as usize;
            self.ram_register.set(&self.memory[memory_index]);
            bus.write_from(&self.ram_register);
        }
    }
}
