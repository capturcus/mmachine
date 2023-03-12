use std::collections::HashMap;
use std::vec;

use crate::cpu_component::{
    reg_in, reg_inc, reg_out, INSTRUCTION_REG_NUM, PROGRAM_COUNTER_REG_NUM,
};

use crate::cpu_component::ControlCable::*;

pub type Microcodes = Vec<Vec<usize>>;

#[derive(PartialEq, Eq, Hash)]
pub enum INSTRUCTION {
    _FETCH = 0,
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
    RET = 0b010001,
    INT = 0b010010,
    EOI = 0b010011,
    INC = 0b010100,
    DEC = 0b010101,
    HLT = 0b010110,
}

use INSTRUCTION::*;

const OPCODE_SHIFT: u8 = 10;
const SOURCE_SHIFT: u8 = 5;
const OPCODE_MASK: u32 = 0b1111110000000000;
const SOURCE_INDIRECT_MASK: u32 = 0b0000001000000000;
const SOURCE_MASK: u32 = 0b0000000111100000;
const DEST_INDIRECT_MASK: u32 = 0b0000000000010000;
const DEST_MASK: u32 = 0b0000000000001111;

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

pub fn create_microcodes(instruction: u32) -> Microcodes {
    vec![vec![reg_inc(PROGRAM_COUNTER_REG_NUM)], vec![Halt as usize]]
}
