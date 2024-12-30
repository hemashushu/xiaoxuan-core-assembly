// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

pub mod assembler;

// https://doc.rust-lang.org/reference/conditional-compilation.html#debug_assertions
// https://doc.rust-lang.org/reference/conditional-compilation.html#test
#[cfg(debug_assertions)]
pub mod utils;

#[derive(Debug)]
pub struct AssemblerError {
    pub error_type: AssembleErrorType,
}

#[derive(Debug)]
pub enum AssembleErrorType {
    FunctionNotFound(String),
    DataNotFound(String),
    ExternalFunctionNotFound(String),
    ExternalDataNotFound(String),
    LocalVariableNotFound {
        local_variable_name: String,
        function_name: String,
    },
    ImportModuleNotFound(String),
    ExternalLibraryNotFound(String),

    /// the last control flow does not close.
    IncompleteControlFlow {
        control_flow_path: String,
        function_name: String,
    },
    DuplicatedLocalVariable {
        variable_name: String,
        function_name: String,
    },
    IncorrectDataValueType {
        expected: String,
        actual: String,
        data_name: String,
    },
    IncorrectInstructionParameterType {
        expected: String,
        actual: String,
        instruction_name: String,
        function_name: String,
    },
    UnknownInstruction {
        instruction_name: String,
        function_name: String,
    },
}

impl AssemblerError {
    pub fn new(error_type: AssembleErrorType) -> Self {
        Self { error_type }
    }
}

impl Display for AssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.error_type {
            AssembleErrorType::FunctionNotFound(function_name) => write!(f, "Can not find the function \"{function_name}\"."),
            AssembleErrorType::DataNotFound(data_name) => write!(f, "Can not find the data \"{data_name}\"."),
            AssembleErrorType::ExternalFunctionNotFound(external_function_name) => write!(f, "Can not find the external function \"{external_function_name}\"."),
            AssembleErrorType::ExternalDataNotFound(external_data_name) => write!(f, "Can not find the external data \"{external_data_name}\"."),
            AssembleErrorType::LocalVariableNotFound { local_variable_name: variable_name, function_name } => write!(f, "Can not find the local variable \"{variable_name}\" in function \"{function_name}\"."),
            AssembleErrorType::ImportModuleNotFound(module_name) => write!(f, "Can not find the import module \"{module_name}\"."),
            AssembleErrorType::ExternalLibraryNotFound(external_library_name) => write!(f, "Can not find the external library \"{external_library_name}\"."),
            AssembleErrorType::IncompleteControlFlow { control_flow_path: flow_path, function_name } => write!(f,
                "Incomplete control flow \"{flow_path}\" in function \"{function_name}\"."),
            AssembleErrorType::DuplicatedLocalVariable { variable_name, function_name } => write!(f,
                "Duplicated local variable \"{variable_name}\" in function \"{function_name}\"."),
            AssembleErrorType::IncorrectDataValueType { expected, actual , data_name} => write!(f,
                "Incorrect value type for data \"{data_name}\", expected \"{expected}\", actual \"{actual}\"."),
            AssembleErrorType::IncorrectInstructionParameterType { expected, actual, instruction_name, function_name } => write!(f,
                "Incorrect parameter for instruction \"{instruction_name}\" in function \"{function_name}\", expected \"{expected}\", actual \"{actual}\"."),
            AssembleErrorType::UnknownInstruction { instruction_name, function_name } => write!(f,
                "Unknown instruction \"{instruction_name}\" in function \"{function_name}\"."),

        }
    }
}

impl std::error::Error for AssemblerError {}
