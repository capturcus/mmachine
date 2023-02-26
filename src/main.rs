#![feature(variant_count)]

use parking_lot::{RwLock, RwLockWriteGuard, Mutex};
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

fn clock_tick(clock_rx: &Receiver<()>, txs: &Vec<Sender<()>>, finished: &AtomicUsize) {
    for t in txs {
        t.send(()).unwrap();
    }
    loop {
        clock_rx.recv().unwrap();
        let amount_finished = finished.load(SeqCst);
        if amount_finished == txs.len() {
            finished.store(0, SeqCst);
            break;
        }
    }
}

fn clock_thread(clock_rx: Receiver<()>, txs: Vec<Sender<()>>, finished: &AtomicUsize) {
    loop {
        clock_tick(&clock_rx, &txs, finished);
    }
}

fn reg_in(reg_num: usize) -> usize {
    ControlCable::RegBase as usize + 2 * reg_num
}

fn reg_out(reg_num: usize) -> usize {
    ControlCable::RegBase as usize + 2 * reg_num + 1
}

fn main() {
    let cables = RwLock::new([false; CONTROL_CABLES_SIZE]);
    let bus = Arc::new(Bus::new());
    let finished = AtomicUsize::new(0);
    let (clock_tx, clock_rx) = channel();
    let (alu_tx, alu_rx) = channel();
    let alu_tx_arc = Arc::new(Mutex::new(alu_tx));

    let mut txs = Vec::new();
    let alu = Arc::new(AluComponent {
        reg_a: MValue::from_u32(0),
        reg_b: MValue::from_u32(0),
    });
    let components: Vec<Arc<dyn CpuComponent + Send + Sync>> = vec![
        alu.clone(),
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
    ];

    std::thread::scope(|s| {
        s.spawn(||{
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
        // s.spawn(|| {
        //     clock_thread(clock_rx, txs, &finished);
        // });
        {
            let mut cables_writer = cables.write();
            *cables_writer = [false; CONTROL_CABLES_SIZE];
            cables_writer[reg_out(3)] = true;
            cables_writer[reg_in(0)] = true;
        }
        clock_tick(&clock_rx, &txs, &finished);
        {
            let mut cables_writer = cables.write();
            *cables_writer = [false; CONTROL_CABLES_SIZE];
            cables_writer[reg_out(3)] = true;
            cables_writer[reg_in(1)] = true;
        }
        clock_tick(&clock_rx, &txs, &finished);
        {
            let mut cables_writer = cables.write();
            *cables_writer = [false; CONTROL_CABLES_SIZE];
            cables_writer[ControlCable::AluOut as usize] = true;
            cables_writer[reg_in(2)] = true;
        }
        clock_tick(&clock_rx, &txs, &finished);
    });
}
