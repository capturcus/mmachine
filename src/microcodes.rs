use std::sync::atomic::Ordering::SeqCst;
use std::vec;

use crate::bits::MValue;
use crate::cpu_component::{
    reg_dec, reg_in, reg_inc, reg_out, EQUAL_BIT_NUM, GREATER_BIT_NUM, INSTRUCTION_REG_NUM,
    PROGRAM_COUNTER_REG_NUM, STACK_POINTER_REG_NUM,
};

use crate::cpu_component::ControlCable::*;

pub type Microcodes = Vec<Vec<usize>>;

#[derive(PartialEq, Eq, Hash, FromPrimitive, Debug, Clone, Copy)]
pub enum INSTRUCTION {
    HLT = 0b000000,
    MOV = 0b000001,
    ADD = 0b000010,
    SUB = 0b000011,
    MUL = 0b000100,
    DIV = 0b000101,
    CALL = 0b000110,
    JE = 0b000111,
    JNE = 0b001000,
    JG = 0b001001,
    JGE = 0b001010,
    JL = 0b001011,
    JLE = 0b001100,
    PUSH = 0b001101,
    POP = 0b001110,
    OUT = 0b001111,
    IN = 0b010000,
    INT = 0b010010,
    EOI = 0b010011,
    INC = 0b010100,
    DEC = 0b010101,
    LOAD = 0b010110,
    STORE = 0b010111,
    LDCNST = 0b011000,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, FromPrimitive)]
pub enum REG {
    A = 0b00000,
    B = 0b00001,
    C = 0b00010,
    D = 0b00011,
    E = 0b00100,
    PC = 0b00101,
    SP = 0b00110,
    INST = 0b00111,
}

use INSTRUCTION::*;

pub const OPCODE_SHIFT: u8 = 10;
pub const SOURCE_SHIFT: u8 = 5;
pub const OPCODE_MASK: u32 = 0b1111110000000000;
pub const SOURCE_MASK: u32 = 0b0000001111100000;
pub const DEST_MASK: u32 = 0b0000000000011111;

pub fn create_fetch_microcodes() -> Microcodes {
    vec![
        vec![reg_out(PROGRAM_COUNTER_REG_NUM), MemoryAddressIn as usize],
        vec![
            RamOut as usize,
            reg_in(INSTRUCTION_REG_NUM),
            reg_inc(PROGRAM_COUNTER_REG_NUM),
        ],
    ]
}

pub fn create_microcodes(instruction: u32, flags_reg: &MValue) -> Microcodes {
    let opcode = (instruction & OPCODE_MASK) >> OPCODE_SHIFT;
    let src: usize = ((instruction & SOURCE_MASK) >> SOURCE_SHIFT) as usize;
    let dst: usize = (instruction & DEST_MASK) as usize;

    let mut ret: Microcodes = vec![];

    match num::FromPrimitive::from_u32(opcode).unwrap() {
        HLT => ret.push(vec![Halt as usize]),
        MOV => ret.push(vec![reg_out(src), reg_in(dst)]),
        ADD => ret.push(vec![AluOut as usize, reg_in(dst)]),
        SUB => ret.push(vec![SubDiv as usize, AluOut as usize, reg_in(dst)]),
        MUL => ret.push(vec![AddMul as usize, AluOut as usize, reg_in(dst)]),
        DIV => ret.push(vec![
            AddMul as usize,
            SubDiv as usize,
            AluOut as usize,
            reg_in(dst),
        ]),
        CALL => {
            ret.push(vec![
                reg_out(STACK_POINTER_REG_NUM),
                MemoryAddressIn as usize,
            ]);
            ret.push(vec![
                reg_out(PROGRAM_COUNTER_REG_NUM),
                RamIn as usize,
                reg_dec(STACK_POINTER_REG_NUM),
            ]);
            ret.push(vec![reg_out(dst), reg_in(PROGRAM_COUNTER_REG_NUM)]);
        }
        JE => {
            if flags_reg.bit(EQUAL_BIT_NUM).load(SeqCst) {
                ret.push(vec![reg_out(dst), reg_in(PROGRAM_COUNTER_REG_NUM)]);
            }
        }
        JNE => {
            if !flags_reg.bit(EQUAL_BIT_NUM).load(SeqCst) {
                ret.push(vec![reg_out(dst), reg_in(PROGRAM_COUNTER_REG_NUM)]);
            }
        }
        JG => {
            if flags_reg.bit(GREATER_BIT_NUM).load(SeqCst) {
                ret.push(vec![reg_out(dst), reg_in(PROGRAM_COUNTER_REG_NUM)]);
            }
        }
        JGE => {
            if flags_reg.bit(GREATER_BIT_NUM).load(SeqCst)
                || flags_reg.bit(EQUAL_BIT_NUM).load(SeqCst)
            {
                ret.push(vec![reg_out(dst), reg_in(PROGRAM_COUNTER_REG_NUM)]);
            }
        }
        JL => {
            if !flags_reg.bit(GREATER_BIT_NUM).load(SeqCst)
                && !flags_reg.bit(EQUAL_BIT_NUM).load(SeqCst)
            {
                ret.push(vec![reg_out(dst), reg_in(PROGRAM_COUNTER_REG_NUM)]);
            }
        }
        JLE => {
            if !flags_reg.bit(GREATER_BIT_NUM).load(SeqCst) {
                ret.push(vec![reg_out(dst), reg_in(PROGRAM_COUNTER_REG_NUM)]);
            }
        }
        PUSH => {
            ret.push(vec![
                reg_out(STACK_POINTER_REG_NUM),
                MemoryAddressIn as usize,
            ]);
            ret.push(vec![
                reg_out(src),
                RamIn as usize,
                reg_dec(STACK_POINTER_REG_NUM),
            ]);
        }
        POP => {
            ret.push(vec![reg_inc(STACK_POINTER_REG_NUM)]);
            ret.push(vec![
                reg_out(STACK_POINTER_REG_NUM),
                MemoryAddressIn as usize,
            ]);
            ret.push(vec![reg_in(dst), RamOut as usize]);
        }
        OUT => {
            ret.push(vec![
                MemoryIsIO as usize,
                reg_out(src),
                MemoryAddressIn as usize,
            ]);
            ret.push(vec![MemoryIsIO as usize, reg_out(dst), RamIn as usize]);
        }
        IN => {
            ret.push(vec![
                MemoryIsIO as usize,
                reg_out(src),
                MemoryAddressIn as usize,
            ]);
            ret.push(vec![MemoryIsIO as usize, reg_in(dst), RamOut as usize]);
        }
        INT => todo!(),
        EOI => todo!(),
        INC => ret.push(vec![reg_inc(dst)]),
        DEC => ret.push(vec![reg_dec(dst)]),
        LOAD => {
            ret.push(vec![reg_out(src), MemoryAddressIn as usize]);
            ret.push(vec![reg_in(dst), RamOut as usize]);
        }
        STORE => {
            ret.push(vec![reg_out(dst), MemoryAddressIn as usize]);
            ret.push(vec![reg_out(src), RamIn as usize]);
        }
        LDCNST => {
            ret.push(vec![
                reg_out(PROGRAM_COUNTER_REG_NUM),
                MemoryAddressIn as usize,
            ]);
            if dst == PROGRAM_COUNTER_REG_NUM {
                ret.push(vec![reg_in(dst), RamOut as usize]);
            } else {
                ret.push(vec![
                    reg_in(dst),
                    RamOut as usize,
                    reg_inc(PROGRAM_COUNTER_REG_NUM),
                ]);
            }
        }
    }
    ret.append(&mut create_fetch_microcodes());
    ret
}
