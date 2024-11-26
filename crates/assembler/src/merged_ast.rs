// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

#[derive(Debug, PartialEq)]
pub struct MergedModuleNode {
    pub name: String,

    pub function_nodes: Vec<CanonicalFunctionNode>,
    pub read_only_data_nodes: Vec<CanonicalDataNode>,
    pub read_write_data_nodes: Vec<CanonicalDataNode>,
    pub uninit_data_nodes: Vec<CanonicalDataNode>,
}

#[derive(Debug, PartialEq)]
pub struct CanonicalFunctionNode {
    // the full name path
    //
    // e.g.
    // the id of function 'add' in main module 'myapp' is 'myapp::add'
    // the id of function 'add' in submodule 'myapp:utils' is 'myapp::utils::add'
    pub fullname: String,

    // the relative name path
    //
    // e.g.
    // the name path of function 'add' in main module 'myapp' is 'add'
    // the name path of function 'add' in submodule 'myapp:utils' is 'utils::add'
    pub name_path: String,

    pub export: bool,
    pub params: Vec<ParamNode>,
    pub results: Vec<DataType>,
    pub locals: Vec<LocalNode>,
    pub code: Vec<Instruction>,
}

#[derive(Debug, PartialEq)]
pub struct CanonicalDataNode {
    // the full name path, for data loading/storing instructions
    //
    // e.g.
    // the id of data 'buf' in main module 'myapp' is 'myapp::buf'
    // the id of data 'buf' in submodule 'myapp:utils' is 'myapp::utils::buf'
    pub id: String,

    // the canonicalize name path, which includes the submodule path, but
    // excludes the module name.
    //
    // e.g.
    // the name path of data 'buf' in main module 'myapp' is 'buf'
    // the name path of data 'buf' in submodule 'myapp:utils' is 'utils::buf'
    pub name_path: String,

    pub export: bool,
    pub data_kind: DataDetailNode,
}