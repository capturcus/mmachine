use crate::bits::MValue;
use std::collections::BTreeMap;
use std::sync::mpsc::{Receiver, Sender};

use parking_lot::Mutex;
use std::sync::Arc;

pub struct IO {
    pub values: Arc<Mutex<BTreeMap<u32, u32>>>,
    pub output_rx: Option<Receiver<(MValue, MValue)>>,
    pub input_tx: Option<Sender<MValue>>,
    pub input_req_rx: Option<Receiver<MValue>>,
}

impl IO {
    pub fn mmachine_thread(&mut self) {
        let output_rx = self.output_rx.take().unwrap();
        loop {
            match output_rx.recv() {
                Ok((port, value)) => {
                    if port.as_u32() == 1 {
                        print!("{}", (value.as_u32() as u8) as char);
                    } else {
                        println!("OUTPUT: port {} value {}", port.as_u32(), value.as_u32());
                    }
                }
                Err(err) => {
                    println!("recv error {:#?}", err);
                    return;
                }
            }
        }
    }

    pub fn server_thread(values: Arc<Mutex<BTreeMap<u32, u32>>>) {}

    pub fn new() -> Self {
        IO {
            values: Arc::new(Mutex::new(BTreeMap::new())),
            output_rx: None,
            input_tx: None,
            input_req_rx: None,
        }
    }
}
