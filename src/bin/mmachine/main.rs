use std::sync::atomic::AtomicBool;

use mmachine::bits::MValue;
use mmachine::bus::Bus;
use mmachine::cpu_component::{AluComponent, CpuComponent, RamComponent, RAM_SIZE, REGISTERS_NUM, RegisterComponent, start_cpu_component, CpuComponentArgs, ControlComponent};
use mmachine::microcodes::{create_fetch_microcodes};
use parking_lot::Mutex;
use std::io::{self, BufRead};
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

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

fn load_ram(args: Vec<String>) -> Box<[MValue; RAM_SIZE]> {
    let contents = std::fs::read(args[1].clone()).unwrap();
    let mut ret = Vec::new();
    for i in 0..contents.len()/2 {
        let x: u32 = (contents[2*i] as u32) << 8 | contents[2*i+1] as u32;
        ret.push(MValue::from_u32(x));
    }
    for i in 0..ret.len() {
        println!("{}", ret[i].as_string());
    }
    ret.resize(RAM_SIZE, MValue::from_u32(0));
    ret.try_into().unwrap()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

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
            memory: load_ram(args),
            memory_address_register: MValue::default(),
            ram_register: MValue::default(),
            output_tx: Arc::new(Mutex::new(output_tx)),
            input_rx: Arc::new(Mutex::new(input_rx)),
            input_req_tx: Arc::new(Mutex::new(input_req_tx)),
        }),
    ];
    for i in 0..REGISTERS_NUM {
        components.push(Arc::new(RegisterComponent {
            value: MValue::from_u32(0),
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
            microcode_counter: AtomicUsize::new(0),
            instruction_register: MValue::from_u32(0),
            current_microcodes: Arc::new(Mutex::new(create_fetch_microcodes(true))),
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