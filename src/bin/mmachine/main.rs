use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use clap::Parser;
use mmachine::bits::MValue;
use mmachine::bus::Bus;
use mmachine::cpu_component::{
    start_cpu_component, AluComponent, ControlComponent, CpuComponent, CpuComponentArgs,
    RamComponent, RegisterComponent, RAM_SIZE, REGISTERS_NUM, STACK_POINTER_REG_NUM,
};
use mmachine::microcodes::create_fetch_microcodes;
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
                if port.as_u32() == 1 {
                    print!("{}", (value.as_u32() as u8) as char);
                } else {
                    println!("OUTPUT: port {} value {}", port.as_u32(), value.as_u32());
                }
            }
            Err(_) => return,
        }
    }
}

fn load_ram(path: PathBuf) -> Box<[MValue; RAM_SIZE]> {
    let contents = std::fs::read(path).unwrap();
    let mut ret = Vec::new();
    for i in 0..contents.len() / 2 {
        let x: u32 = (contents[2 * i] as u32) << 8 | contents[2 * i + 1] as u32;
        ret.push(MValue::from_u32(x));
    }
    ret.resize(RAM_SIZE, MValue::from_u32(0));
    ret.try_into().unwrap()
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// the binary file that will be loaded at 0 at startup
    bin_file: PathBuf,

    /// whether to wait for enter to step
    #[arg(short, long, default_value_t = false)]
    step: bool,
}

fn main() {
    let args = Args::parse();

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
    let (clock_step_tx, clock_step_rx) = channel();

    let mut txs = Vec::new();

    let mut components: Vec<Arc<dyn CpuComponent + Send + Sync>> = vec![
        Arc::new(RamComponent {
            memory: load_ram(args.bin_file),
            memory_address_register: MValue::default(),
            ram_register: MValue::default(),
            output_tx: Arc::new(Mutex::new(output_tx)),
            input_rx: Arc::new(Mutex::new(input_rx)),
            input_req_tx: Arc::new(Mutex::new(input_req_tx)),
        }),
    ];
    for i in 0..REGISTERS_NUM {
        let mut start_value: u32 = 0;
        if i == STACK_POINTER_REG_NUM {
            start_value = RAM_SIZE as u32 - 1;
        }
        components.push(Arc::new(RegisterComponent {
            value: MValue::from_u32(start_value),
            reg_num: i,
            alu_tx: alu_tx_arc.clone(),
            sent_to_alu: sent_to_alu.clone(),
        }));
    }
    let flags_register = Arc::new(MValue::from_u32(0));
    let alu = Arc::new(AluComponent {
        reg_a: MValue::from_u32(0),
        reg_b: MValue::from_u32(0),
        flags_reg: flags_register.clone(),
    });
    components.push(alu.clone());

    let print_components = components.clone();

    std::thread::scope(|s| {
        s.spawn(|| {
            alu.run(alu_rx, alu_clock_tx, ctrl_tx);
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
            current_microcodes: Arc::new(Mutex::new(create_fetch_microcodes())),
            clock_step_rx: clock_step_rx,
            clock_step: args.step,
            flags_register: flags_register.clone(),
        };
        s.spawn(move || {
            clock.run(ctrl_rx);
        });
        let mut line = String::new();
        let stdin = io::stdin();

        loop {
            if args.step {
                stdin.lock().read_line(&mut line).unwrap();
                for c in &print_components {
                    c.step_print();
                }
                clock_step_tx.send(()).unwrap();
            }
        }
    });
}
