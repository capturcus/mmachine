use parking_lot::RwLock;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::{Sender, channel};
use std::sync::atomic::Ordering::SeqCst;

mod control_cables;
mod cpu_component;

use crate::control_cables::ControlCables;
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

fn main() {
    let cables = RwLock::new(ControlCables::new());
    let bus = AtomicU32::new(0);
    let finished = AtomicUsize::new(0);
    let (clock_tx, clock_rx) = channel();

    let components: Vec<Box<dyn CpuComponent + Send + Sync>> = vec![
        Box::new(ProgramCounterComponent { program_counter: 0 }),
        Box::new(RegisterComponent {
            value: 10,
            reg_num: 0,
        }),
        Box::new(RegisterComponent {
            value: 0,
            reg_num: 1,
        }),
        Box::new(RegisterComponent {
            value: 0,
            reg_num: 2,
        }),
        Box::new(RegisterComponent {
            value: 0,
            reg_num: 3,
        }),
    ];

    let mut txs = Vec::new();

    std::thread::scope(|s| {
        for c in components {
            let (tx, rx) = channel();
            start_cpu_component(
                CpuComponentArgs {
                    cables: &cables,
                    bus: &bus,
                    rx: rx,
                    finished: &finished,
                    clock_tx: clock_tx.clone()
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
            cables_writer.reg_out[0] = true;
        }
        clock_tick(&clock_rx, &txs, &finished);
        {
            let mut cables_writer = cables.write();
            cables_writer.reset();
            cables_writer.reg_in[1] = true;
        }
        clock_tick(&clock_rx, &txs, &finished);
    });
}
