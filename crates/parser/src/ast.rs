// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_types::{
    opcode::Opcode, DataSectionType, DataType, ExternalLibraryType, MemoryDataType, ModuleShareType,
};

#[derive(Debug, PartialEq)]
pub struct ModuleNode {
    // module names can not be duplicated
    pub name_path: String,

    pub runtime_version_major: u16,
    pub runtime_version_minor: u16,

    pub constructor_function_name:Option<String>,
    pub destructor_function_name:Option<String>,

    pub element_nodes: Vec<ModuleElementNode>,
}

#[derive(Debug, PartialEq)]
pub enum ModuleElementNode {
    FunctionNode(FunctionNode),
    DataNode(DataNode),
    ExternalNode(ExternalNode),
    ImportNode(ImportNode)
}

#[derive(Debug, PartialEq)]
pub struct FunctionNode {
    // the names of functions (includes imported function)
    // in a module can not be duplicated.
    pub name: String,

    pub exported: bool,
    pub params: Vec<ParamNode>,
    pub results: Vec<DataType>,
    pub locals: Vec<LocalNode>,
    pub code: Vec<Instruction>,
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

#[derive(Debug, PartialEq)]
pub struct ExternalNode {
    pub external_library_node: ExternalLibraryNode,
    pub external_items: Vec<ExternalItem>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExternalItem {
    ExternalFunction(ExternalFunctionNode),
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExternalLibraryNode {
    pub external_library_type: ExternalLibraryType,
    pub name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ExternalFunctionNode {
    pub id: String,          // the identifier of the external function
    pub name: String,        // the original exported name/symbol
    pub params: Vec<DataType>, // the parameters of external functions have no identifier
    pub results: Vec<DataType>,
}

#[derive(Debug, PartialEq)]
pub struct ImportNode {
    pub import_module_node: ImportModuleNode,
    pub import_items: Vec<ImportItem>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ImportItem {
    ImportFunction(ImportFunctionNode),
    ImportData(ImportDataNode),
}

#[derive(Debug, PartialEq, Clone)]
pub struct ImportModuleNode {
    pub module_share_type: ModuleShareType,
    pub name: String,
    pub version_major: u16,
    pub version_minor: u16
}

#[derive(Debug, PartialEq, Clone)]
pub struct ImportFunctionNode {
    pub id: String,            // the identifier of the imported function
    pub name_path: String,     // the original exported name path (excludes the module name)
    pub params: Vec<DataType>, // the parameters of external functions have no identifier
    pub results: Vec<DataType>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ImportDataNode {
    pub id: String,        // the identifier of the imported data
    pub name_path: String, // the original exported name path (excludes the module name)
    pub data_section_type: DataSectionType,
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
        name_path: String,
        offset: u16,
    },

    // bytecode: (param offset_bytes:i16 data_public_index:i32)
    DataStore {
        opcode: Opcode,
        name_path: String,
        offset: u16,
        value: Box<Instruction>,
    },

    // bytecode: (param data_public_index:i32)
    DataLongLoad {
        opcode: Opcode,
        name_path: String,
        offset: Box<Instruction>,
    },

    // bytecode: (param data_public_index:i32)
    DataLongStore {
        opcode: Opcode,
        name_path: String,
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
        // structure 'when' has NO params and NO results, NO local variables.
        // locals: Vec<LocalNode>,
        test: Box<Instruction>,
        consequent: Box<Instruction>,
    },

    // bytecode:
    // - block_alt (param type_index:i32, local_list_index:i32, alt_inst_offset:i32)
    // - break (param reversed_index:i16, next_inst_offset:i32)
    If {
        // structure 'If' has NO params, NO local variables,
        // but can return values.
        results: Vec<DataType>,
        // locals: Vec<LocalNode>,
        test: Box<Instruction>,
        consequent: Box<Instruction>,
        alternate: Box<Instruction>,
    },

    // bytecode:
    // - block (param type_index:i32, local_list_index:i32)
    // - block_nez (param local_list_index:i32, next_inst_offset:i32)
    // - break (param reversed_index:i16, next_inst_offset:i32)
    Branch {
        // structure 'Branch' has NO params, NO local variables,
        // but can return values.
        results: Vec<DataType>,
        // locals: Vec<LocalNode>,
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

    // Code(Vec<Instruction>),
    Do(Vec<Instruction>),

    // to break the nearest 'for' structure
    //
    // bytecode: break (param reversed_index:i16, next_inst_offset:i32)
    Break(Vec<Instruction>),

    // bytecode: recur (param reversed_index:i16, start_inst_offset:i32)
    Recur(Vec<Instruction>),

    // bytecode: break (param reversed_index:i16, next_inst_offset:i32)
    Return(Vec<Instruction>),

    // bytecode: recur (param reversed_index:i16, start_inst_offset:i32)
    Rerun(Vec<Instruction>),

    // bytecode: (param function_public_index:i32)
    Call {
        name_path: String,
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

    // bytecode: (param external_func_idx:i32)
    ExtCall {
        name_path: String,
        args: Vec<Instruction>,
    },

    Debug(/* code */ u32),
    Unreachable(/* code */ u32),
    HostAddrFunction(/* name_path */ String),

    // macro.get_function_public_index
    //
    // for obtaining the public index of the specified function
    MacroGetFunctionPublicIndex(/* name_path */ String),
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

#[derive(Debug, PartialEq)]
pub struct DataNode {
    // the names of datas (includes imported data)
    // in a module can not be duplicated.
    pub name: String,
    pub exported: bool,
    pub data_kind: DataKindNode,
}

#[derive(Debug, PartialEq, Clone)]
pub enum DataKindNode {
    ReadOnly(InitedData),
    ReadWrite(InitedData),
    Uninit(UninitData),
}

#[derive(Debug, PartialEq, Clone)]
pub struct InitedData {
    pub memory_data_type: MemoryDataType,
    pub length: u32,

    // if the data is a byte array (includes string), the value should be 1,
    // if the data is a struct, the value should be the max one of the length of its fields.
    // currently the MIN value is 1.
    pub align: u16,
    pub value: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct UninitData {
    pub memory_data_type: MemoryDataType,
    pub length: u32,

    // if the data is a byte array (includes string), the value should be 1,
    // if the data is a struct, the value should be the max one of the length of its fields.
    // currently the MIN value is 1.
    pub align: u16,
}
