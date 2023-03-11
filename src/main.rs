#![feature(variant_count)]

use parking_lot::Mutex;
use std::collections::HashMap;
use std::io::{self, BufRead};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

mod bits;
mod bus;
mod cpu_component;

use crate::bits::MValue;
use crate::bus::Bus;
use crate::cpu_component::ControlCable::*;
use crate::cpu_component::{reg_in, reg_out};

#[cfg(test)]
mod tests;

use crate::cpu_component::*;

fn run_input(input_tx: Sender<Option<MValue>>, input_req_rx: Receiver<MValue>) {
    loop {
        match input_req_rx.recv() {
            Ok(_) => input_tx.send(None).unwrap(),
            Err(_) => return,
        }
    }
}

fn run_output(output_rx: Receiver<(MValue, MValue)>) {
    loop {
        match output_rx.recv() {
            Ok((port, value)) => {
                println!("OUTPUT: port {} value {}", port.as_u32(), value.as_u32());
            }
            Err(_) => return,
        }
    }
}

fn main() {
    let cables = array_init::array_init(|_| AtomicBool::new(false));
    let bus = Arc::new(Bus::new());
    let finished = AtomicUsize::new(0);
    let sent_to_alu = Arc::new(AtomicUsize::new(0));
    let (clock_tx, clock_rx) = channel();
    let (alu_tx, alu_rx) = channel();
    let (alu_clock_tx, alu_clock_rx) = channel();
    let alu_tx_arc = Arc::new(Mutex::new(alu_tx));
    let (ctrl_tx, ctrl_rx) = channel();
    let (output_tx, output_rx) = channel();
    let (input_tx, input_rx) = channel();
    let (input_req_tx, input_req_rx) = channel();

    let mut txs = Vec::new();
    let alu = Arc::new(AluComponent {
        reg_a: MValue::from_u32(0),
        reg_b: MValue::from_u32(0),
    });

    let mut components: Vec<Arc<dyn CpuComponent + Send + Sync>> = vec![
        alu.clone(),
        Arc::new(RamComponent {
            memory: vec![MValue::from_u32(0); RAM_SIZE].try_into().unwrap(),
            memory_address_register: MValue::default(),
            ram_register: MValue::default(),
            output_tx: Arc::new(Mutex::new(output_tx)),
            input_rx: Arc::new(Mutex::new(input_rx)),
            input_req_tx: Arc::new(Mutex::new(input_req_tx)),
        }),
    ];
    for i in 0..REGISTERS_NUM {
        components.push(Arc::new(RegisterComponent {
            value: MValue::from_u32(i as u32),
            reg_num: i,
            alu_tx: alu_tx_arc.clone(),
            sent_to_alu: sent_to_alu.clone(),
        }));
    }

    std::thread::scope(|s| {
        s.spawn(|| {
            alu.run(alu_rx, alu_clock_tx, ctrl_tx, &cables);
        });
        s.spawn(move || {
            run_input(input_tx, input_req_rx);
        });
        s.spawn(move || {
            run_output(output_rx);
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
        let clock = ControlComponent {
            clock_rx: clock_rx,
            txs: txs,
            finished: &finished,
            alu_clock_rx: alu_clock_rx,
            sent_to_alu: sent_to_alu.clone(),
            cables: &cables,
            bus: bus.clone(),
            test_instruction: vec![
                vec![MemoryAddressIn as usize, reg_out(0)],
                vec![RamIn as usize, reg_out(1)],
                vec![MemoryAddressIn as usize, reg_out(2)],
                vec![MemoryAddressIn as usize, reg_out(0)],
                vec![RamOut as usize, reg_in(3)],
                vec![MemoryAddressIn as usize, reg_out(5)],
                vec![MemoryIsIO as usize, RamIn as usize, reg_out(3)],
                vec![Halt as usize],
            ],
            microcode_counter: AtomicUsize::new(0),
            instruction_register: MValue::from_u32(0),
        };
        s.spawn(move || {
            clock.run(ctrl_rx);
        });

        let mut line = String::new();
        let stdin = io::stdin();

        loop {
            stdin.lock().read_line(&mut line).unwrap();
        }
    });
}
