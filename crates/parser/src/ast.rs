// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

#[derive(Debug, PartialEq)]
pub struct ModuleNode {
    // e.g. "network", "network::http", "network::http::client"
    pub name_path: String,
    pub uses: Vec<UseNode>,
    pub externals: Vec<ExternalNode>,
    pub datas: Vec<DataNode>,
    pub functions: Vec<FunctionNode>,
}

// #[derive(Debug, PartialEq)]
// pub struct ModuleNode {
//     pub name_path: String,
//     pub runtime_version: Option<EffectiveVersion>,
//
//     // the relative name path of constructor function
//     // a package can only defined one constructor
//     pub constructor_function_name_path: Option<String>,
//
//     // the relative name path of destructor function
//     // a package can only defined one destructor
//     pub destructor_function_name_path: Option<String>,
//
//     pub depend_node: Option<DependNode>,
//
//     pub element_nodes: Vec<ModuleElementNode>,
// }

// #[derive(Debug, PartialEq)]
// pub struct DependNode {
//     pub depend_items: Vec<DependItem>,
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub enum DependItem {
//     DependentModule(DependentModuleNode),
//     DependentLibrary(DependentLibraryNode),
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub struct DependentModuleNode {
//     pub id: String,
//     pub module_share_type: ModuleShareType,
//     pub name: String,
//     pub module_version: EffectiveVersion,
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub struct DependentLibraryNode {
//     pub id:String,
//     pub external_library_type: ExternalLibraryType,
//     pub name: String, // the library link name
// }

#[derive(Debug, PartialEq)]
pub struct UseNode {
    // e.g. "network", "network::http", "network::http::status_code"
    pub name_path: String,
    pub alias_name: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum ExternalNode {
    Function(ExternalFunction),
    Data(ExternalData),
}

#[derive(Debug, PartialEq)]
pub struct ExternalFunction {
    pub library: String,
    pub name: String,
    pub params: Vec<FunctionDataType>,
    pub returns: Vec<FunctionDataType>,
    pub alias_name: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum FunctionDataType {
    I64,
    I32,
    F64,
    F32,
}

#[derive(Debug, PartialEq)]
pub struct ExternalData {
    pub library: String,
    pub identifier: String,
    pub data_type: ExternalDataType,
    pub alias_name: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum ExternalDataType {
    I64,
    I32,
    F64,
    F32,
    Bytes,                          // e.g. `byte[]`
    FixedBytes(/* length */ usize), // e.g. `byte[1024]`
}

// #[derive(Debug, PartialEq)]
// pub enum ModuleElementNode {
//     FunctionNode(FunctionNode),
//     DataNode(DataNode),
//     ExternalNode(ExternalNode),
//     ImportNode(ImportNode),
// }

#[derive(Debug, PartialEq)]
pub struct DataNode {
    pub is_public: bool,
    pub name: String,
    pub data_section: DataSection,
}

#[derive(Debug, PartialEq)]
pub enum DataSection {
    ReadOnly(InitedDataTypeValuePair),
    ReadWrite(InitedDataTypeValuePair),
    Uninit(MemoryDataType),
}

#[derive(Debug, PartialEq)]
pub struct InitedDataTypeValuePair {
    pub data_type: InitedDataType,
    pub value: InitedDataValue,
}

#[derive(Debug, PartialEq)]
pub enum InitedDataType {
    I64,
    I32,
    F64,
    F32,
    Bytes,                          // e.g. `byte[]`
    FixedBytes(/* length */ usize), // e.g. `byte[1024]`
}

#[derive(Debug, PartialEq)]
pub enum InitedDataValue {
    I64(u64),
    I32(u32),
    F64(f64),
    F32(f32),
    Byte(Vec<u8>),
    String(String),

    // e.g. [11_i32, 13_i32, 17_i32, 19_i32]
    List(Vec<InitedDataValue>),
}

#[derive(Debug, PartialEq)]
pub struct FunctionNode {
    pub is_public: bool,
    pub name: String,
    pub params: Vec<NamedParameter>,
    pub returns: Vec<FunctionDataType>,
    pub locals: Vec<LocalVariable>,
    pub body: Box<ExpressionNode>,
}

#[derive(Debug, PartialEq)]
pub struct NamedParameter {
    pub name: String,
    pub data_type: FunctionDataType,
}

#[derive(Debug, PartialEq)]
pub struct LocalVariable {
    pub name: String,
    pub data_type: MemoryDataType,

    /// if the data is a byte array (e.g. a string), the value should be 1,
    /// if the data is a struct, the value should be the max one of the length of its fields.
    /// the MAX value of align is 8, the MIN value is 1.
    ///
    /// e.g. `align(name:byte[1024], 4)`
    pub align: Option<usize>,
}

#[derive(Debug, PartialEq)]
pub enum MemoryDataType {
    I64,
    I32,
    F64,
    F32,
    FixedBytes(/* length */ usize),
}

// #[derive(Debug, PartialEq)]
// pub struct DataNode {
//     pub is_public:bool,
//     pub section_type: DataSection,
//     pub name: String,
//     pub data_type: InitedDataType,
//     pub value: InitedDataValue
// }
//
// #[derive(Debug, PartialEq)]
// pub struct FunctionNode {
//     // nate that the names of functions can not be duplicated within a module,
//     // including the name of imported functions.
//     pub name: String,
//
//     pub export: bool,
//     pub params: Vec<ParamNode>,
//     pub results: Vec<DataType>,
//     pub locals: Vec<LocalNode>,
//     pub code: Vec<Instruction>,
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub struct ParamNode {
//     // nate that the names of all parameters and local variables within a function
//     // can not be duplicated.
//     pub name: String,
//     pub data_type: DataType,
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub struct LocalNode {
//     // nate that the names of all parameters and local variables within a function
//     // can not be duplicated.
//     pub name: String,
//
//     pub memory_data_type: MemoryDataType,
//     pub data_length: u32,
//
//     // if the data is a byte array (includes string), the value should be 1,
//     // if the data is a struct, the value should be the max one of the length of its fields.
//     // currently the MAX value of align is 8, MIN value is 1.
//     pub align: u16,
// }
//
// #[derive(Debug, PartialEq)]
// pub struct ExternalNode {
//     // pub external_library_node: ExternalLibraryNode,
//     pub library_id: String,
//     pub external_items: Vec<ExternalItem>,
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub enum ExternalItem {
//     ExternalFunction(ExternalFunctionNode),
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub struct ExternalFunctionNode {
//     pub id: String,   // the identifier of the external function for 'extcall' instruction
//     pub name: String, // the original exported name/symbol
//     pub params: Vec<DataType>, // the parameters of external functions have no identifier
//     pub results: Vec<DataType>,
// }
//
// #[derive(Debug, PartialEq)]
// pub struct ImportNode {
//     // pub import_module_node: ImportModuleNode,
//     pub module_id: String,
//     pub import_items: Vec<ImportItem>,
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub enum ImportItem {
//     ImportFunction(ImportFunctionNode),
//     ImportData(ImportDataNode),
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub struct ImportFunctionNode {
//     // the identifier of the imported function for calling instructons
//     pub id: String,
//
//     // the original exported name path,
//     // includes the submodule name path, but excludes the module name.
//     //
//     // e.g.
//     // the name path of functon 'add' in module 'myapp' is 'add',
//     // the name path of function 'add' in submodule 'myapp:utils' is 'utils::add'.
//     pub name_path: String,
//
//     // the parameters of external functions have no identifier
//     pub params: Vec<DataType>,
//     pub results: Vec<DataType>,
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub struct ImportDataNode {
//     // the identifier of the imported data for data loading/storing instructions
//     pub id: String,
//
//     // the original exported name path,
//     // includes the submodule name path, but excludes the module name.
//     //
//     // e.g.
//     // the name path of data 'buf' in module 'myapp' is 'buf',
//     // the name path of data 'buf' in submodule 'myapp:utils' is 'utils::buf'.
//     pub name_path: String,
//
//     // pub data_kind_node: SimplifiedDataKindNode,
//     pub memory_data_type: MemoryDataType,
//
//     pub data_section_type: DataSection,
// }

#[derive(Debug, PartialEq)]
pub enum ExpressionNode {
    Instruction(InstructionNode),
    When(WhenNode),
    If(IfNode),
    For(ForNode),
    Break(BreakNode),
    Recur(RecurNode),
    Group(Vec<ExpressionNode>),
}

#[derive(Debug, PartialEq)]
pub struct WhenNode {
    pub testing: Box<ExpressionNode>,
    pub locals: Vec<LocalVariable>,
    pub consequence: Box<ExpressionNode>,
}

#[derive(Debug, PartialEq)]
pub struct IfNode {
    pub returns: Vec<FunctionDataType>,
    pub testing: Box<ExpressionNode>,
    pub consequence: Box<ExpressionNode>,
    pub alternative: Box<ExpressionNode>,
}

#[derive(Debug, PartialEq)]
pub struct ForNode {
    pub params: Vec<NamedParameter>,
    pub returns: Vec<FunctionDataType>,
    pub locals: Vec<LocalVariable>,
    pub expressions: Box<ExpressionNode>,
}

#[derive(Debug, PartialEq)]
pub enum BreakNode {
    Break(/* values */ Vec<ExpressionNode>),
    BreakIf(
        /* testing */ Box<ExpressionNode>,
        /* values */ Vec<ExpressionNode>,
    ),
    BreakFn(/* values */ Vec<ExpressionNode>),
}

#[derive(Debug, PartialEq)]
pub enum RecurNode {
    Recur(/* values */ Vec<ExpressionNode>),
    RecurIf(
        /* testing */ Box<ExpressionNode>,
        /* values */ Vec<ExpressionNode>,
    ),
    RecurFn(/* values */ Vec<ExpressionNode>),
}

#[derive(Debug, PartialEq)]
pub struct InstructionNode {
    pub name: String,
    pub position_args: Vec<ArgumentValue>,
    pub named_args: Vec<NamedArgument>,
}

#[derive(Debug, PartialEq)]
pub enum ArgumentValue {
    Identifier(String),
    Const(Const),
    Expression(Box<ExpressionNode>),
}

#[derive(Debug, PartialEq)]
pub struct NamedArgument {
    pub name: String,
    pub value: ArgumentValue,
}

#[derive(Debug, PartialEq)]
pub enum Const {
    I64(u64),
    I32(u32),
    I16(u16),
    F64(f64),
    F32(f32),
}

// #[derive(Debug, PartialEq, Clone)]
// pub enum Instruction {
//     // bytecode: ()
//     NoParams {
//         opcode: Opcode,
//         operands: Vec<Instruction>,
//     },
//
//     // bytecode: (param immediate_number:i32)
//     ImmI32(u32),
//
//     // bytecode: (param immediate_number_low:i32, immediate_number_high:i32)
//     ImmI64(u64),
//
//     // bytecode: (param immediate_number:i32)
//     ImmF32(f32),
//
//     // bytecode: (param immediate_number_low:i32, immediate_number_high:i32)
//     ImmF64(f64),
//
//     // bytecode: (param reversed_index:i16 offset_bytes:i16 local_variable_index:i16)
//     LocalLoad {
//         opcode: Opcode,
//         name: String,
//         offset: u16,
//     },
//
//     // bytecode: (param reversed_index:i16 offset_bytes:i16 local_variable_index:i16)
//     LocalStore {
//         opcode: Opcode,
//         name: String,
//         offset: u16,
//         value: Box<Instruction>,
//     },
//
//     // bytecode: (param reversed_index:i16 local_variable_index:i32)
//     LocalLongLoad {
//         opcode: Opcode,
//         name: String,
//         offset: Box<Instruction>,
//     },
//
//     // bytecode: (param reversed_index:i16 local_variable_index:i32)
//     LocalLongStore {
//         opcode: Opcode,
//         name: String,
//         offset: Box<Instruction>,
//         value: Box<Instruction>,
//     },
//
//     // bytecode: (param offset_bytes:i16 data_public_index:i32)
//     DataLoad {
//         opcode: Opcode,
//
//         // the data identifier, or the (relative/absolute) name path
//         id: String,
//         offset: u16,
//     },
//
//     // bytecode: (param offset_bytes:i16 data_public_index:i32)
//     DataStore {
//         opcode: Opcode,
//
//         // the data identifier, or the (relative/absolute) name path
//         id: String,
//         offset: u16,
//         value: Box<Instruction>,
//     },
//
//     // bytecode: (param data_public_index:i32)
//     DataLongLoad {
//         opcode: Opcode,
//
//         // the data identifier, or the (relative/absolute) name path
//         id: String,
//         offset: Box<Instruction>,
//     },
//
//     // bytecode: (param data_public_index:i32)
//     DataLongStore {
//         opcode: Opcode,
//
//         // the data identifier, or the (relative/absolute) name path
//         id: String,
//         offset: Box<Instruction>,
//         value: Box<Instruction>,
//     },
//
//     // bytecode: (param offset_bytes:i16)
//     HeapLoad {
//         opcode: Opcode,
//         offset: u16,
//         addr: Box<Instruction>,
//     },
//
//     // bytecode: (param offset_bytes:i16)
//     HeapStore {
//         opcode: Opcode,
//         offset: u16,
//         addr: Box<Instruction>,
//         value: Box<Instruction>,
//     },
//
//     // bytecode: ()
//     UnaryOp {
//         opcode: Opcode,
//         source: Box<Instruction>,
//     },
//
//     // bytecode: (param imm:i16)
//     UnaryOpWithImmI16 {
//         opcode: Opcode,
//         imm: u16,
//         source: Box<Instruction>,
//     },
//
//     // bytecode: ()
//     BinaryOp {
//         opcode: Opcode,
//         left: Box<Instruction>,
//         right: Box<Instruction>,
//     },
//
//     // bytecode:
//     // - block_nez (param local_list_index:i32, next_inst_offset:i32)
//     When {
//         // structure 'when' has NO params and NO results, NO local variables.
//         // locals: Vec<LocalNode>,
//         test: Box<Instruction>,
//         consequent: Box<Instruction>,
//     },
//
//     // bytecode:
//     // - block_alt (param type_index:i32, local_list_index:i32, alt_inst_offset:i32)
//     // - break (param reversed_index:i16, next_inst_offset:i32)
//     If {
//         // structure 'If' has NO params, NO local variables,
//         // but can return values.
//         results: Vec<DataType>,
//         // locals: Vec<LocalNode>,
//         test: Box<Instruction>,
//         consequent: Box<Instruction>,
//         alternate: Box<Instruction>,
//     },
//
//     // bytecode:
//     // - block (param type_index:i32, local_list_index:i32)
//     // - block_nez (param local_list_index:i32, next_inst_offset:i32)
//     // - break (param reversed_index:i16, next_inst_offset:i32)
//     Branch {
//         // structure 'Branch' has NO params, NO local variables,
//         // but can return values.
//         results: Vec<DataType>,
//         // locals: Vec<LocalNode>,
//         cases: Vec<BranchCase>,
//
//         // the branch 'default' is optional, but for the structure 'branch' with
//         // return value, it SHOULD add instruction 'unreachable' follow the last branch
//         // to avoid missing matches.
//         default: Option<Box<Instruction>>,
//     },
//
//     // bytecode:
//     // - block (param type_index:i32, local_list_index:i32)
//     // - recur (param reversed_index:i16, start_inst_offset:i32)
//     For {
//         params: Vec<ParamNode>,
//         results: Vec<DataType>,
//         locals: Vec<LocalNode>,
//         code: Box<Instruction>,
//     },
//
//     // Code(Vec<Instruction>),
//     Do(Vec<Instruction>),
//
//     // to break the nearest 'for' structure
//     //
//     // bytecode: break (param reversed_index:i16, next_inst_offset:i32)
//     Break(Vec<Instruction>),
//
//     // bytecode: recur (param reversed_index:i16, start_inst_offset:i32)
//     Recur(Vec<Instruction>),
//
//     // bytecode: break (param reversed_index:i16, next_inst_offset:i32)
//     Return(Vec<Instruction>),
//
//     // bytecode: recur (param reversed_index:i16, start_inst_offset:i32)
//     FnRecur(Vec<Instruction>),
//
//     // bytecode: (param function_public_index:i32)
//     Call {
//         // the function identifier (name), or the (relative/absolute) name path
//         id: String,
//         args: Vec<Instruction>,
//     },
//
//     // bytecode: ()
//     DynCall {
//         public_index: Box<Instruction>,
//         args: Vec<Instruction>,
//     },
//
//     // bytecode: (param env_function_num:i32)
//     EnvCall {
//         num: u32,
//         args: Vec<Instruction>,
//     },
//
//     // bytecode: ()
//     SysCall {
//         num: u32,
//         args: Vec<Instruction>,
//     },
//
//     // bytecode: (param external_function_idx:i32)
//     ExtCall {
//         // the external function identifier, or the (relative/absolute) name path
//         id: String,
//         args: Vec<Instruction>,
//     },
//
//     Debug {
//         code: u32,
//     },
//
//     Unreachable {
//         code: u32,
//     },
//
//     // id:
//     // the function identifier (name), or the (relative/absolute) name path
//     HostAddrFunction {
//         id: String,
//     },
//
//     // macro.get_function_public_index
//     //
//     // for obtaining the public index of the specified function
//     //
//     // id:
//     // the function identifier (name), or the (relative/absolute) name path
//     MacroGetFunctionPublicIndex {
//         id: String,
//     },
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub struct BranchCase {
//     pub test: Box<Instruction>,
//     pub consequent: Box<Instruction>,
// }
//
// #[derive(Debug, PartialEq)]
// pub struct DataNode {
//     // the names of data can not be duplicated within a module,
//     // including the name of imported data.
//     pub name: String,
//     pub export: bool,
//     pub data_detail: DataDetailNode,
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub enum DataDetailNode {
//     ReadOnly(InitedData),
//     ReadWrite(InitedData),
//     Uninit(UninitData),
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub struct InitedData {
//     pub memory_data_type: MemoryDataType,
//     pub length: u32,
//
//     // if the data is a byte array (includes string), the value should be 1,
//     // if the data is a struct, the value should be the max one of the length of its fields.
//     // currently the MIN value is 1.
//     pub align: u16,
//     pub value: Vec<u8>,
// }
//
// #[derive(Debug, PartialEq, Clone)]
// pub struct UninitData {
//     pub memory_data_type: MemoryDataType,
//     pub length: u32,
//
//     // if the data is a byte array (includes string), the value should be 1,
//     // if the data is a struct, the value should be the max one of the length of its fields.
//     // currently the MIN value is 1.
//     pub align: u16,
// }
