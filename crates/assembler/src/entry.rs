// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use anc_image::{
    entry::{
        DataNameEntry, ExternalFunctionEntry, ExternalLibraryEntry, FunctionEntry, FunctionNameEntry, ImportDataEntry, ImportFunctionEntry, ImportModuleEntry, InitedDataEntry, LocalVariableListEntry, RelocateListEntry, TypeEntry, UninitDataEntry
    },
    module_image::ImageType,
};

#[derive(Debug)]
pub struct ImageCommonEntry {
    // Note that this is the name of module/package,
    // it CANNOT be the name of submodule even if the current image is
    // a "object module", it also CANNOT be the full name or name path.
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    //
    // e.g.
    // the name path of function "add" in submodule "myapp:utils" is "utils::add",
    // and the full name is "myapp::utils::add"
    pub name: String,

    pub image_type: ImageType,

    // the dependencies
    pub import_module_entries: Vec<ImportModuleEntry>,

    // the following entries are used for linking:
    // - import_function_entries
    // - import_data_entries
    // - function_name_entries
    // - data_name_entries
    pub import_function_entries: Vec<ImportFunctionEntry>,
    pub import_data_entries: Vec<ImportDataEntry>,

    pub type_entries: Vec<TypeEntry>,
    pub local_variable_list_entries: Vec<LocalVariableListEntry>,
    pub function_entries: Vec<FunctionEntry>,

    pub read_only_data_entries: Vec<InitedDataEntry>,
    pub read_write_data_entries: Vec<InitedDataEntry>,
    pub uninit_data_entries: Vec<UninitDataEntry>,

    // the name path entries only contain the internal functions.
    pub function_name_entries: Vec<FunctionNameEntry>,

    // the name path entries only contain the internal data items.
    pub data_name_entries: Vec<DataNameEntry>,

    pub relocate_list_entries: Vec<RelocateListEntry>,

    // the dependencies
    pub external_library_entries: Vec<ExternalLibraryEntry>,
    pub external_function_entries: Vec<ExternalFunctionEntry>,
}

/*
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
 */

// #[derive(Debug, PartialEq)]
// pub struct CanonicalModuleNode {
//     pub name: String,
//
//     pub imports: Vec<ImportNode>,
//     pub externals: Vec<ExternalNode>,
//
//     pub datas: Vec<CanonicalDataNode>,
//     pub functions: Vec<CanonicalFunctionNode>,
// }
//
// #[derive(Debug, PartialEq)]
// pub struct CanonicalFunctionNode {
//     // the full name path
//     //
//     // e.g.
//     // the id of function 'add' in main module 'myapp' is 'myapp::add'
//     // the id of function 'add' in submodule 'myapp:utils' is 'myapp::utils::add'
//     pub fullname: String,
//
//     // the relative name path
//     //
//     // e.g.
//     // the name path of function 'add' in main module 'myapp' is 'add'
//     // the name path of function 'add' in submodule 'myapp:utils' is 'utils::add'
//     pub name_path: String,
//
//     pub export: bool,
//     pub params: Vec<NamedParameter>,
//     pub returns: Vec<OperandDataType>,
//     pub locals: Vec<LocalVariable>,
//     pub body: Box<ExpressionNode>,
// }
//
// #[derive(Debug, PartialEq)]
// pub struct CanonicalDataNode {
//     // the full name path
//     //
//     // e.g.
//     // the id of data 'buf' in main module 'myapp' is 'myapp::buf'
//     // the id of data 'buf' in submodule 'myapp:utils' is 'myapp::utils::buf'
//     pub fullname: String,
//
//     // the relative name path
//     //
//     // e.g.
//     // the name path of data 'buf' in main module 'myapp' is 'buf'
//     // the name path of data 'buf' in submodule 'myapp:utils' is 'utils::buf'
//     pub name_path: String,
//
//     pub export: bool,
//     pub data_section: DataSection,
// }
