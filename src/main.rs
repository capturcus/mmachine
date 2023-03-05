#![feature(variant_count)]

use parking_lot::{Mutex, RwLock, RwLockWriteGuard, Condvar};
use std::io::{self, BufRead};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

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
    let (clock_tx, clock_rx) = channel();
    let (alu_tx, alu_rx) = channel();
    let alu_tx_arc = Arc::new(Mutex::new(alu_tx));
    let (clock_ctrl_tx, clock_ctrl_rx) = channel();

    let mut txs = Vec::new();
    let alu = Arc::new(AluComponent {
        reg_a: MValue::from_u32(0),
        reg_b: MValue::from_u32(0),
    });
    let components: Vec<Arc<dyn CpuComponent + Send + Sync>> = vec![
        Arc::new(ControlComponent{

        }),
        Arc::new(ProgramCounterComponent {
            program_counter: MValue::from_u32(0),
        }),
        Arc::new(RegisterComponent {
            value: MValue::from_u32(0),
            reg_num: 0,
            alu_tx: alu_tx_arc.clone(),
        }),
        Arc::new(RegisterComponent {
            value: MValue::from_u32(0),
            reg_num: 1,
            alu_tx: alu_tx_arc.clone(),
        }),
        Arc::new(RegisterComponent {
            value: MValue::from_u32(0),
            reg_num: 2,
            alu_tx: alu_tx_arc.clone(),
        }),
        Arc::new(RegisterComponent {
            value: MValue::from_u32(10),
            reg_num: 3,
            alu_tx: alu_tx_arc.clone(),
        }),
        alu.clone(),
    ];

    std::thread::scope(|s| {
        s.spawn(|| {
            alu.run(alu_rx);
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
            clock_ctrl_rx: clock_ctrl_rx,
        };
        s.spawn(move || {
            clock.run();
        });
        // {
        //     let mut cables_writer = cables.write();
        //     *cables_writer = [false; CONTROL_CABLES_SIZE];
        //     cables_writer[reg_out(3)] = true;
        //     cables_writer[reg_in(0)] = true;
        // }
        // clock_tick(&clock_rx, &txs, &finished);
        // {
        //     let mut cables_writer = cables.write();
        //     *cables_writer = [false; CONTROL_CABLES_SIZE];
        //     cables_writer[reg_out(3)] = true;
        //     cables_writer[reg_in(1)] = true;
        // }
        // clock_tick(&clock_rx, &txs, &finished);
        // {
        //     let mut cables_writer = cables.write();
        //     *cables_writer = [false; CONTROL_CABLES_SIZE];
        //     cables_writer[ControlCable::AluOut as usize] = true;
        //     cables_writer[reg_in(2)] = true;
        // }
        // clock_tick(&clock_rx, &txs, &finished);
        let mut line = String::new();
        let stdin = io::stdin();

        loop {
            stdin.lock().read_line(&mut line).unwrap();
            clock_ctrl_tx.send(()).unwrap();
        }
    });
}
