use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;

const BITNESS: usize = 16;

pub struct MValue {
    val: [AtomicBool; BITNESS],
}

impl MValue {
    pub fn set(&self, other: &MValue) {
        for i in 0..BITNESS {
            self.val[i].store(other.val[i].load(SeqCst), SeqCst);
        }
    }

    pub fn as_u32(&self) -> u32 {
        let mut ret = 0;
        let mut two = 1;
        for i in 0..BITNESS {
            ret += (self.val[i].load(SeqCst) as u32) * two;
            two *= 2;
        }
        ret
    }

    pub fn as_string(&self) -> String {
        let mut ret = String::new();
        for i in 0..BITNESS {
            ret += if self.val[i].load(SeqCst) { "1" } else { "0" };
        }
        ret.chars().rev().collect::<String>()
    }

    pub fn from_u32(num: u32) -> Self {
        let ret = MValue {
            val: array_init::array_init(|_| AtomicBool::new(false)),
        };
        let mut two = 1;
        for i in 0..BITNESS {
            ret.val[i].store((two & num) != 0, SeqCst);
            two *= 2;
        }
        ret
    }

    pub fn add(&self, other: &MValue) {
        let my_val = self.as_u32();
        let other_val = other.as_u32();
        let ret = (my_val + other_val) % (1 << BITNESS);
        self.set(&MValue::from_u32(ret));
    }

    pub fn sub(&self, other: &MValue) {
        let mut my_val = self.as_u32();
        let other_val = other.as_u32();
        if other_val > my_val {
            my_val += 1 << BITNESS;
        }
        let ret = my_val - other_val;
        self.set(&MValue::from_u32(ret));
    }

    pub fn mul(&self, other: &MValue) {
        let my_val = self.as_u32();
        let other_val = other.as_u32();
        let ret = (my_val * other_val) % (1 << BITNESS);
        self.set(&MValue::from_u32(ret));
    }

    pub fn div(&self, other: &MValue) {
        let my_val = self.as_u32();
        let other_val = other.as_u32();
        let ret = my_val / other_val;
        self.set(&MValue::from_u32(ret));
    }
}

impl Clone for MValue {
    fn clone(&self) -> Self {
        let ret = MValue::from_u32(0);
        ret.set(&self);
        ret
    }

    fn clone_from(&mut self, source: &Self) {
        todo!()
    }
}
