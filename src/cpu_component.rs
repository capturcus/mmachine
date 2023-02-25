use parking_lot::RwLock;
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
    fn step(&mut self, bus: Arc<Bus>, cables: &ControlCables);
}

pub fn start_cpu_component<'a, T: CpuComponent + std::marker::Send + ?Sized + 'static>(
    args: CpuComponentArgs<'a>,
    mut component: Box<T>,
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
    fn step(&mut self, bus: Arc<Bus>, cables: &ControlCables) {
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
}

impl CpuComponent for RegisterComponent {
    fn step(&mut self, bus: Arc<Bus>, cables: &ControlCables) {
        if cables[ControlCable::RegBase as usize + 2 * self.reg_num] {
            bus.read_into(&self.value);
        }
        if cables[ControlCable::RegBase as usize + 2 * self.reg_num + 1] {
            bus.write_from(&self.value);
        }
        println!("register {} is now {}", self.reg_num, self.value.as_u32());
    }
}
