use parking_lot::{RwLock, Mutex};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::mpsc::Sender;

use crate::bits::MValue;
use crate::bus::Bus;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc;
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

pub const CONTROL_CABLES_SIZE: usize = std::mem::variant_count::<ControlCable>() + REGISTERS_NUM * 2 - 1;

pub type ControlCables = [bool; CONTROL_CABLES_SIZE];

pub struct CpuComponentArgs<'a> {
    pub cables: &'a RwLock<ControlCables>,
    pub bus: Arc<Bus>,
    pub rx: mpsc::Receiver<()>,
    pub finished: &'a AtomicUsize,
    pub clock_tx: Sender<()>,
}

pub trait CpuComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables);
}

pub fn start_cpu_component<'a, T: CpuComponent + std::marker::Send + std::marker::Sync + ?Sized + 'static>(
    args: CpuComponentArgs<'a>,
    component: Arc<T>,
    scope: &'a std::thread::Scope<'a, '_>,
) {
    scope.spawn(move || loop {
        args.rx.recv().unwrap();
        component.step(args.bus.clone(), &args.cables.read());
        args.finished.fetch_add(1, SeqCst);
        args.clock_tx.send(()).unwrap();
    });
}

pub struct ProgramCounterComponent {
    pub program_counter: MValue,
}

impl CpuComponent for ProgramCounterComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables) {
        if cables[ControlCable::CounterEnable as usize] {
            self.program_counter
                .set(&MValue::from_u32(self.program_counter.as_u32() + 1));
        }
        if cables[ControlCable::CounterIn as usize] {
            bus.read_into(&self.program_counter);
        }
        if cables[ControlCable::CounterOut as usize] {
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
        if cables[ControlCable::RegBase as usize + 2 * self.reg_num] {
            bus.read_into(&self.value);
            {
                let lock = self.alu_tx.lock();
                lock.send((self.reg_num, self.value.clone()));
            }
        }
        if cables[ControlCable::RegBase as usize + 2 * self.reg_num + 1] {
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
        if cables[ControlCable::AluOut as usize] {
            let ret = self.reg_a.clone();
            if cables[ControlCable::AddMul as usize] {
                // multiplication or division
                if cables[ControlCable::SubDiv as usize] {
                    // division
                    ret.div(&self.reg_b);
                } else {
                    // multiplication
                    ret.mul(&self.reg_b);
                }
            } else {
                // addition or subtraction
                if cables[ControlCable::SubDiv as usize] {
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
