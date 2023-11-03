// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_types::{opcode::Opcode, DataType, MemoryDataType};

#[derive(Debug, PartialEq)]
pub struct ModuleNode {
    // module names can not be duplicated
    pub name: String,

    pub runtime_version_major: u16,
    pub runtime_version_minor: u16,

    pub shared_packages: Vec<String>,
    pub element_nodes: Vec<ModuleElementNode>,
}

#[derive(Debug, PartialEq)]
pub enum ModuleElementNode {
    FuncNode(FuncNode),
    TODONode,
}

#[derive(Debug, PartialEq)]
pub struct FuncNode {
    // the names of functions (includes imported function)
    // in a module can not be duplicated.
    pub name: Option<String>,

    pub exported: bool,
    pub params: Vec<ParamNode>,
    pub results: Vec<DataType>,
    pub locals: Vec<LocalNode>,
    pub code: Box<Instruction>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ParamNode {
    // the names of all parameters and local variables in a function
    // can not be duplicated.
    pub name: String,
    pub data_type: DataType,
}

#[derive(Debug, PartialEq, Clone)]
pub struct LocalNode {
    // the names of all parameters and local variables in a function
    // can not be duplicated.
    pub name: String,

    pub memory_data_type: MemoryDataType,
    pub data_length: u32,

    // if the data is a byte array (includes string), the value should be 1,
    // if the data is a struct, the value should be the max one of the length of its fields.
    // currently the MAX value of align is 8, MIN value is 1.
    pub align: u16,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Instruction {
    // bytecode: ()
    NoParams {
        opcode: Opcode,
        operands: Vec<Instruction>,
    },

    // bytecode: (param immediate_number:i32)
    ImmI32(u32),

    // bytecode: (param immediate_number_low:i32, immediate_number_high:i32)
    ImmI64(u64),

    // bytecode: (param immediate_number:i32)
    ImmF32(ImmF32),

    // bytecode: (param immediate_number_low:i32, immediate_number_high:i32)
    ImmF64(ImmF64),

    // bytecode: (param reversed_index:i16 offset_bytes:i16 local_variable_index:i16)
    LocalLoad {
        opcode: Opcode,
        name: String,
        offset: u16,
    },

    // bytecode: (param reversed_index:i16 offset_bytes:i16 local_variable_index:i16)
    LocalStore {
        opcode: Opcode,
        name: String,
        offset: u16,
        value: Box<Instruction>,
    },

    // bytecode: (param reversed_index:i16 local_variable_index:i32)
    LocalLongLoad {
        opcode: Opcode,
        name: String,
        offset: Box<Instruction>,
    },

    // bytecode: (param reversed_index:i16 local_variable_index:i32)
    LocalLongStore {
        opcode: Opcode,
        name: String,
        offset: Box<Instruction>,
        value: Box<Instruction>,
    },

    // bytecode: (param offset_bytes:i16 data_public_index:i32)
    DataLoad {
        opcode: Opcode,
        name: String,
        offset: u16,
    },

    // bytecode: (param offset_bytes:i16 data_public_index:i32)
    DataStore {
        opcode: Opcode,
        name: String,
        offset: u16,
        value: Box<Instruction>,
    },

    // bytecode: (param data_public_index:i32)
    DataLongLoad {
        opcode: Opcode,
        name: String,
        offset: Box<Instruction>,
    },

    // bytecode: (param data_public_index:i32)
    DataLongStore {
        opcode: Opcode,
        name: String,
        offset: Box<Instruction>,
        value: Box<Instruction>,
    },

    // bytecode: (param offset_bytes:i16)
    HeapLoad {
        opcode: Opcode,
        offset: u16,
        addr: Box<Instruction>,
    },

    // bytecode: (param offset_bytes:i16)
    HeapStore {
        opcode: Opcode,
        offset: u16,
        addr: Box<Instruction>,
        value: Box<Instruction>,
    },

    // bytecode: ()
    UnaryOp {
        opcode: Opcode,
        number: Box<Instruction>,
    },

    // bytecode: (param amount:i16)
    UnaryOpParamI16 {
        opcode: Opcode,
        amount: u16,
        number: Box<Instruction>,
    },

    // bytecode: ()
    BinaryOp {
        opcode: Opcode,
        left: Box<Instruction>,
        right: Box<Instruction>,
    },

    // bytecode:
    // - block_nez (param local_list_index:i32, next_inst_offset:i32)
    When {
        // structure 'when' has NO params and NO results, however,
        // can contains local variables.
        locals: Vec<LocalNode>,
        test: Box<Instruction>,
        consequent: Box<Instruction>,
    },

    // bytecode:
    // - block_alt (param type_index:i32, local_list_index:i32, alt_inst_offset:i32)
    // - break (param reversed_index:i16, next_inst_offset:i32)
    If {
        params: Vec<ParamNode>,
        results: Vec<DataType>,
        locals: Vec<LocalNode>,
        test: Box<Instruction>,
        consequent: Box<Instruction>,
        alternate: Box<Instruction>,
    },

    // bytecode:
    // - block (param type_index:i32, local_list_index:i32)
    // - block_nez (param local_list_index:i32, next_inst_offset:i32)
    // - break (param reversed_index:i16, next_inst_offset:i32)
    Branch {
        params: Vec<ParamNode>,
        results: Vec<DataType>,
        locals: Vec<LocalNode>,
        cases: Vec<BranchCase>,

        // the branch 'default' is optional, but for the structure 'branch' with
        // return value, it SHOULD add instruction 'unreachable' follow the last branch
        // to avoid missing matches.
        default: Option<Box<Instruction>>,
    },

    // bytecode:
    // - block (param type_index:i32, local_list_index:i32)
    // - recur (param reversed_index:i16, start_inst_offset:i32)
    For {
        params: Vec<ParamNode>,
        results: Vec<DataType>,
        locals: Vec<LocalNode>,
        code: Box<Instruction>,
    },

    Code(Vec<Instruction>),
    Do(Vec<Instruction>),

    // bytecode: break (param reversed_index:i16, next_inst_offset:i32)
    Break(Vec<Instruction>),

    // bytecode: recur (param reversed_index:i16, start_inst_offset:i32)
    Recur(Vec<Instruction>),

    // bytecode: break (param reversed_index:i16, next_inst_offset:i32)
    Return(Vec<Instruction>),

    // bytecode: recur (param reversed_index:i16, start_inst_offset:i32)
    TailCall(Vec<Instruction>),

    // bytecode: (param func_pub_index:i32)
    Call {
        name: String,
        args: Vec<Instruction>,
    },

    // bytecode: ()
    DynCall {
        num: Box<Instruction>,
        args: Vec<Instruction>,
    },

    // bytecode: (param env_func_num:i32)
    EnvCall {
        num: u32,
        args: Vec<Instruction>,
    },

    // bytecode: ()
    SysCall {
        num: u32,
        args: Vec<Instruction>,
    },

    // bytecode: ()
    ExtCall {
        name: String,
        args: Vec<Instruction>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum ImmF32 {
    Float(f32),
    Hex(u32),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ImmF64 {
    Float(f64),
    Hex(u64),
}

#[derive(Debug, PartialEq, Clone)]
pub struct BranchCase {
    pub test: Box<Instruction>,
    pub consequent: Box<Instruction>,
}
