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
}
