use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;

pub const BITNESS: usize = 16;

#[derive(Default, Debug)]
pub struct MValue {
    val: [AtomicBool; BITNESS],
}

impl MValue {
    pub fn bit(&self, i: usize) -> &AtomicBool {
        &self.val[i]
    }

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
        let result = MValue::from_u32(0);
        let carry = AtomicBool::new(false);

        for i in 0..BITNESS {
            let a_bit = self.val[i].load(SeqCst);
            let b_bit = other.val[i].load(SeqCst);

            // Calculate the sum of a_bit, b_bit, and the previous carry.
            let sum = (a_bit ^ b_bit) ^ carry.load(SeqCst);

            // Set the result bit to the sum.
            result.val[i].store(sum, SeqCst);

            // Calculate the new carry.
            carry.store((a_bit & b_bit) | (a_bit & carry.load(SeqCst)) | (b_bit & carry.load(SeqCst)), SeqCst);
        }
        self.set(&result);
    }

    pub fn sub(&self, other: &MValue) {
        let result = MValue::from_u32(0);
        let borrow = AtomicBool::new(false);

        for i in 0..BITNESS {
            let a_bit = self.val[i].load(SeqCst);
            let b_bit = other.val[i].load(SeqCst);

            // Calculate the difference of a_bit, b_bit, and the previous borrow.
            let diff = (a_bit ^ b_bit) ^ borrow.load(SeqCst);

            // Set the result bit to the difference.
            result.val[i].store(diff, SeqCst);

            // Calculate the new borrow.
            borrow.store((!a_bit & b_bit) | ((!a_bit | b_bit) & borrow.load(SeqCst)), SeqCst);
        }
        self.set(&result);
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

    fn clone_from(&mut self, _source: &Self) {
        todo!()
    }
}
