use std::io::Write;
use std::{collections::HashMap, path::PathBuf};

use clap::{Parser};
use mmachine::microcodes::{INSTRUCTION::*, SOURCE_SHIFT};
use mmachine::microcodes::{INSTRUCTION, OPCODE_SHIFT};
use phf::phf_map;
use regex::Regex;

#[derive(Debug)]
enum Statement<'a> {
    Command(&'a INSTRUCTION, Vec<&'a REG>),
    Ldcnst(&'a REG, String),
    Label(String),
    Data(String),
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

#[derive(Debug, Clone, Copy)]
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

fn populate_labels(statements: &Vec<Statement>, labels: &mut HashMap<String, u16>) {
    let mut offset: u16 = 0;
    for s in statements {
        match s {
            Statement::Command(_, _) => offset += 1,
            Statement::Ldcnst(_, _) => offset += 2,
            Statement::Label(l) => {
                labels.insert(l.to_string(), offset);
            }
            Statement::Data(d) => offset += d.len() as u16,
        }
    }
}

fn parse_data(tokens: Vec<String>) -> Statement<'static> {
    let line = tokens.join(" ");
    let re = Regex::new("\"([a-zA-Z0-9! ]+)\"").unwrap();
    if !re.is_match(&line) {
        panic!("wrong data format: {}", line);
    }
    let data = &re.captures(&line).unwrap()[1];
    Statement::Data(data.to_string())
}

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
        let temp = tokens.remove(0);
        if temp == "data" {
            ret.push(parse_data(tokens));
            continue;
        }
        let mnemonic = temp.as_str();
        let maybe_op_code = MNEMONICS.get(mnemonic);
        if maybe_op_code.is_none() {
            panic!("wrong mnemonic: {}", mnemonic);
        }
        let op_code = maybe_op_code.unwrap();
        if *op_code == LDCNST {
            ret.push(Statement::Ldcnst(REG_NAMES.get(&tokens[0]).unwrap(), tokens[1].clone()));
            continue;
        }
        let regs: Vec<&REG> = tokens.iter().map(|x| REG_NAMES.get(x).unwrap()).collect();
        ret.push(Statement::Command(op_code, regs));
    }
    ret
}

fn generate_binary(ast: &Vec<Statement>, labels: &HashMap<String, u16>) -> Vec<u16> {
    let mut ret = vec![];
    for s in ast {
        let mut opcode: u16 = 0;
        match s {
            Statement::Command(c, args) => {
                opcode |= (**c as u16) << OPCODE_SHIFT;
                if args.len() == 1 {
                    if **c == PUSH {
                        opcode |= (*args[0] as u16) << SOURCE_SHIFT;
                    } else {
                        opcode |= *args[0] as u16; // destination
                    }
                } else if args.len() == 2 {
                    opcode |= (*args[0] as u16) << SOURCE_SHIFT;
                    opcode |= *args[1] as u16;
                }
                ret.push(opcode);
            }
            Statement::Ldcnst(reg, data) => {
                opcode |= (LDCNST as u16) << OPCODE_SHIFT;
                opcode |= **reg as u16;
                ret.push(opcode);
                let maybe_constant: Result<u16, _> = data.parse();
                match maybe_constant {
                    Ok(constant) => ret.push(constant),
                    Err(_) => {
                        let label_location = labels.get(data).unwrap();
                        ret.push(*label_location);
                    },
                }
            },
            Statement::Label(_) => {},
            Statement::Data(d) => {
                for c in d.chars() {
                    ret.push(c as u16);
                }
            },
        }
    }
    ret
}

pub fn to_bytes(input: Vec<u16>) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(2 * input.len());

    for value in input {
        bytes.extend(&value.to_be_bytes());
    }

    bytes
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// the source mmasm file
    src_file: PathBuf,

    /// the output binary file
    #[arg(short, long)]
    output: PathBuf,
}

fn main() {
    let args = Args::parse();
    let contents = std::fs::read_to_string(args.src_file).unwrap();
    let mut labels = HashMap::new();
    let ast = parse_text(contents);
    populate_labels(&ast, &mut labels);
    let bin = generate_binary(&ast, &labels);
    let mut f = std::fs::File::create(args.output).unwrap();
    f.write_all(&to_bytes(bin)).unwrap();
}
