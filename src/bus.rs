use crate::bits::MValue;
use parking_lot::{Mutex, Condvar};

pub struct Bus {
    value: MValue,
    mutex: parking_lot::Mutex<bool>,
    cvar: parking_lot::Condvar,
}

impl Bus {
    pub fn write_from(&self, val: &MValue) {
        let mut wrote = self.mutex.lock();
        *wrote = true;
        self.value.set(&val);
        self.cvar.notify_one();
    }

    pub fn read_into(&self, val: &MValue) {
        let mut wrote = self.mutex.lock();
        if !*wrote {
            self.cvar.wait(&mut wrote);
        }
        *wrote = false;
        val.set(&self.value);
    }

    pub fn new() -> Self {
        Bus {
            value: MValue::from_u32(0),
            mutex: Mutex::new(false),
            cvar: Condvar::new()
        }
    }
}
