use std::collections::HashMap;

use mmachine::microcodes::INSTRUCTION;
use mmachine::microcodes::INSTRUCTION::*;
use phf::phf_map;

#[derive(Debug)]
enum Statement<'a> {
    Command(&'a INSTRUCTION, Vec<&'a REG>),
    Ldcnst(String),
    Label(String),
}

static MNEMONICS: phf::Map<&'static str, INSTRUCTION> = phf_map! {
    "hlt" => HLT,
    "mov" => MOV,
    "add" => ADD,
    "sub" => SUB,
    "mul" => MUL,
    "div" => DIV,
    "call" => CALL,
    "je" => JE,
    "jne" => JNE,
    "jg" => JG,
    "jge" => JGE,
    "jl" => JL,
    "jle" => JLE,
    "push" => PUSH,
    "pop" => POP,
    "out" => OUT,
    "in" => IN,
    "int" => INT,
    "eoi" => EOI,
    "inc" => INC,
    "dec" => DEC,
    "load" => LOAD,
    "store" => STORE,
    "ldcnst" => LDCNST,
};

#[derive(Debug)]
enum REG {
    A = 0b00000,
    B = 0b00001,
    C = 0b00010,
    D = 0b00011,
    E = 0b00100,
    PC = 0b00101,
    SP = 0b00110,
    INST = 0b00111,
}

static REG_NAMES: phf::Map<&'static str, REG> = phf_map! {
    "a" => REG::A,
    "b" => REG::B,
    "c" => REG::C,
    "d" => REG::D,
    "e" => REG::E,
    "pc" => REG::PC,
    "sp" => REG::SP,
    "inst" => REG::INST,
};

fn populate_labels(labels: &mut HashMap<String, usize>) {}

fn parse_text(text: String) -> Vec<Statement<'static>> {
    let mut ret = Vec::new();
    for dirty_line in text.lines() {
        let line = dirty_line.split(";").collect::<Vec<&str>>()[0]
            .trim()
            .to_lowercase();
        if line.len() == 0 {
            continue;
        }
        let mut tokens: Vec<String> = line.split_whitespace().map(|x| x.to_string()).collect();
        if tokens[0].contains(":") {
            let mut label_name = tokens[0].clone();
            label_name.pop();
            ret.push(Statement::Label(label_name));
            continue;
        }
        let op_code = MNEMONICS.get(tokens.remove(0).as_str()).unwrap();
        if *op_code == LDCNST {
            ret.push(Statement::Ldcnst(tokens[0].clone()));
            continue;
        }
        let regs: Vec<&REG> = tokens.iter().map(|x| REG_NAMES.get(x).unwrap()).collect();
        ret.push(Statement::Command(op_code, regs));
    }
    ret
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let contents = std::fs::read_to_string(args[1].clone()).unwrap();
    // let mut labels = HashMap::new();
    let ast = parse_text(contents);
    println!("{:#?}", ast);
}
