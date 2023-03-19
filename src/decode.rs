
use std::{collections::HashMap};
use crate::decode::INSTRUCTION::*;

use crate::microcodes::{OPCODE_MASK, SOURCE_MASK, DEST_MASK, INSTRUCTION, REG, OPCODE_SHIFT, SOURCE_SHIFT};

fn mnemonic_names() -> HashMap<INSTRUCTION, &'static str> {
    let mut ret = HashMap::new();
    ret.insert(HLT, "hlt");
    ret.insert(MOV, "mov");
    ret.insert(ADD, "add");
    ret.insert(SUB, "sub");
    ret.insert(MUL, "mul");
    ret.insert(DIV, "div");
    ret.insert(CALL, "call");
    ret.insert(JE, "je");
    ret.insert(JNE, "jne");
    ret.insert(JG, "jg");
    ret.insert(JGE, "jge");
    ret.insert(JL, "jl");
    ret.insert(JLE, "jle");
    ret.insert(PUSH, "push");
    ret.insert(POP, "pop");
    ret.insert(OUT, "out");
    ret.insert(IN, "in");
    ret.insert(INT, "int");
    ret.insert(EOI, "eoi");
    ret.insert(INC, "inc");
    ret.insert(DEC, "dec");
    ret.insert(LOAD, "load");
    ret.insert(STORE, "store");
    ret.insert(LDCNST, "ldcnst");
    ret
}

fn reg_names() -> HashMap<REG, &'static str> {
    let mut ret = HashMap::new();
    ret.insert(REG::A, "a");
    ret.insert(REG::B, "b");
    ret.insert(REG::C, "c");
    ret.insert(REG::D, "d");
    ret.insert(REG::E, "e");
    ret.insert(REG::PC, "pc");
    ret.insert(REG::SP, "sp");
    ret.insert(REG::INST, "inst");
    ret
}

pub fn decode_instruction(instr: u32) -> String {
    let op_num = (instr & OPCODE_MASK) >> OPCODE_SHIFT;
    let src_num = (instr & SOURCE_MASK) >> SOURCE_SHIFT;
    let dst_num = instr & DEST_MASK;
    let op: INSTRUCTION = num::FromPrimitive::from_u32(op_num).unwrap();
    let src: REG = num::FromPrimitive::from_u32(src_num).unwrap();
    let dst: REG = num::FromPrimitive::from_u32(dst_num).unwrap();
    format!("{} {} {}", mnemonic_names().get(&op).unwrap(), reg_names().get(&src).unwrap(), reg_names().get(&dst).unwrap())
}
