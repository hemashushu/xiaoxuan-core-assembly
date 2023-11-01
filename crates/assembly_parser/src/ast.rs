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
    NoParams {
        opcode: Opcode,
        operands: Vec<Instruction>,
    },

    ImmI32(u32),
    ImmI64(u64),
    ImmF32(ImmF32),
    ImmF64(ImmF64),

    LocalLoad {
        opcode: Opcode,
        name: String,
        offset: u16,
    },

    LocalStore {
        opcode: Opcode,
        name: String,
        offset: u16,
        value: Box<Instruction>,
    },

    LocalLongLoad {
        opcode: Opcode,
        name: String,
        offset: Box<Instruction>,
    },

    LocalLongStore {
        opcode: Opcode,
        name: String,
        offset: Box<Instruction>,
        value: Box<Instruction>,
    },

    DataLoad {
        opcode: Opcode,
        name: String,
        offset: u16,
    },

    DataStore {
        opcode: Opcode,
        name: String,
        offset: u16,
        value: Box<Instruction>,
    },

    DataLongLoad {
        opcode: Opcode,
        name: String,
        offset: Box<Instruction>,
    },

    DataLongStore {
        opcode: Opcode,
        name: String,
        offset: Box<Instruction>,
        value: Box<Instruction>,
    },

    HeapLoad {
        opcode: Opcode,
        offset: u16,
        addr: Box<Instruction>,
    },

    HeapStore {
        opcode: Opcode,
        offset: u16,
        addr: Box<Instruction>,
        value: Box<Instruction>,
    },

    UnaryOp {
        opcode: Opcode,
        number: Box<Instruction>,
    },

    UnaryOpParamI16 {
        opcode: Opcode,
        amount: u16,
        number: Box<Instruction>,
    },

    BinaryOp {
        opcode: Opcode,
        left: Box<Instruction>,
        right: Box<Instruction>,
    },

    When {
        // structure 'when' has NO params and NO results, however,
        // can contains local variables.
        locals: Vec<LocalNode>,
        test: Box<Instruction>,
        consequent: Box<Instruction>,
    },

    If {
        params: Vec<ParamNode>,
        results: Vec<DataType>,
        locals: Vec<LocalNode>,
        test: Box<Instruction>,
        consequent: Box<Instruction>,
        alternate: Box<Instruction>,
    },

    Branch {
        params: Vec<ParamNode>,
        results: Vec<DataType>,
        locals: Vec<LocalNode>,
        cases: Vec<BranchCase>,
        default: Option<Box<Instruction>>,
    },

    For {
        params: Vec<ParamNode>,
        results: Vec<DataType>,
        locals: Vec<LocalNode>,
        code: Box<Instruction>,
    },

    Code(Vec<Instruction>),
    Do(Vec<Instruction>),
    Break(Vec<Instruction>),
    Recur(Vec<Instruction>),
    Return(Vec<Instruction>),
    TailCall(Vec<Instruction>),

    Call {
        name: String,
        args: Vec<Instruction>,
    },

    DynCall {
        num: Box<Instruction>,
        args: Vec<Instruction>,
    },

    EnvCall {
        num: u32,
        args: Vec<Instruction>,
    },

    SysCall {
        num: u32,
        args: Vec<Instruction>,
    },

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
