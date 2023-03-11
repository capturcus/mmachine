use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::Ordering::SeqCst;

use crate::bits::{MValue, BITNESS};
use crate::bus::Bus;
use std::sync::atomic::{AtomicBool, AtomicUsize, AtomicPtr};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::Arc;

pub const REGISTERS_NUM: usize = 8;
pub const RAM_SIZE: usize = 1 << BITNESS;

pub enum ControlCable {
    Halt,
    MemoryAddressIn,
    RamIn,
    RamOut,
    MemoryIsIO,
    AddMul,
    SubDiv,
    AluOut,

    Interrupt,

    Equal,
    Greater,

    RegBase,
}

use crate::ControlCable::*;

pub const CONTROL_CABLES_SIZE: usize =
    std::mem::variant_count::<ControlCable>() + REGISTERS_NUM * 4 - 1;
pub const INSTRUCTION_REG_NUM: usize = REGISTERS_NUM - 1;
pub const STACK_POINTER_REG_NUM: usize = REGISTERS_NUM - 2;
pub const PROGRAM_COUNTER: usize = REGISTERS_NUM - 3;

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
    pub rx: Receiver<()>,
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

pub struct RegisterComponent {
    pub reg_num: usize,
    pub value: MValue,
    pub alu_tx: Arc<Mutex<Sender<(usize, MValue)>>>,
    pub sent_to_alu: Arc<AtomicUsize>,
}

pub fn reg_in(reg_num: usize) -> usize {
    RegBase as usize + 4 * reg_num
}

pub fn reg_out(reg_num: usize) -> usize {
    RegBase as usize + 4 * reg_num + 1
}

pub fn reg_inc(reg_num: usize) -> usize {
    RegBase as usize + 4 * reg_num + 2
}

pub fn reg_dec(reg_num: usize) -> usize {
    RegBase as usize + 4 * reg_num + 3
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
        if cables[reg_inc(self.reg_num)].load(SeqCst) {
            self.value.add(&MValue::from_u32(1));
        }
        if cables[reg_dec(self.reg_num)].load(SeqCst) {
            self.value.sub(&MValue::from_u32(1));
        }
        println!("register {} is now {}", self.reg_num, self.value.as_u32());
    }
}

pub struct AluComponent {
    pub reg_a: MValue,
    pub reg_b: MValue,
}

impl AluComponent {
    pub fn run(
        &self,
        reg_rx: Receiver<(usize, MValue)>,
        alu_clock_tx: Sender<()>,
        ctrl_tx: Sender<MValue>,
    ) {
        loop {
            let (reg_num, mvalue) = reg_rx.recv().unwrap();
            if reg_num == 0 {
                self.reg_a.set(&mvalue);
            }
            if reg_num == 1 {
                self.reg_b.set(&mvalue);
            }
            if reg_num == INSTRUCTION_REG_NUM {
                ctrl_tx.send(mvalue).unwrap();
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

pub struct ControlComponent<'a> {
    pub clock_rx: Receiver<()>,
    pub txs: Vec<Sender<()>>,
    pub finished: &'a AtomicUsize,
    pub alu_clock_rx: Receiver<()>,
    pub sent_to_alu: Arc<AtomicUsize>,
    pub cables: &'a ControlCables,
    pub bus: Arc<Bus>,
    pub test_instruction: Vec<Vec<usize>>,
    pub microcode_counter: AtomicUsize,
    pub instruction_register: MValue,
}

impl<'a> ControlComponent<'a> {
    pub fn run(&self, ctrl_rx: Receiver<MValue>) {
        loop {
            self.set_cables(self.cables);
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
            match ctrl_rx.try_recv() {
                Ok(mvalue) => self.instruction_register.set(&mvalue),
                Err(_) => {}
            }
        }
    }
}

impl<'a> ControlComponent<'a> {
    fn set_cables(&self, cables: &ControlCables) {
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
    pub output_tx: Arc<Mutex<Sender<(MValue, MValue)>>>,
    pub input_req_tx: Arc<Mutex<Sender<MValue>>>,
    pub input_rx: Arc<Mutex<Receiver<Option<MValue>>>>,
}

impl CpuComponent for RamComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables) {
        if cables.load(MemoryAddressIn) {
            bus.read_into(&self.memory_address_register);
            if !cables.load(MemoryIsIO) {
                let memory_index = self.memory_address_register.as_u32() as usize;
                self.ram_register.set(&self.memory[memory_index]);
            }
        }
        if cables.load(RamIn) {
            bus.read_into(&self.ram_register);
            if cables.load(MemoryIsIO) {
                self.output_tx.lock()
                    .send((self.memory_address_register.clone(), self.ram_register.clone())).unwrap();
            } else {
                let memory_index = self.memory_address_register.as_u32() as usize;
                self.memory[memory_index].set(&self.ram_register);
            }
        }
        if cables.load(RamOut) {
            if cables.load(MemoryIsIO) {
                self.input_req_tx.lock().send(self.memory_address_register.clone()).unwrap();
                let v = self.input_rx.lock().recv().unwrap();
                if v.is_some() {
                    bus.write_from(&v.unwrap());
                }
            } else {
                let memory_index = self.memory_address_register.as_u32() as usize;
                self.ram_register.set(&self.memory[memory_index]);
                bus.write_from(&self.ram_register);
            }
        }
    }
}
