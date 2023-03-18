use std::vec;

use crate::cpu_component::{
    reg_dec, reg_in, reg_inc, reg_out, ControlCables, ControlCablesExt, INSTRUCTION_REG_NUM,
    PROGRAM_COUNTER_REG_NUM, STACK_POINTER_REG_NUM,
};

use crate::cpu_component::ControlCable::*;

pub type Microcodes = Vec<Vec<usize>>;

#[derive(PartialEq, Eq, Hash, FromPrimitive, Debug)]
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

use INSTRUCTION::*;

const OPCODE_SHIFT: u8 = 10;
const SOURCE_SHIFT: u8 = 5;
const OPCODE_MASK: u32 = 0b1111110000000000;
const SOURCE_MASK: u32 = 0b0000001111100000;
const DEST_MASK: u32 = 0b0000000000011111;

pub fn create_fetch_microcodes(increment_pc: bool) -> Microcodes {
    let mut load_ir = vec![RamOut as usize, reg_in(INSTRUCTION_REG_NUM)];
    if increment_pc {
        load_ir.push(reg_inc(PROGRAM_COUNTER_REG_NUM));
    }
    vec![
        vec![reg_out(PROGRAM_COUNTER_REG_NUM), MemoryAddressIn as usize],
        load_ir,
    ]
}

pub fn create_microcodes(instruction: u32, cables: &ControlCables) -> Microcodes {
    let opcode = (instruction & OPCODE_MASK) >> OPCODE_SHIFT;
    let src: usize = ((instruction & SOURCE_MASK) >> SOURCE_SHIFT) as usize;
    let dst: usize = (instruction & DEST_MASK) as usize;

    let mut ret: Microcodes = vec![];

    let mut jump = false;

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
            jump = true;
        }
        JE => {
            if cables.load(Equal) {
                ret.push(vec![reg_out(dst), reg_in(PROGRAM_COUNTER_REG_NUM)]);
                jump = true;
            }
        }
        JNE => {
            if !cables.load(Equal) {
                ret.push(vec![reg_out(dst), reg_in(PROGRAM_COUNTER_REG_NUM)]);
                jump = true;
            }
        }
        JG => {
            if cables.load(Greater) {
                ret.push(vec![reg_out(dst), reg_in(PROGRAM_COUNTER_REG_NUM)]);
                jump = true;
            }
        }
        JGE => {
            if cables.load(Greater) || cables.load(Equal) {
                ret.push(vec![reg_out(dst), reg_in(PROGRAM_COUNTER_REG_NUM)]);
                jump = true;
            }
        }
        JL => {
            if !cables.load(Greater) && !cables.load(Equal) {
                ret.push(vec![reg_out(dst), reg_in(PROGRAM_COUNTER_REG_NUM)]);
                jump = true;
            }
        }
        JLE => {
            if !cables.load(Greater) {
                ret.push(vec![reg_out(dst), reg_in(PROGRAM_COUNTER_REG_NUM)]);
                jump = true;
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
            ret.push(vec![
                reg_out(STACK_POINTER_REG_NUM),
                MemoryAddressIn as usize,
            ]);
            ret.push(vec![
                reg_in(dst),
                RamOut as usize,
                reg_inc(STACK_POINTER_REG_NUM),
            ]);
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
        INC => ret.push(vec![reg_inc(dst - 1)]),
        DEC => ret.push(vec![reg_dec(dst - 1)]),
        LOAD => {
            ret.push(vec![reg_out(src), MemoryAddressIn as usize]);
            ret.push(vec![reg_in(dst), RamOut as usize]);
        }
        STORE => {
            ret.push(vec![reg_out(dst), MemoryAddressIn as usize]);
            ret.push(vec![reg_out(src), RamIn as usize]);
        }
        LDCNST => {
            ret.push(vec![reg_out(PROGRAM_COUNTER_REG_NUM), MemoryAddressIn as usize]);
            ret.push(vec![reg_in(dst), RamOut as usize, reg_inc(PROGRAM_COUNTER_REG_NUM)]);
        }
    }
    ret.append(&mut create_fetch_microcodes(!jump));
    ret
}
