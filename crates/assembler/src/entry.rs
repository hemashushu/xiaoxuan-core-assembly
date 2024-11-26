// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

#[derive(Debug)]
pub struct CommonEntry {
    pub name: String,
    pub runtime_version: EffectiveVersion,
    pub import_read_only_data_count: usize,
    pub import_read_write_data_count: usize,
    pub import_uninit_data_count: usize,
    pub import_function_count: usize,
    pub constructor_function_public_index: Option<u32>,
    pub destructor_function_public_index: Option<u32>,

    pub read_only_data_entries: Vec<InitedDataEntry>,
    pub read_write_data_entries: Vec<InitedDataEntry>,
    pub uninit_data_entries: Vec<UninitDataEntry>,

    pub type_entries: Vec<TypeEntry>,
    pub local_list_entries: Vec<LocalVariableListEntry>,
    pub function_entries: Vec<FunctionEntry>,

    pub external_library_entries: Vec<ExternalLibraryEntry>,
    pub external_function_entries: Vec<ExternalFunctionEntry>,

    // the dependencies
    pub import_module_entries: Vec<ImportModuleEntry>,

    // the import_function_entries, import_data_entries,
    // function_name_entries, data_name_entries are
    // used for linking.
    pub import_function_entries: Vec<ImportFunctionEntry>,
    pub import_data_entries: Vec<ImportDataEntry>,

    // the name entries only contain the internal functions,
    // and the value of 'index' is the 'function public index'.
    pub function_name_entries: Vec<FunctionNameEntry>,

    // the name entries only contain the internal data items,
    // and the value of 'index' is the 'data public index'.
    pub data_name_entries: Vec<DataNameEntry>,
}

// only application type module contains `Index` sections.
#[derive(Debug)]
pub struct IndexEntry {
    // essential
    pub entry_function_public_index: u32,

    // essential
    pub function_index_lists: Vec<FunctionIndexListEntry>,

    // optional
    pub data_index_lists: Vec<DataIndexListEntry>,

    // optional
    pub external_function_index_lists: Vec<ExternalFunctionIndexListEntry>,
    pub unified_external_library_entries: Vec<UnifiedExternalLibraryEntry>,
    pub unified_external_function_entries: Vec<UnifiedExternalFunctionEntry>,
}
