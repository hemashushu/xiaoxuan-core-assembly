// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

use anc_isa::{DataSectionType, MemoryDataType, OperandDataType};

#[derive(Debug, PartialEq)]
pub struct ModuleNode {
    pub imports: Vec<ImportNode>,
    pub externals: Vec<ExternalNode>,
    pub datas: Vec<DataNode>,
    pub functions: Vec<FunctionNode>,
}

#[derive(Debug, PartialEq)]
pub enum ImportNode {
    Function(ImportFunctionNode),
    Data(ImportDataNode),
}

#[derive(Debug, PartialEq)]
pub struct ImportFunctionNode {
    /// about the "full_name" and "name_path"
    /// -------------------------------------
    /// - "full_name" = "module_name::name_path"
    /// - "name_path" = "namespace::identifier"
    /// - "namespace" = "sub_module_name"{0,N}
    pub full_name: String,
    pub params: Vec<OperandDataType>,
    pub results: Vec<OperandDataType>,
    pub alias_name: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct ImportDataNode {
    pub data_section_type: DataSectionType,
    pub full_name: String,
    pub data_type: MemoryDataType,
    pub alias_name: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum ExternalNode {
    Function(ExternalFunctionNode),
    Data(ExternalDataNode),
}

#[derive(Debug, PartialEq)]
pub struct ExternalFunctionNode {
    pub full_name: String,
    pub params: Vec<OperandDataType>,
    pub result: Option<OperandDataType>,
    pub alias_name: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct ExternalDataNode {
    pub full_name: String,
    pub data_type: MemoryDataType,
    pub alias_name: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct DataNode {
    // field 'export' is used to indicate the visibility of this item when this
    // module is used as a shared module.
    // Note that in the case of static linking, the item is always
    // visible to other modules, regardless of the value of this property.
    pub export: bool,
    pub name: String,
    pub data_section: DataSection,
}

#[derive(Debug, PartialEq)]
pub enum DataSection {
    ReadOnly(DataTypeValuePair),
    ReadWrite(DataTypeValuePair),
    Uninit(FixedDeclareDataType),
}

#[derive(Debug, PartialEq)]
pub struct DataTypeValuePair {
    pub data_type: DeclareDataType,
    pub value: DataValue,
}

#[derive(Debug, PartialEq)]
pub enum DeclareDataType {
    I64,
    I32,
    F64,
    F32,

    // e.g. `byte[]`, `byte[align=4]`
    Bytes(/* align */ Option<usize>),

    // e.g. `byte[1024]`, `byte[1024, align=4]`
    FixedBytes(/* length */ usize, /* align */ Option<usize>),
}

#[derive(Debug, PartialEq)]
pub enum DataValue {
    I8(u8),
    I16(u16),
    I64(u64),
    I32(u32),
    F64(f64),
    F32(f32),
    String(String),
    ByteData(Vec<u8>),

    // e.g. [11_i32, 13_i32, 17_i32, 19_i32]
    List(Vec<DataValue>),
}

#[derive(Debug, PartialEq)]
pub struct FunctionNode {
    // field 'export' is used to indicate the visibility of this item when this
    // module is used as a shared module.
    // Note that in the case of static linking, the item is always
    // visible to other modules, regardless of the value of this property.
    pub export: bool,
    pub name: String,
    pub params: Vec<NamedParameter>,
    pub results: Vec<OperandDataType>,
    pub locals: Vec<LocalVariable>,
    pub body: Box<ExpressionNode>,
}

#[derive(Debug, PartialEq)]
pub struct NamedParameter {
    pub name: String,
    pub data_type: OperandDataType,
}

#[derive(Debug, PartialEq)]
pub struct LocalVariable {
    pub name: String,
    pub data_type: FixedDeclareDataType,
}

#[derive(Debug, PartialEq)]
pub enum FixedDeclareDataType {
    I64,
    I32,
    F64,
    F32,

    /// - if the content of the data is a byte array (e.g. a string),
    ///   the value should be 1,
    /// - if the content of the data is a struct, the value should be
    ///   the max length of its fields.
    /// - for local variables, the MAX value of align is 8, the MIN value is 1.
    /// - the value should not be 0.
    ///
    /// e.g. `name:byte[1024, align=4]`
    FixedBytes(/* length */ usize, /* align */ Option<usize>),
}

#[derive(Debug, PartialEq)]
pub enum ExpressionNode {
    Group(Vec<ExpressionNode>),
    Instruction(InstructionNode),
    When(WhenNode),
    If(IfNode),
    // Branch(BranchNode),
    Block(BlockNode),
    Break(BreakNode),
    Recur(BreakNode),
}

#[derive(Debug, PartialEq)]
pub struct WhenNode {
    pub testing: Box<ExpressionNode>,
    pub locals: Vec<LocalVariable>,
    pub consequence: Box<ExpressionNode>,
}

// #[derive(Debug, PartialEq)]
// pub struct BranchNode {
//     pub params: Vec<NamedParameter>,
//     pub results: Vec<OperandDataType>,
//     pub locals: Vec<LocalVariable>,
//     pub cases: Vec<CaseNode>,
//     pub default: Box<ExpressionNode>
// }
//
// #[derive(Debug, PartialEq)]
// pub struct CaseNode {
//     pub testing: Box<ExpressionNode>,
//     pub consequence: Box<ExpressionNode>,
// }

#[derive(Debug, PartialEq)]
pub struct IfNode {
    pub params: Vec<NamedParameter>,
    pub results: Vec<OperandDataType>,
    pub testing: Box<ExpressionNode>,
    pub consequence: Box<ExpressionNode>,
    pub alternative: Box<ExpressionNode>,
}

#[derive(Debug, PartialEq)]
pub struct BlockNode {
    pub params: Vec<NamedParameter>,
    pub results: Vec<OperandDataType>,
    pub locals: Vec<LocalVariable>,
    pub body: Box<ExpressionNode>,
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
pub struct InstructionNode {
    pub name: String,
    pub positional_args: Vec<ArgumentValue>,
    pub named_args: Vec<NamedArgument>,
}

#[derive(Debug, PartialEq)]
pub enum ArgumentValue {
    // The identifier can only bet the name of function or data.
    // not includes the name path or full name
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    Identifier(String),

    LiteralNumber(LiteralNumber),

    Expression(Box<ExpressionNode>),
}

#[derive(Debug, PartialEq)]
pub struct NamedArgument {
    pub name: String,
    pub value: ArgumentValue,
}

#[derive(Debug, PartialEq)]
pub enum LiteralNumber {
    I8(u8),
    I16(u16),
    I32(u32),
    I64(u64),
    F32(f32),
    F64(f64),
}

impl Display for DeclareDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeclareDataType::I64 => f.write_str("i64"),
            DeclareDataType::I32 => f.write_str("i32"),
            DeclareDataType::F64 => f.write_str("f64"),
            DeclareDataType::F32 => f.write_str("f32"),
            DeclareDataType::Bytes(opt_align) => {
                if let Some(align) = opt_align {
                    write!(f, "byte[align={}]", align)
                } else {
                    f.write_str("byte[]")
                }
            }
            DeclareDataType::FixedBytes(length, opt_align) => {
                if let Some(align) = opt_align {
                    write!(f, "byte[{}, align={}]", length, align)
                } else {
                    write!(f, "byte[{}]", length)
                }
            }
        }
    }
}

impl Display for FixedDeclareDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FixedDeclareDataType::I64 => f.write_str("i64"),
            FixedDeclareDataType::I32 => f.write_str("i32"),
            FixedDeclareDataType::F64 => f.write_str("f64"),
            FixedDeclareDataType::F32 => f.write_str("f32"),
            FixedDeclareDataType::FixedBytes(length, opt_align) => {
                if let Some(align) = opt_align {
                    write!(f, "byte[{}, align={}]", length, align)
                } else {
                    write!(f, "byte[{}]", length)
                }
            }
        }
    }
}
