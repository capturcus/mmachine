use parking_lot::{Mutex};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::mpsc::Sender;

use crate::bits::MValue;
use crate::bus::Bus;
use std::sync::atomic::{AtomicUsize, AtomicBool};
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

const REGISTERS_NUM: usize = 4;
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
    RegBase,
}

pub const CONTROL_CABLES_SIZE: usize =
    std::mem::variant_count::<ControlCable>() + REGISTERS_NUM * 2 - 1;

pub type ControlCables = [AtomicBool; CONTROL_CABLES_SIZE];

trait ControlCablesExt {
    fn reset(&self);
}

impl ControlCablesExt for ControlCables {
    fn reset(&self) {
        for i in 0..CONTROL_CABLES_SIZE {
            self[i].store(false, SeqCst);
        }
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
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables) {}
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
        args.rx.recv().unwrap();
        component.step(args.bus.clone(), &args.cables);
        args.finished.fetch_add(1, SeqCst);
        args.clock_tx.send(()).unwrap();
    });
}

pub struct ProgramCounterComponent {
    pub program_counter: MValue,
}

impl CpuComponent for ProgramCounterComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables) {
        if cables[ControlCable::CounterEnable as usize].load(SeqCst) {
            self.program_counter
                .set(&MValue::from_u32(self.program_counter.as_u32() + 1));
        }
        if cables[ControlCable::CounterIn as usize].load(SeqCst) {
            bus.read_into(&self.program_counter);
        }
        if cables[ControlCable::CounterOut as usize].load(SeqCst) {
            bus.write_from(&self.program_counter);
        }
    }
}

pub struct RegisterComponent {
    pub reg_num: usize,
    pub value: MValue,
    pub alu_tx: Arc<Mutex<mpsc::Sender<(usize, MValue)>>>,
}

impl CpuComponent for RegisterComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables) {
        if cables[ControlCable::RegBase as usize + 2 * self.reg_num].load(SeqCst) {
            bus.read_into(&self.value);
            {
                let lock = self.alu_tx.lock();
                lock.send((self.reg_num, self.value.clone())).unwrap();
            }
        }
        if cables[ControlCable::RegBase as usize + 2 * self.reg_num + 1].load(SeqCst) {
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
    pub fn run(&self, reg_rx: mpsc::Receiver<(usize, MValue)>) {
        loop {
            let (reg_num, mvalue) = reg_rx.recv().unwrap();
            if reg_num == 0 {
                self.reg_a.set(&mvalue);
            }
            if reg_num == 1 {
                self.reg_b.set(&mvalue);
            }
        }
    }
}

impl CpuComponent for AluComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables) {
        if cables[ControlCable::AluOut as usize].load(SeqCst) {
            let ret = self.reg_a.clone();
            if cables[ControlCable::AddMul as usize].load(SeqCst) {
                // multiplication or division
                if cables[ControlCable::SubDiv as usize].load(SeqCst) {
                    // division
                    ret.div(&self.reg_b);
                } else {
                    // multiplication
                    ret.mul(&self.reg_b);
                }
            } else {
                // addition or subtraction
                if cables[ControlCable::SubDiv as usize].load(SeqCst) {
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
    pub clock_ctrl_rx: Receiver<()>,
}

impl<'a> ClockComponent<'a> {
    pub fn run(&self) {
        loop {
            // self.clock_ctrl_rx.recv().unwrap();
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
        }
    }
}

pub struct ControlComponent {}

fn reg_in(reg_num: usize) -> usize {
    ControlCable::RegBase as usize + 2 * reg_num
}

fn reg_out(reg_num: usize) -> usize {
    ControlCable::RegBase as usize + 2 * reg_num + 1
}

impl CpuComponent for ControlComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables) {
        cables.reset();
        cables[reg_out(3)].store(true, SeqCst);
        cables[reg_in(0)].store(true, SeqCst);
    }
}
