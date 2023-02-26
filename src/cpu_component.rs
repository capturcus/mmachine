use parking_lot::{Mutex, RwLock, RwLockWriteGuard};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::mpsc::Sender;

use crate::bits::MValue;
use crate::bus::Bus;
use std::sync::atomic::AtomicUsize;
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

pub type ControlCables = [bool; CONTROL_CABLES_SIZE];

pub struct AluSynchronizer {
    pub mutex: parking_lot::Mutex<u32>,
    pub cvar: parking_lot::Condvar,
}

pub struct CpuComponentArgs<'a> {
    pub cables: &'a RwLock<ControlCables>,
    pub bus: Arc<Bus>,
    pub rx: mpsc::Receiver<()>,
    pub finished: &'a AtomicUsize,
    pub clock_tx: Sender<()>,
}

pub trait CpuComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables) {}
    fn control_step(&self, bus: Arc<Bus>, cables: RwLockWriteGuard<ControlCables>) {}
    fn is_control(&self) -> bool {
        return false;
    }
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
        if component.is_control() {
            component.control_step(args.bus.clone(), args.cables.write());
        } else {
            component.step(args.bus.clone(), &args.cables.read());
        }
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
    pub alu_sync: Arc<AluSynchronizer>,
}

impl CpuComponent for RegisterComponent {
    fn step(&self, bus: Arc<Bus>, cables: &ControlCables) {
        if cables[ControlCable::RegBase as usize + 2 * self.reg_num] {
            bus.read_into(&self.value);
            {
                let lock = self.alu_tx.lock();
                lock.send((self.reg_num, self.value.clone())).unwrap();
            }
            {
                println!("reg {}: waiting for alu_sync mutex", self.reg_num);
                let mut sent_mvalues = self.alu_sync.mutex.lock();
                *sent_mvalues += 1;
                println!("reg {}: sent_mvalues: {}", self.reg_num, *sent_mvalues);
            }
            println!("reg {}: released alu_sync mutex", self.reg_num);
            self.alu_sync.cvar.notify_one();
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
    pub alu_sync: Arc<AluSynchronizer>,
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
            {
                println!("alu: waiting for alu_sync mutex");
                let mut sent_mvalues = self.alu_sync.mutex.lock();
                println!("alu: sent_mvalues: {}", *sent_mvalues);
                *sent_mvalues -= 1;
                self.alu_sync.cvar.notify_one();
            }
            println!("alu: released alu_sync mutex");
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

pub struct ClockComponent<'a> {
    pub clock_rx: Receiver<()>,
    pub txs: Vec<Sender<()>>,
    pub finished: &'a AtomicUsize,
    pub clock_ctrl_rx: Receiver<()>,
    pub alu_sync: Arc<AluSynchronizer>,
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
            {
                println!("clock: waiting for alu_sync mutex");
                let mut sent_mvalues = self.alu_sync.mutex.lock();
                println!("clock: acquired alu_sync mutex");
                while *sent_mvalues != 0 {
                    println!("clock: sent_mvalues = {}, waiting", *sent_mvalues);
                    self.alu_sync.cvar.wait(&mut sent_mvalues);
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
    fn is_control(&self) -> bool {
        return true;
    }

    fn control_step(&self, bus: Arc<Bus>, mut cables: RwLockWriteGuard<ControlCables>) {
        *cables = [false; CONTROL_CABLES_SIZE];
        cables[reg_out(3)] = true;
        cables[reg_in(0)] = true;
    }
}
