use std::sync::mpsc::{Receiver, Sender, channel};

use crate::bits::MValue;
use parking_lot::{Mutex, Condvar};

pub struct Bus {
    value: MValue,
    rx: Mutex<Receiver<()>>,
    tx: Mutex<Sender<()>>,
}

impl Bus {
    pub fn write_from(&self, val: &MValue) {
        self.value.set(&val);
        let tx = self.tx.lock();
        tx.send(()).unwrap();
    }

    pub fn read_into(&self, val: &MValue) {
        let rx = self.rx.lock();
        rx.recv().unwrap();
        val.set(&self.value);
    }

    pub fn new() -> Self {
        let (tx, rx) = channel();
        Bus {
            value: MValue::from_u32(0),
            rx: Mutex::new(rx),
            tx: Mutex::new(tx),
        }
    }
}
