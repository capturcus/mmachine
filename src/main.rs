#![feature(variant_count)]

use parking_lot::{Condvar, Mutex, RwLock, RwLockWriteGuard};
use std::io::{self, BufRead};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;

mod bits;
mod bus;
mod cpu_component;

use crate::bits::MValue;
use crate::bus::Bus;

#[cfg(test)]
mod tests;

use crate::cpu_component::*;

fn main() {
    let cables = array_init::array_init(|_| AtomicBool::new(false));
    let bus = Arc::new(Bus::new());
    let finished = AtomicUsize::new(0);
    let sent_to_alu = Arc::new(AtomicUsize::new(0));
    let (clock_tx, clock_rx) = channel();
    let (alu_tx, alu_rx) = channel();
    let (alu_clock_tx, alu_clock_rx) = channel();
    let alu_tx_arc = Arc::new(Mutex::new(alu_tx));

    let mut txs = Vec::new();
    let alu = Arc::new(AluComponent {
        reg_a: MValue::from_u32(0),
        reg_b: MValue::from_u32(0),
    });

    let components: Vec<Arc<dyn CpuComponent + Send + Sync>> = vec![
        Arc::new(ControlComponent::new()),
        Arc::new(ProgramCounterComponent {
            program_counter: MValue::from_u32(0),
        }),
        Arc::new(RegisterComponent {
            value: MValue::from_u32(10),
            reg_num: 0,
            alu_tx: alu_tx_arc.clone(),
            sent_to_alu: sent_to_alu.clone(),
        }),
        Arc::new(RegisterComponent {
            value: MValue::from_u32(20),
            reg_num: 1,
            alu_tx: alu_tx_arc.clone(),
            sent_to_alu: sent_to_alu.clone(),
        }),
        Arc::new(RegisterComponent {
            value: MValue::from_u32(12),
            reg_num: 2,
            alu_tx: alu_tx_arc.clone(),
            sent_to_alu: sent_to_alu.clone(),
        }),
        Arc::new(RegisterComponent {
            value: MValue::from_u32(0),
            reg_num: 3,
            alu_tx: alu_tx_arc.clone(),
            sent_to_alu: sent_to_alu.clone(),
        }),
        alu.clone(),
        Arc::new(RamComponent {
            memory: vec![MValue::from_u32(0); RAM_SIZE].try_into().unwrap(),
            memory_address_register: MValue::default(),
            ram_register: MValue::default(),
        }),
    ];

    std::thread::scope(|s| {
        s.spawn(|| {
            alu.run(alu_rx, alu_clock_tx);
        });
        for c in components {
            let (tx, rx) = channel();
            start_cpu_component(
                CpuComponentArgs {
                    cables: &cables,
                    bus: bus.clone(),
                    rx: rx,
                    finished: &finished,
                    clock_tx: clock_tx.clone(),
                },
                c,
                s,
            );
            txs.push(tx);
        }
        let clock = ClockComponent {
            clock_rx: clock_rx,
            txs: txs,
            finished: &finished,
            alu_clock_rx: alu_clock_rx,
            sent_to_alu: sent_to_alu.clone(),
            cables: &cables,
        };
        s.spawn(move || {
            clock.run();
        });

        let mut line = String::new();
        let stdin = io::stdin();

        loop {
            stdin.lock().read_line(&mut line).unwrap();
        }
    });
}
