// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::{any::Any, fmt::Display};

use ancvm_binary::module_image::{
    data_index_section::DataIndexModuleEntry,
    data_name_section::DataNameEntry,
    data_section::{InitedDataEntry, UninitDataEntry},
    external_func_name_section::ExternalFuncNameEntry,
    func_index_section::FuncIndexModuleEntry,
    func_name_section::FuncNameEntry,
    func_section::FuncEntry,
    local_variable_section::LocalListEntry,
    type_section::TypeEntry,
};
use ancvm_types::VMError;

pub mod assembler;
pub mod linker;

pub struct ModuleEntry {
    pub name: String,
    pub runtime_version_major: u16,
    pub runtime_version_minor: u16,

    pub type_entries: Vec<TypeEntry>,
    pub local_list_entries: Vec<LocalListEntry>,
    pub func_entries: Vec<FuncEntry>,

    pub read_only_data_entries: Vec<InitedDataEntry>,
    pub read_write_data_entries: Vec<InitedDataEntry>,
    pub uninit_data_entries: Vec<UninitDataEntry>,

    pub func_name_entries: Vec<FuncNameEntry>,
    pub data_name_entries: Vec<DataNameEntry>,
    pub external_func_name_entries: Vec<ExternalFuncNameEntry>,
}

pub struct IndexEntry {
    // essential
    pub func_index_module_entries: Vec<FuncIndexModuleEntry>,
    // optional
    pub data_index_module_entries: Vec<DataIndexModuleEntry>,
}

#[derive(Debug)]
pub struct AssembleError {
    pub message: String,
}

impl AssembleError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }
}

impl Display for AssembleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Assemble error: {}", self.message)
    }
}

impl VMError for AssembleError {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
