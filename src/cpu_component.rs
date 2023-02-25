use parking_lot::{RwLock, Mutex};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::mpsc::Sender;

use crate::control_cables::ControlCables;
use std::sync::atomic::{AtomicU32, AtomicUsize};
use std::sync::mpsc;
use std::sync::Arc;

pub struct Bus {
    pub value: AtomicU32,
    pub mutex: parking_lot::Mutex<bool>,
    pub cvar: parking_lot::Condvar,
}

impl Bus {
    fn write(&self, val: u32) {
        let mut wrote = self.mutex.lock();
        *wrote = true;
        self.value.store(val, SeqCst);
        self.cvar.notify_one();
    }

    fn read(&self) -> u32 {
        let mut wrote = self.mutex.lock();
        if !*wrote {
            self.cvar.wait(&mut wrote);
        }
        *wrote = false;
        self.value.load(SeqCst)
    }
}

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
    pub program_counter: u32,
}

impl CpuComponent for ProgramCounterComponent {
    fn step(&mut self, bus: Arc<Bus>, cables: &ControlCables) {
        if cables.counter_enable {
            self.program_counter += 1;
        }
        if cables.counter_in {
            self.program_counter = bus.read();
        }
        if cables.counter_out {
            bus.write(self.program_counter);
        }
    }
}

pub struct RegisterComponent {
    pub reg_num: usize,
    pub value: u32,
}

impl CpuComponent for RegisterComponent {
    fn step(&mut self, bus: Arc<Bus>, cables: &ControlCables) {
        if cables.reg_in[self.reg_num] {
            self.value = bus.read();
        }
        if cables.reg_out[self.reg_num] {
            bus.write(self.value);
        }
        println!("register {} is now {}", self.reg_num, self.value);
    }
}
