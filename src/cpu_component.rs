use parking_lot::RwLock;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::mpsc::Sender;

use crate::control_cables::ControlCables;
use std::sync::atomic::{AtomicU32, AtomicUsize};
use std::sync::mpsc;

pub struct CpuComponentArgs<'a> {
    pub cables: &'a RwLock<ControlCables>,
    pub bus: &'a AtomicU32,
    pub rx: mpsc::Receiver<()>,
    pub finished: &'a AtomicUsize,
    pub clock_tx: Sender<()>,
}

pub trait CpuComponent {
    fn step(&mut self, bus: &AtomicU32, cables: &ControlCables);
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
    pub program_counter: u32,
}

impl CpuComponent for ProgramCounterComponent {
    fn step(&mut self, bus: &AtomicU32, cables: &ControlCables) {
        if cables.counter_enable {
            self.program_counter += 1;
        }
        if cables.counter_in {
            self.program_counter = bus.load(SeqCst);
        }
        if cables.counter_out {
            bus.store(self.program_counter, SeqCst);
        }
    }
}

pub struct RegisterComponent {
    pub reg_num: usize,
    pub value: u32,
}

impl CpuComponent for RegisterComponent {
    fn step(&mut self, bus: &AtomicU32, cables: &ControlCables) {
        if cables.reg_in[self.reg_num] {
            self.value = bus.load(SeqCst);
        }
        if cables.reg_out[self.reg_num] {
            bus.store(self.value, SeqCst);
        }
        println!("register {} is now {}", self.reg_num, self.value);
    }
}
