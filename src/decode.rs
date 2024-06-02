
use std::{collections::HashMap};
use crate::decode::INSTRUCTION::*;

use crate::microcodes::{OPCODE_MASK, SOURCE_MASK, DEST_MASK, INSTRUCTION, REG, OPCODE_SHIFT, SOURCE_SHIFT};
use crate::{ControlCable, ControlCables};
use crate::ControlCable::*;
use crate::CONTROL_CABLES_SIZE;

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

fn cable_names() -> HashMap<ControlCable, &'static str> {
    let mut ret = HashMap::new();
    ret.insert(Halt,"Halt");
    ret.insert(MemoryAddressIn,"MemoryAddressIn");
    ret.insert(RamIn,"RamIn");
    ret.insert(RamOut,"RamOut");
    ret.insert(MemoryIsIO,"MemoryIsIO");
    ret.insert(AddMul,"AddMul");
    ret.insert(SubDiv,"SubDiv");
    ret.insert(AluOut,"AluOut");
    ret.insert(Interrupt,"Interrupt");
    ret
}

fn op_names() -> HashMap<usize, &'static str> {
    let mut ret = HashMap::new();
    ret.insert(0, "in");
    ret.insert(1, "out");
    ret.insert(2, "inc");
    ret.insert(3, "dec");
    ret
}

pub fn dump_cables(cables: &ControlCables) -> String {
    let mut ret = String::new();
    for i in 0..CONTROL_CABLES_SIZE {
        if cables[i].load(std::sync::atomic::Ordering::SeqCst) {
            if i < RegBase as usize{
                ret.push_str(cable_names()[&num::FromPrimitive::from_usize(i).unwrap()]);
                ret.push_str(" ");
            } else {
                let reg_num = (i - RegBase as usize)/4;
                let reg_op = (i - RegBase as usize)%4;
                let reg_name = reg_names()[&num::FromPrimitive::from_usize(reg_num).unwrap()];
                let op_name = op_names()[&reg_op];
                ret.push_str(format!("{}_{} ", reg_name, op_name).as_str());
            }
        }
    }
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
