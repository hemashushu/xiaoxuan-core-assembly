// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancasm_parser::ast::{
    DataDetailNode, DependItem, ExternalItem, ExternalNode, ImportItem, ImportNode,
};
use ancasm_parser::ast::{Instruction, LocalNode, ParamNode};
use ancvm_binary::bytecode_writer::BytecodeWriter;
use ancvm_types::entry::{
    DataNameEntry, ExternalFunctionEntry, ExternalLibraryEntry, FunctionEntry, FunctionNameEntry,
    ImportDataEntry, ImportFunctionEntry, ImportModuleEntry, InitedDataEntry, LocalListEntry,
    LocalVariableEntry, ModuleEntry, TypeEntry, UninitDataEntry,
};
use ancvm_types::DataSectionType;

use ancvm_types::{opcode::Opcode, DataType};

use crate::preprocessor::{CanonicalDataNode, CanonicalFunctionNode};
use crate::{preprocessor::MergedModuleNode, AssembleError, UNREACHABLE_CODE_NO_DEFAULT_ARM};

// the identifier of functions and datas
//
// the identifier is used for the function calling instructions, and
// the data loading/storing instructions.
//
// the 'path name' in the function name entry (data name entry,
// import function entry, import data entry) is different from the identifier,
// the 'path name' is used for linking.
struct IdentifierLookupTable {
    function_identifiers: Vec<IdentifierIndex>,
    data_identifiers: Vec<IdentifierIndex>,
    external_function_identifiers: Vec<IdentifierIndex>,
}

struct IdentifierIndex {
    id: String,

    // the function or data public index.
    //
    // the index of function item in public
    //
    // the function public index includes (and are sorted by the following order):
    // - the imported functions
    // - the internal functions
    //
    //
    // the data public index includes (and are sorted by the following order):
    //
    // - imported read-only data items
    // - internal read-only data items
    // - imported read-write data items
    // - internal read-write data items
    // - imported uninitilized data items
    // - internal uninitilized data items
    public_index: usize,
}

struct IdentifierSource {
    import_function_ids: Vec<String>,
    function_ids: Vec<String>,
    import_read_only_data_ids: Vec<String>,
    read_only_data_ids: Vec<String>,
    import_read_write_data_ids: Vec<String>,
    read_write_data_ids: Vec<String>,
    import_uninit_data_ids: Vec<String>,
    uninit_data_ids: Vec<String>,
    external_function_ids: Vec<String>,
}

impl IdentifierLookupTable {
    pub fn new(
        // function_name_entries: &[FunctionNameEntry],
        // data_name_entries: &[DataNameEntry],
        // external_nodes: &[ExternalNode],
        identifier_source: IdentifierSource,
    ) -> Self {
        let mut function_identifiers: Vec<IdentifierIndex> = vec![];
        let mut data_identifiers: Vec<IdentifierIndex> = vec![];
        let mut external_function_identifiers: Vec<IdentifierIndex> = vec![];

        let mut function_public_index_offset: usize = 0;
        let mut data_public_index_offset: usize = 0;

        // fill function ids

        function_identifiers.extend(
            identifier_source
                .import_function_ids
                .iter()
                .enumerate()
                .map(|(idx, id)| IdentifierIndex {
                    id: id.to_owned(),
                    public_index: function_public_index_offset + idx,
                }),
        );

        function_public_index_offset += identifier_source.import_function_ids.len();

        function_identifiers.extend(identifier_source.function_ids.iter().enumerate().map(
            |(idx, id)| IdentifierIndex {
                id: id.to_owned(),
                public_index: function_public_index_offset + idx,
            },
        ));

        // fill read-only data ids

        data_identifiers.extend(
            identifier_source
                .import_read_only_data_ids
                .iter()
                .enumerate()
                .map(|(idx, id)| IdentifierIndex {
                    id: id.to_owned(),
                    public_index: data_public_index_offset + idx,
                }),
        );

        data_public_index_offset += identifier_source.import_read_only_data_ids.len();

        data_identifiers.extend(identifier_source.read_only_data_ids.iter().enumerate().map(
            |(idx, id)| IdentifierIndex {
                id: id.to_owned(),
                public_index: data_public_index_offset + idx,
            },
        ));

        data_public_index_offset += identifier_source.read_only_data_ids.len();

        // fill read-write data ids

        data_identifiers.extend(
            identifier_source
                .import_read_write_data_ids
                .iter()
                .enumerate()
                .map(|(idx, id)| IdentifierIndex {
                    id: id.to_owned(),
                    public_index: data_public_index_offset + idx,
                }),
        );

        data_public_index_offset += identifier_source.import_read_write_data_ids.len();

        data_identifiers.extend(
            identifier_source
                .read_write_data_ids
                .iter()
                .enumerate()
                .map(|(idx, id)| IdentifierIndex {
                    id: id.to_owned(),
                    public_index: data_public_index_offset + idx,
                }),
        );

        data_public_index_offset += identifier_source.read_write_data_ids.len();

        // fill uninit data ids

        data_identifiers.extend(
            identifier_source
                .import_uninit_data_ids
                .iter()
                .enumerate()
                .map(|(idx, id)| IdentifierIndex {
                    id: id.to_owned(),
                    public_index: data_public_index_offset + idx,
                }),
        );

        data_public_index_offset += identifier_source.import_uninit_data_ids.len();

        data_identifiers.extend(identifier_source.uninit_data_ids.iter().enumerate().map(
            |(idx, id)| IdentifierIndex {
                id: id.to_owned(),
                public_index: data_public_index_offset + idx,
            },
        ));

        // external function ids

        external_function_identifiers.extend(
            identifier_source
                .external_function_ids
                .iter()
                .enumerate()
                .map(|(idx, id)| IdentifierIndex {
                    id: id.to_owned(),
                    public_index: idx,
                }),
        );

        // complete

        Self {
            function_identifiers,
            data_identifiers,
            external_function_identifiers,
        }
    }

    pub fn get_function_public_index(&self, identifier: &str) -> Result<usize, AssembleError> {
        match self
            .function_identifiers
            .iter()
            .find(|entry| entry.id == identifier)
        {
            Some(ii) => Ok(ii.public_index),
            None => Err(AssembleError::new(&format!(
                "Can not find the function: {}",
                identifier
            ))),
        }
    }

    pub fn get_data_public_index(&self, identifier: &str) -> Result<usize, AssembleError> {
        match self
            .data_identifiers
            .iter()
            .find(|entry| entry.id == identifier)
        {
            Some(ii) => Ok(ii.public_index),
            None => Err(AssembleError::new(&format!(
                "Can not find the data: {}",
                identifier
            ))),
        }
    }

    pub fn get_external_function_index(&self, identifier: &str) -> Result<usize, AssembleError> {
        match self
            .external_function_identifiers
            .iter()
            .find(|entry| entry.id == identifier)
        {
            Some(ii) => Ok(ii.public_index),
            None => Err(AssembleError::new(&format!(
                "Can not find the external function: {}",
                identifier
            ))),
        }
    }
}

// the stack for the control flows of a function.
// used to stub out instructions such as 'block', 'block?' and 'break'.
//
// - call FlowStack::push() when entering a block
//   (includes instruction 'block', 'block_nez', 'blocl_alt')
// - call FlowStack::add_break() when encounting instruction 'break'
// - call FlowStack::pop() when leaving a block
//   i.e. the instruction 'end', and fill all stubs.
//
// note that instruction 'recur' doesn't need to stub,
// because in the XiaoXuan Core Assembly, it only exists in
// the direct or indirect child nodes of node 'block', and
// the address of the 'block' is known at compile time.
struct FlowStack {
    items: Vec<FlowStackItem>,
}

struct FlowStackItem {
    // the address of instruction
    addr: usize,
    flow_kind: FlowKind,

    // all 'break' instructions which require a stub to be filled when
    // the current control flow reach the instruction 'end'.
    break_items: Vec<BreakItem>,
    local_names: Vec<String>,
}

#[derive(Debug, PartialEq)]
enum FlowKind {
    Function,

    // for structure: 'branch', 'for'
    // bytecode:
    // (opcode:u16 padding:u16 type_index:i32, local_list_index:i32)
    Block,

    // for structure: 'when', 'case'
    //
    // bytecode:
    // (opcode:u16 padding:u16 local_list_index:u32 next_inst_offset:u32)
    //
    // stub:
    // - next_inst_offset:u32
    BlockNez,

    // for structure: 'if'
    //
    // bytecode:
    // (opcode:u16 padding:u16 type_index:u32 local_list_index:u32 alt_inst_offset:u32)
    //
    // stub:
    // - alt_inst_offset:u32
    BlockAlt,
}

// bytecode:
// (opcode:u16 param reversed_index:u16 next_inst_offset:i32)
//
// stub:
// - next_inst_offset:i32
struct BreakItem {
    // the address of instruction 'break'
    addr: usize,
}

impl FlowStack {
    pub fn new() -> Self {
        Self { items: vec![] }
    }

    pub fn push(&mut self, addr: usize, flow_kind: FlowKind, local_names: Vec<String>) {
        let stub_item = FlowStackItem {
            addr,
            flow_kind,
            break_items: vec![],
            local_names,
        };
        self.items.push(stub_item);
    }

    pub fn pop(&mut self) -> FlowStackItem {
        self.items.pop().unwrap()
    }

    pub fn pop_and_fill_stubs(
        &mut self,
        bytecode_writer: &mut BytecodeWriter,
        addr_of_next_to_end: usize,
    ) {
        // pop flow stack
        let flow_item = self.pop();

        // fill stubs of 'block_nez'
        match flow_item.flow_kind {
            FlowKind::BlockNez => {
                let addr_of_block = flow_item.addr;
                let next_inst_offset = (addr_of_next_to_end - addr_of_block) as u32;
                bytecode_writer.fill_block_nez_stub(addr_of_block, next_inst_offset);
            }
            _ => {
                // only inst 'block_nez' has stub 'next_inst_offset'.
            }
        }

        // fill stubs of insts 'break'
        for break_item in &flow_item.break_items {
            let addr_of_break = break_item.addr;
            let next_inst_offset = (addr_of_next_to_end - addr_of_break) as u32;
            bytecode_writer.fill_break_stub(break_item.addr, next_inst_offset);
        }
    }

    pub fn add_break(&mut self, addr: usize, reversed_index: usize) {
        let flow_item = self.get_flow_item_by_reversed_index(reversed_index);

        if flow_item.flow_kind == FlowKind::Function {
            // the instruction 'break' does not need to stub when the
            // target is a function.
            // because the param 'next_inst_offset' of 'break' is ignored
            // (where can be set to '0') when the target block is the function itself.
        } else {
            flow_item.break_items.push(BreakItem { addr });
        }
    }

    fn get_flow_item_by_reversed_index(&mut self, reversed_index: usize) -> &mut FlowStackItem {
        let idx = self.items.len() - reversed_index - 1;
        &mut self.items[idx]
    }

    pub fn get_block_addr(&self, reversed_index: usize) -> usize {
        let idx = self.items.len() - reversed_index - 1;
        self.items[idx].addr
    }

    pub fn get_reversed_index_to_function(&self) -> usize {
        self.items.len() - 1
    }

    pub fn get_reversed_index_to_nearest_block(&self) -> usize {
        let idx = self
            .items
            .iter()
            .rposition(|item| item.flow_kind == FlowKind::Block)
            .expect("Can't find \"block\" on the control flow stack.");
        self.items.len() - idx - 1
    }

    // return (reversed_index, variable_index)
    //
    // within a function, all local variables, including the parameters
    // of function and all parameters and local varialbes within all blocks,
    // must not have duplicate names in the valid scope. e.g.
    //
    // ```text
    // {
    //     let abc = 0
    //     {
    //         let abc = 1     // invalid
    //         let xyz = 2     // valid
    //     }
    //
    //     {
    //         let xyz = 3     // valid
    //     }
    // }
    // ```

    pub fn get_local_variable_reversed_index_and_variable_index(
        &self,
        local_variable_name: &str,
    ) -> Result<(usize, usize), AssembleError> {
        let mut result: Option<(usize, usize)> = None;

        for (level_index, flow_item) in self.items.iter().enumerate() {
            if let Some(variable_index) = flow_item
                .local_names
                .iter()
                .position(|name| name == local_variable_name)
            {
                let reversed_index = self.items.len() - level_index - 1;

                if result.is_none() {
                    result.replace((reversed_index, variable_index));
                } else {
                    return Err(AssembleError::new(&format!(
                        "Local variable with duplicate name found: {}",
                        local_variable_name
                    )));
                }
            }
        }

        if let Some(val) = result {
            Ok(val)
        } else {
            Err(AssembleError::new(&format!(
                "Can not find the local variable: {}",
                local_variable_name
            )))
        }
    }
}

// note:
// the entry function 'entry' of an application should be
// inserted before assemble, as well as the auto-generated
// functions '_start' and '_exit'.
pub fn assemble_merged_module_node(
    merged_module_node: &MergedModuleNode,
) -> Result<ModuleEntry, AssembleError> {
    let module_name = merged_module_node.name.clone();
    // let runtime_version_major = merged_module_node.runtime_version_major;
    // let runtime_version_minor = merged_module_node.runtime_version_minor;
    let compiler_version = merged_module_node.compiler_version;

    let mut type_entries = vec![];

    let AssembleResultForDependItems {
        import_module_entries,
        import_module_ids,
        external_library_entries,
        external_library_ids,
    } = assemble_depend_items(&merged_module_node.depend_items)?;

    let AssembleResultForImportNodes {
        // import_module_entries,
        import_function_entries,
        import_data_entries,
        import_function_ids,
        import_read_only_data_ids,
        import_read_write_data_ids,
        import_uninit_data_ids,
    } = assemble_import_nodes(
        &import_module_ids,
        &merged_module_node.import_nodes,
        &mut type_entries,
    )?;

    let (
        // external_library_entries,
        external_function_entries,
        external_function_ids,
    ) = assemble_external_nodes(
        &external_library_ids,
        &merged_module_node.external_nodes,
        &mut type_entries,
    )?;

    let import_function_count = import_function_ids.len();
    let import_read_only_data_count = import_read_only_data_ids.len();
    let import_read_write_data_count = import_read_write_data_ids.len();
    let import_uninit_data_count = import_uninit_data_ids.len();

    let (function_name_entries, function_ids) =
        build_function_name_entries(&merged_module_node.function_nodes, import_function_count);

    let BuildDataNameEntryResult {
        data_name_entries,
        read_only_data_ids,
        read_write_data_ids,
        uninit_data_ids,
    } = build_data_name_entries(
        &merged_module_node.read_only_data_nodes,
        &merged_module_node.read_write_data_nodes,
        &merged_module_node.uninit_data_nodes,
        import_read_only_data_count,
        import_read_write_data_count,
        import_uninit_data_count,
    );

    let identifier_lookup_table = IdentifierLookupTable::new(IdentifierSource {
        import_function_ids,
        function_ids,
        import_read_only_data_ids,
        read_only_data_ids,
        import_read_write_data_ids,
        read_write_data_ids,
        import_uninit_data_ids,
        uninit_data_ids,
        external_function_ids,
    });

    let (local_list_entries, function_entries) = assemble_function_nodes(
        &merged_module_node.function_nodes,
        &mut type_entries,
        &identifier_lookup_table,
    )?;

    let (read_only_data_entries, read_write_data_entries, uninit_data_entries) =
        assemble_data_nodes(
            &merged_module_node.read_only_data_nodes,
            &merged_module_node.read_write_data_nodes,
            &merged_module_node.uninit_data_nodes,
        )?;

    // find the public index of constructor and destructor
    let constructor_function_public_index =
        if let Some(function_name) = &merged_module_node.constructor_function_name_path {
            let entry_opt = function_name_entries
                .iter()
                .find(|entry| &entry.name_path == function_name);

            if let Some(entry) = entry_opt {
                Some(entry.function_public_index as u32)
            } else {
                return Err(AssembleError {
                    message: format!(
                        "Can not find the constructor function \"{}\" in module \"{}\".",
                        function_name, module_name
                    ),
                });
            }
        } else {
            None
        };

    let destructor_function_public_index =
        if let Some(function_name) = &merged_module_node.destructor_function_name_path {
            let entry_opt = function_name_entries
                .iter()
                .find(|entry| &entry.name_path == function_name);

            if let Some(entry) = entry_opt {
                Some(entry.function_public_index as u32)
            } else {
                return Err(AssembleError {
                    message: format!(
                        "Can not find the destructor function \"{}\" in module \"{}\".",
                        function_name, module_name
                    ),
                });
            }
        } else {
            None
        };

    let module_entry = ModuleEntry {
        name: module_name,
        runtime_version: compiler_version,
        // runtime_version_major,
        // runtime_version_minor,
        //
        import_function_count,
        import_read_only_data_count,
        import_read_write_data_count,
        import_uninit_data_count,
        //
        constructor_function_public_index,
        destructor_function_public_index,
        //
        type_entries,
        local_list_entries,
        function_entries,
        read_only_data_entries,
        read_write_data_entries,
        uninit_data_entries,
        //
        import_module_entries,
        import_function_entries,
        import_data_entries,
        //
        external_library_entries,
        external_function_entries,
        //
        function_name_entries,
        data_name_entries,
    };

    Ok(module_entry)
}

// fn count_imported_function(merged_module_node: &MergedModuleNode) -> usize {
//     let mut count: usize = 0;
//     for import_node in &merged_module_node.import_nodes {
//         for import_item in &import_node.import_items {
//             if let ImportItem::ImportFunction(_) = import_item {
//                 count += 1;
//             }
//         }
//     }
//     count
// }
//
// fn count_imported_data(merged_module_node: &MergedModuleNode) -> (usize, usize, usize) {
//     let mut count_ro: usize = 0;
//     let mut count_rw: usize = 0;
//     let mut count_uninit: usize = 0;
//     for import_node in &merged_module_node.import_nodes {
//         for import_item in &import_node.import_items {
//             if let ImportItem::ImportData(import_data) = import_item {
//                 match import_data.data_kind_node {
//                     SimplifiedDataKindNode::ReadOnly(_) => count_ro += 1,
//                     SimplifiedDataKindNode::ReadWrite(_) => count_rw += 1,
//                     SimplifiedDataKindNode::Uninit(_) => count_uninit += 1,
//                 }
//             }
//         }
//     }
//     (count_ro, count_rw, count_uninit)
// }

fn build_function_name_entries(
    function_nodes: &[CanonicalFunctionNode],
    import_function_count: usize,
) -> (Vec<FunctionNameEntry>, Vec<String>) {
    let mut function_name_entries = vec![];
    let mut function_public_index = import_function_count;
    let mut function_ids: Vec<String> = vec![];

    for function_node in function_nodes {
        // add function id
        function_ids.push(function_node.id.clone());

        // add function name entry
        let function_name_entry = FunctionNameEntry {
            name_path: function_node.name_path.clone(),
            function_public_index,
            export: function_node.export,
        };

        function_name_entries.push(function_name_entry);
        function_public_index += 1;
    }

    (function_name_entries, function_ids)
}

struct BuildDataNameEntryResult {
    data_name_entries: Vec<DataNameEntry>,
    read_only_data_ids: Vec<String>,
    read_write_data_ids: Vec<String>,
    uninit_data_ids: Vec<String>,
}

fn build_data_name_entries(
    read_only_data_nodes: &[CanonicalDataNode],
    read_write_data_nodes: &[CanonicalDataNode],
    uninit_data_nodes: &[CanonicalDataNode],
    import_read_only_data_count: usize,
    import_read_write_data_count: usize,
    import_uninit_data_count: usize,
) -> BuildDataNameEntryResult {
    let mut data_name_entries = vec![];
    let mut data_public_index = 0;

    let mut read_only_data_ids: Vec<String> = vec![];
    let mut read_write_data_ids: Vec<String> = vec![];
    let mut uninit_data_ids: Vec<String> = vec![];

    data_public_index += import_read_only_data_count;

    for read_only_data_node in read_only_data_nodes {
        // add data id
        read_only_data_ids.push(read_only_data_node.id.clone());

        // add data name entry
        let data_name_entry = DataNameEntry {
            name_path: read_only_data_node.name_path.clone(),
            data_public_index,
            export: read_only_data_node.export,
        };
        data_name_entries.push(data_name_entry);
        data_public_index += 1;
    }

    data_public_index += import_read_write_data_count;

    for read_write_data_node in read_write_data_nodes {
        // add data id
        read_write_data_ids.push(read_write_data_node.id.clone());

        // add data name entry
        let data_name_entry = DataNameEntry {
            name_path: read_write_data_node.name_path.clone(),
            data_public_index,
            export: read_write_data_node.export,
        };
        data_name_entries.push(data_name_entry);
        data_public_index += 1;
    }

    data_public_index += import_uninit_data_count;

    for uninit_data_node in uninit_data_nodes {
        // add data id
        uninit_data_ids.push(uninit_data_node.id.clone());

        // add data name entry
        let data_name_entry = DataNameEntry {
            name_path: uninit_data_node.name_path.clone(),
            data_public_index,
            export: uninit_data_node.export,
        };
        data_name_entries.push(data_name_entry);
        data_public_index += 1;
    }

    BuildDataNameEntryResult {
        data_name_entries,
        read_only_data_ids,
        read_write_data_ids,
        uninit_data_ids,
    }
}

type AssembleResultForFuncNode = (Vec<LocalListEntry>, Vec<FunctionEntry>);

fn assemble_function_nodes(
    function_nodes: &[CanonicalFunctionNode],
    type_entries: &mut Vec<TypeEntry>,
    identifier_lookup_table: &IdentifierLookupTable,
) -> Result<AssembleResultForFuncNode, AssembleError> {
    let mut local_list_entries = vec![];
    let mut function_entries = vec![];

    for function_node in function_nodes {
        let type_index = find_existing_type_index_with_creating_when_not_found_by_param_nodes(
            type_entries,
            &function_node.params,
            &function_node.results,
        );

        let local_list_index = find_existing_local_index_with_creating_when_not_found(
            &mut local_list_entries,
            &function_node.params,
            &function_node.locals,
        );

        let local_names =
            get_local_names_with_params_and_locals(&function_node.params, &function_node.locals);

        let code = assemble_function_code(
            &function_node.name_path,
            local_names,
            &function_node.code,
            identifier_lookup_table,
            type_entries,
            &mut local_list_entries,
            // &mut flow_stack,
        )?;

        function_entries.push(FunctionEntry {
            type_index,
            local_list_index,
            code,
        });
    }

    Ok((local_list_entries, function_entries))
}

fn find_existing_type_index_with_creating_when_not_found_by_param_nodes(
    type_entries: &mut Vec<TypeEntry>,
    param_nodes: &[ParamNode],
    results: &[DataType],
) -> usize {
    let params = param_nodes
        .iter()
        .map(|node| node.data_type)
        .collect::<Vec<_>>();
    find_existing_type_index_with_creating_when_not_found(type_entries, &params, results)
}

// type = params + results
// local = params + local vars
fn find_existing_type_index_with_creating_when_not_found(
    type_entries: &mut Vec<TypeEntry>,
    params: &[DataType],
    results: &[DataType],
) -> usize {
    let opt_idx = type_entries
        .iter()
        .position(|entry| entry.params == params && entry.results == results);

    if let Some(idx) = opt_idx {
        idx
    } else {
        let idx = type_entries.len();
        type_entries.push(TypeEntry {
            params: params.to_vec(),
            results: results.to_vec(),
        });
        idx
    }
}

fn find_existing_local_index_with_creating_when_not_found(
    local_list_entries: &mut Vec<LocalListEntry>,
    param_nodes: &[ParamNode],
    local_nodes: &[LocalNode],
) -> usize {
    let variable_entries_from_params = param_nodes
        .iter()
        .map(|node| LocalVariableEntry::from_datatype(node.data_type))
        .collect::<Vec<_>>();

    let variable_entries_from_local = local_nodes
        .iter()
        .map(|node| LocalVariableEntry {
            memory_data_type: node.memory_data_type,
            length: node.data_length,
            align: node.align,
        })
        .collect::<Vec<_>>();

    let mut variable_entries = vec![];
    variable_entries.extend_from_slice(&variable_entries_from_params);
    variable_entries.extend_from_slice(&variable_entries_from_local);

    let opt_idx = local_list_entries
        .iter()
        .position(|entry| entry.local_variable_entries == variable_entries);

    if let Some(idx) = opt_idx {
        idx
    } else {
        let idx = local_list_entries.len();
        local_list_entries.push(LocalListEntry::new(variable_entries));
        idx
    }
}

fn get_local_names_with_params_and_locals(
    param_nodes: &[ParamNode],
    local_nodes: &[LocalNode],
) -> Vec<String> {
    let names_from_params = param_nodes
        .iter()
        .map(|node| node.name.clone())
        .collect::<Vec<_>>();

    let names_from_locals = local_nodes
        .iter()
        .map(|node| node.name.clone())
        .collect::<Vec<_>>();

    let mut names = vec![];
    names.extend_from_slice(&names_from_params);
    names.extend_from_slice(&names_from_locals);

    names
}

fn assemble_function_code(
    function_name: &str,
    local_names: Vec<String>,
    instructions: &[Instruction],
    identifier_lookup_table: &IdentifierLookupTable,
    type_entries: &mut Vec<TypeEntry>,
    local_list_entries: &mut Vec<LocalListEntry>,
    // flow_stack: &mut FlowStack,
) -> Result<Vec<u8>, AssembleError> {
    let mut bytecode_writer = BytecodeWriter::new();

    // push flow stack
    let mut flow_stack = FlowStack::new();
    flow_stack.push(0, FlowKind::Function, local_names);

    for instruction in instructions {
        assemble_instruction(
            instruction,
            identifier_lookup_table,
            type_entries,
            local_list_entries,
            &mut flow_stack,
            &mut bytecode_writer,
        )?;
    }

    // write the implied instruction 'end'
    bytecode_writer.write_opcode(Opcode::end);

    // pop flow stack
    flow_stack.pop();

    // check control flow stack
    if !flow_stack.items.is_empty() {
        return Err(AssembleError::new(&format!(
            "Control flow does not end in the function \"{}\"",
            function_name
        )));
    }

    Ok(bytecode_writer.to_bytes())
}

fn assemble_instruction(
    instruction: &Instruction,
    identifier_lookup_table: &IdentifierLookupTable,
    type_entries: &mut Vec<TypeEntry>,
    local_list_entries: &mut Vec<LocalListEntry>,
    flow_stack: &mut FlowStack,
    bytecode_writer: &mut BytecodeWriter,
) -> Result<(), AssembleError> {
    match instruction {
        Instruction::NoParams { opcode, operands } => assemble_instruction_kind_no_params(
            opcode,
            operands,
            identifier_lookup_table,
            type_entries,
            local_list_entries,
            flow_stack,
            bytecode_writer,
        )?,
        Instruction::ImmI32(value) => {
            bytecode_writer.write_opcode_i32(Opcode::i32_imm, *value);
        }
        Instruction::ImmI64(value) => {
            bytecode_writer.write_opcode_pesudo_i64(Opcode::i64_imm, *value);
        }
        Instruction::ImmF32(value) => {
            bytecode_writer.write_opcode_pesudo_f32(Opcode::f32_imm, *value);
        }
        Instruction::ImmF64(value) => {
            bytecode_writer.write_opcode_pesudo_f64(Opcode::f64_imm, *value);
        }
        Instruction::LocalLoad {
            opcode,
            name,
            offset,
        } => {
            let (reversed_index, variable_index) =
                flow_stack.get_local_variable_reversed_index_and_variable_index(name)?;

            // bytecode: (param reversed_index:i16 offset_bytes:i16 local_variable_index:i16)
            bytecode_writer.write_opcode_i16_i16_i16(
                *opcode,
                reversed_index as u16,
                *offset,
                variable_index as u16,
            );
        }
        Instruction::LocalStore {
            opcode,
            name,
            offset,
            value,
        } => {
            assemble_instruction(
                value,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            let (reversed_index, variable_index) =
                flow_stack.get_local_variable_reversed_index_and_variable_index(name)?;

            // bytecode: (param reversed_index:i16 offset_bytes:i16 local_variable_index:i16)
            bytecode_writer.write_opcode_i16_i16_i16(
                *opcode,
                reversed_index as u16,
                *offset,
                variable_index as u16,
            );
        }
        Instruction::LocalLongLoad {
            opcode,
            name,
            offset,
        } => {
            assemble_instruction(
                offset,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            let (reversed_index, variable_index) =
                flow_stack.get_local_variable_reversed_index_and_variable_index(name)?;

            // bytecode: (param reversed_index:i16 local_variable_index:i32)
            bytecode_writer.write_opcode_i16_i32(
                *opcode,
                reversed_index as u16,
                variable_index as u32,
            );
        }
        Instruction::LocalLongStore {
            opcode,
            name,
            offset,
            value,
        } => {
            // assemble 'offset' first, then 'value'
            assemble_instruction(
                offset,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            assemble_instruction(
                value,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            let (reversed_index, variable_index) =
                flow_stack.get_local_variable_reversed_index_and_variable_index(name)?;

            // bytecode: (param reversed_index:i16 local_variable_index:i32)
            bytecode_writer.write_opcode_i16_i32(
                *opcode,
                reversed_index as u16,
                variable_index as u32,
            );
        }
        Instruction::DataLoad {
            opcode,
            id: name,
            offset,
        } => {
            let data_public_index = identifier_lookup_table.get_data_public_index(name)?;

            // bytecode: (param offset_bytes:i16 data_public_index:i32)
            bytecode_writer.write_opcode_i16_i32(*opcode, *offset, data_public_index as u32);
        }
        Instruction::DataStore {
            opcode,
            id: name,
            offset,
            value,
        } => {
            assemble_instruction(
                value,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            let data_public_index = identifier_lookup_table.get_data_public_index(name)?;

            // bytecode: (param offset_bytes:i16 data_public_index:i32)
            bytecode_writer.write_opcode_i16_i32(*opcode, *offset, data_public_index as u32);
        }
        Instruction::DataLongLoad {
            opcode,
            id: name,
            offset,
        } => {
            assemble_instruction(
                offset,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            let data_public_index = identifier_lookup_table.get_data_public_index(name)?;

            // bytecode: (param data_public_index:i32)
            bytecode_writer.write_opcode_i32(*opcode, data_public_index as u32);
        }
        Instruction::DataLongStore {
            opcode,
            id: name,
            offset,
            value,
        } => {
            assemble_instruction(
                offset,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            assemble_instruction(
                value,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            let data_public_index = identifier_lookup_table.get_data_public_index(name)?;

            // bytecode: (param data_public_index:i32)
            bytecode_writer.write_opcode_i32(*opcode, data_public_index as u32);
        }
        Instruction::HeapLoad {
            opcode,
            offset,
            addr,
        } => {
            assemble_instruction(
                addr,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            // bytecode: (param offset_bytes:i16)
            bytecode_writer.write_opcode_i16(*opcode, *offset);
        }
        Instruction::HeapStore {
            opcode,
            offset,
            addr,
            value,
        } => {
            assemble_instruction(
                addr,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            assemble_instruction(
                value,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            // bytecode: (param offset_bytes:i16)
            bytecode_writer.write_opcode_i16(*opcode, *offset);
        }
        Instruction::UnaryOp {
            opcode,
            source: number,
        } => {
            assemble_instruction(
                number,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            bytecode_writer.write_opcode(*opcode);
        }
        Instruction::UnaryOpWithImmI16 {
            opcode,
            imm: amount,
            source: number,
        } => {
            assemble_instruction(
                number,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            bytecode_writer.write_opcode_i16(*opcode, *amount);
        }
        Instruction::BinaryOp {
            opcode,
            left,
            right,
        } => {
            // assemble 'left' first, then 'right'
            assemble_instruction(
                left,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            assemble_instruction(
                right,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            bytecode_writer.write_opcode(*opcode);
        }
        Instruction::When {
            // locals,
            test,
            consequent,
        } => {
            // | structure         | assembly          | instruction(s)     |
            // |-------------------|-------------------|--------------------|
            // |                   |                   | ..a..              |
            // | if ..a.. {        | (when (a)         | block_nez -\       |
            // |    ..b..          |       (b)         |   ..b..    |       |
            // | }                 | )                 | end        |       |
            // |                   |                   | ...    <---/       |
            // |-------------------|-------------------|--------------------|

            // bytecode:
            // - block_nez (param local_list_index:i32, next_inst_offset:i32)

            // assemble node 'test'
            assemble_instruction(
                test,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            // local index and names
            let local_list_index = find_existing_local_index_with_creating_when_not_found(
                local_list_entries,
                &[],
                &[],
            );

            // write inst 'block_nez'
            let addr_of_block_nez = bytecode_writer.write_opcode_i32_i32(
                Opcode::block_nez,
                local_list_index as u32,
                0, // stub for 'next_inst_offset'
            );

            // push flow stack
            flow_stack.push(addr_of_block_nez, FlowKind::BlockNez, vec![]);

            // assemble node 'consequent'
            assemble_instruction(
                consequent,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            // write inst 'end'
            bytecode_writer.write_opcode(Opcode::end);
            let addr_of_next_to_end = bytecode_writer.get_addr();

            // pop flow stck and fill stubs
            flow_stack.pop_and_fill_stubs(bytecode_writer, addr_of_next_to_end);
        }
        Instruction::If {
            results,
            // locals,
            test,
            consequent,
            alternate,
        } => {
            // | structure         | assembly          | instruction(s)     |
            // |-------------------|-------------------|--------------------|
            // |                   |                   | ..a..              |
            // | if ..a.. {        | (if (a)           | block_alt ---\     |
            // |    ..b..          |     (b)           |   ..b..      |     |
            // | } else {          |     (c)           |   break 0  --|-\   |
            // |    ..c..          | )                 |   ..c..  <---/ |   |
            // | }                 |                   | end            |   |
            // |                   |                   | ...      <-----/   |
            // |-------------------|-------------------|--------------------|
            // |                   |                   | ..a..              |
            // | if ..a.. {        | (if (a)           | block_alt ---\     |
            // |    ..b..          |     (b)           |   ..b..      |     |
            // | } else if ..c.. { |     (if (c)       |   break 0 ---|---\ |
            // |    ..d..          |         (d)       |   ..c..  <---/   | |
            // | } else {          |         (e)       |   block_alt --\  | |
            // |    ..e..          |     )             |     ..d..     |  | |
            // | }                 | )                 |     break 0 --|-\| |
            // |                   |                   |     ..e..  <--/ || |
            // |                   |                   |   end           || |
            // |                   |                   | end        <----/| |
            // |                   |                   | ...        <-----/ |
            // |                   |                   |                    |
            // |                   | ----------------- | ------------------ |

            // bytecode:
            // - block_alt (param type_index:i32, local_list_index:i32, alt_inst_offset:i32)
            // - break (param reversed_index:i16, next_inst_offset:i32)

            // assemble node 'test'
            assemble_instruction(
                test,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            // type index
            let type_index =
                find_existing_type_index_with_creating_when_not_found(type_entries, &[], results);

            // local index
            let local_list_index = find_existing_local_index_with_creating_when_not_found(
                local_list_entries,
                &[],
                &[],
            );

            // write inst 'block_alt'
            let addr_of_block_alt = bytecode_writer.write_opcode_i32_i32_i32(
                Opcode::block_alt,
                type_index as u32,
                local_list_index as u32,
                0, // stub for 'alt_inst_offset'
            );

            // push flow stack
            flow_stack.push(addr_of_block_alt, FlowKind::BlockAlt, vec![]);

            // assemble node 'consequent'
            assemble_instruction(
                consequent,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            // write inst 'break'
            let addr_of_break = bytecode_writer.write_opcode_i16_i32(
                Opcode::break_,
                0, // reversed_index
                0, // next_inst_offset
            );
            let addr_of_next_to_break = bytecode_writer.get_addr();
            let alt_inst_offset = (addr_of_next_to_break - addr_of_block_alt) as u32;

            // fill the stub of inst 'block_alt'
            bytecode_writer.fill_block_alt_stub(addr_of_block_alt, alt_inst_offset);

            // add break item
            flow_stack.add_break(addr_of_break, 0);

            // assemble node 'alternate'
            assemble_instruction(
                alternate,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            // write inst 'end'
            bytecode_writer.write_opcode(Opcode::end);
            let addr_of_next_to_end = bytecode_writer.get_addr();

            // pop flow stack and fill stubs
            flow_stack.pop_and_fill_stubs(bytecode_writer, addr_of_next_to_end);
        }
        Instruction::Branch {
            results,
            // locals,
            cases,
            default,
        } => {
            // | structure         | assembly          | instruction(s)     |
            // |-------------------|-------------------|--------------------|
            // |                   |                   |                    |
            // |                   | (branch           | block              |
            // |                   |   (case (a) (b))  |   ..a..            |
            // |                   |   (case (c) (d))  |   block_nez -\     |
            // |                   |   (default (e))   |     ..b..    |     |
            // |                   | )                 |     break 1 -|--\  |
            // |                   |                   |   end        |  |  |
            // |                   |                   |   ..c..  <---/  |  |
            // |                   |                   |   block_nez -\  |  |
            // |                   |                   |     ..d..    |  |  |
            // |                   |                   |     break 1 -|--|  |
            // |                   |                   |   end        |  |  |
            // |                   |                   |   ..e..  <---/  |  |
            // |                   |                   | end             |  |
            // |                   |                   | ...        <----/  |
            // |-------------------|-------------------|--------------------|

            // bytecode:
            // - block (param type_index:i32, local_list_index:i32)
            // - block_nez (param local_list_index:i32, next_inst_offset:i32)
            // - break (param reversed_index:i16, next_inst_offset:i32)

            // type index
            let type_index =
                find_existing_type_index_with_creating_when_not_found(type_entries, &[], results);

            // local index and names
            let local_list_index = find_existing_local_index_with_creating_when_not_found(
                local_list_entries,
                &[],
                &[],
            );

            // write inst 'block'
            let addr_of_block = bytecode_writer.write_opcode_i32_i32(
                Opcode::block,
                type_index as u32,
                local_list_index as u32,
            );

            // push flow stack
            flow_stack.push(addr_of_block, FlowKind::Block, vec![]);

            // write branches
            for case in cases {
                // assemble node 'test'
                assemble_instruction(
                    &case.test,
                    identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;

                // local index and names
                let case_local_list_index = find_existing_local_index_with_creating_when_not_found(
                    local_list_entries,
                    &[],
                    &[],
                );

                // write inst 'block_nez'
                let addr_of_block_nez = bytecode_writer.write_opcode_i32_i32(
                    Opcode::block_nez,
                    case_local_list_index as u32,
                    0, // stub for 'next_inst_offset'
                );

                // push flow stack
                flow_stack.push(addr_of_block_nez, FlowKind::BlockNez, vec![]);

                // assemble node 'consequent'
                assemble_instruction(
                    &case.consequent,
                    identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;

                // write inst 'break 1'

                let addr_of_break = bytecode_writer.write_opcode_i16_i32(
                    Opcode::break_,
                    1,
                    0, // stub for 'next_inst_offset'
                );

                // add 'break' item to control flow stack
                flow_stack.add_break(addr_of_break, 1);

                // write inst 'end'
                bytecode_writer.write_opcode(Opcode::end);
                let addr_of_next_to_end = bytecode_writer.get_addr();

                // pop flow stack and fill stubs
                flow_stack.pop_and_fill_stubs(bytecode_writer, addr_of_next_to_end);
            }

            // write node 'default'
            if let Some(default_instruction) = default {
                // assemble node 'consequent'
                assemble_instruction(
                    default_instruction,
                    identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            } else {
                // write the inst 'unreachable'
                bytecode_writer
                    .write_opcode_i32(Opcode::unreachable, UNREACHABLE_CODE_NO_DEFAULT_ARM);
            }

            // write inst 'end'
            bytecode_writer.write_opcode(Opcode::end);
            let addr_of_next_to_end = bytecode_writer.get_addr();

            // pop flow stack and fill stubs
            flow_stack.pop_and_fill_stubs(bytecode_writer, addr_of_next_to_end);
        }
        Instruction::For {
            params,
            results,
            locals,
            code,
        } => {
            // | structure         | assembly          | instructions(s)    |
            // |-------------------|-------------------|--------------------|
            // | loop {            | (for (code        | block              |
            // |    ...            |   ...             |   ...   <--\       |
            // | }                 |   (recur ...)     |   recur 0 -/       |
            // |                   | ))                | end                |
            // |-------------------|-------------------|--------------------|
            // |                   |                   |                    |
            // |                   | (for (code        | block              |
            // |                   |   (when (a)       |   ..a..    <---\   |
            // |                   |     (code ...     |   block_nez    |   |
            // |                   |       (recur ...) |     ...        |   |
            // |                   |     )             |     recur 1 ---/   |
            // |                   |   )               |   end              |
            // |                   | ))                | end                |
            // |                   |                   |                    |
            // |                   |                   |                    |
            // |-------------------|-------------------|--------------------|
            // |                   |                   |                    |
            // |                   | (for (code        | block              |
            // |                   |   ...             |   ...      <---\   |
            // |                   |   (when (a)       |   ..a..        |   |
            // |                   |     (recur ...)   |   block_nez    |   |
            // |                   |   )               |     recur 1 ---/   |
            // |                   | ))                |   end              |
            // |                   |                   | end                |
            // |                   |                   |                    |
            // |                   |                   |                    |
            // |-------------------|-------------------|--------------------|

            // bytecode:
            // - block (param type_index:i32, local_list_index:i32)
            // - recur (param reversed_index:i16, start_inst_offset:i32)

            // type index
            let type_index = find_existing_type_index_with_creating_when_not_found_by_param_nodes(
                type_entries,
                params,
                results,
            );

            // local index
            let local_list_index = find_existing_local_index_with_creating_when_not_found(
                local_list_entries,
                params,
                locals,
            );

            // local names
            let local_names = get_local_names_with_params_and_locals(params, locals);

            // write inst 'block'
            let addr_of_block = bytecode_writer.write_opcode_i32_i32(
                Opcode::block,
                type_index as u32,
                local_list_index as u32,
            );

            // push flow stack
            flow_stack.push(addr_of_block, FlowKind::Block, local_names);

            // assemble node 'consequent'
            assemble_instruction(
                code,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            // write inst 'end'
            bytecode_writer.write_opcode(Opcode::end);
            let addr_of_next_to_end = bytecode_writer.get_addr();

            // pop flow stack and fill stubs
            flow_stack.pop_and_fill_stubs(bytecode_writer, addr_of_next_to_end);
        }
        Instruction::Do(instructions) => {
            for instruction in instructions {
                assemble_instruction(
                    instruction,
                    identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            }
        }
        Instruction::Break(instructions) => {
            // note that the statement 'break' is not the same as the instruction 'break',
            // the statement 'break' only break the nearest instruction 'block'.

            for instruction in instructions {
                assemble_instruction(
                    instruction,
                    identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            }

            let reversed_index = flow_stack.get_reversed_index_to_nearest_block();

            // write inst 'break'
            let addr_of_break = bytecode_writer.write_opcode_i16_i32(
                Opcode::break_,
                reversed_index as u16,
                0, // stub for 'next_inst_offset'
            );

            flow_stack.add_break(addr_of_break, reversed_index);
        }
        Instruction::Recur(instructions) => {
            // note that the statement 'recur' is not the same as the instruction 'recur',
            // the statement 'recur' only recur to the nearest instruction 'block'.

            for instruction in instructions {
                assemble_instruction(
                    instruction,
                    identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            }

            let reversed_index = flow_stack.get_reversed_index_to_nearest_block();

            // write inst 'recur'
            let addr_of_recur = bytecode_writer.write_opcode_i16_i32(
                Opcode::recur,
                reversed_index as u16,
                0, // stub for 'start_inst_offset'
            );

            let addr_of_block = flow_stack.get_block_addr(reversed_index);

            // the length of inst 'block' is 12 bytes
            let addr_of_next_to_block = addr_of_block + 12;
            let start_inst_offset = (addr_of_recur - addr_of_next_to_block) as u32;
            bytecode_writer.fill_recur_stub(addr_of_recur, start_inst_offset);
        }
        Instruction::Return(instructions) => {
            // break to the function
            for instruction in instructions {
                assemble_instruction(
                    instruction,
                    identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            }

            let reversed_index = flow_stack.get_reversed_index_to_function();

            // write inst 'break'
            bytecode_writer.write_opcode_i16_i32(
                Opcode::break_,
                reversed_index as u16,
                0, // 'next_inst_offset' is ignored when the target is the function
            );
        }
        Instruction::FnRecur(instructions) => {
            // recur to the function

            for instruction in instructions {
                assemble_instruction(
                    instruction,
                    identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            }

            let reversed_index = flow_stack.get_reversed_index_to_function();

            // write inst 'recur'
            bytecode_writer.write_opcode_i16_i32(
                Opcode::recur,
                reversed_index as u16,
                0, // 'start_inst_offset' is ignored when the target is function
            );
        }
        Instruction::Call { id, args } => {
            for instruction in args {
                assemble_instruction(
                    instruction,
                    identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            }

            let function_public_index = identifier_lookup_table.get_function_public_index(id)?;
            bytecode_writer.write_opcode_i32(Opcode::call, function_public_index as u32);
        }
        Instruction::DynCall {
            public_index: num,
            args,
        } => {
            for instruction in args {
                assemble_instruction(
                    instruction,
                    identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            }

            // assemble the function public index operand
            assemble_instruction(
                num,
                identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            bytecode_writer.write_opcode(Opcode::dyncall);
        }
        Instruction::EnvCall { num, args } => {
            for instruction in args {
                assemble_instruction(
                    instruction,
                    identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            }

            bytecode_writer.write_opcode_i32(Opcode::envcall, *num);
        }
        Instruction::SysCall { num, args } => {
            for instruction in args {
                assemble_instruction(
                    instruction,
                    identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            }

            bytecode_writer.write_opcode_i32(Opcode::i32_imm, *num);
            bytecode_writer.write_opcode_i32(Opcode::i32_imm, args.len() as u32);
            bytecode_writer.write_opcode(Opcode::syscall);
        }
        Instruction::ExtCall { id, args } => {
            for instruction in args {
                assemble_instruction(
                    instruction,
                    identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            }

            let external_function_idx = identifier_lookup_table.get_external_function_index(id)?;
            bytecode_writer.write_opcode_i32(Opcode::extcall, external_function_idx as u32);
        }
        // macro
        Instruction::MacroGetFunctionPublicIndex { id } => {
            let function_public_index = identifier_lookup_table.get_function_public_index(id)?;
            bytecode_writer.write_opcode_i32(Opcode::i32_imm, function_public_index as u32);
        }
        Instruction::Debug { code } => {
            bytecode_writer.write_opcode_i32(Opcode::debug, *code);
        }
        Instruction::Unreachable { code } => {
            bytecode_writer.write_opcode_i32(Opcode::unreachable, *code);
        }
        Instruction::HostAddrFunction { id } => {
            let function_public_index = identifier_lookup_table.get_function_public_index(id)?;
            bytecode_writer
                .write_opcode_i32(Opcode::host_addr_function, function_public_index as u32);
        }
    }

    Ok(())
}

fn assemble_instruction_kind_no_params(
    opcode: &Opcode,
    operands: &[Instruction],
    identifier_lookup_table: &IdentifierLookupTable,
    type_entries: &mut Vec<TypeEntry>,
    local_list_entries: &mut Vec<LocalListEntry>,
    flow_stack: &mut FlowStack,
    bytecode_writer: &mut BytecodeWriter,
) -> Result<(), AssembleError> {
    for instruction in operands {
        assemble_instruction(
            instruction,
            identifier_lookup_table,
            type_entries,
            local_list_entries,
            flow_stack,
            bytecode_writer,
        )?;
    }

    bytecode_writer.write_opcode(*opcode);

    Ok(())
}

type AssembleResultForDataNodes = (
    Vec<InitedDataEntry>,
    Vec<InitedDataEntry>,
    Vec<UninitDataEntry>,
);

fn assemble_data_nodes(
    read_only_data_nodes: &[CanonicalDataNode],
    read_write_data_nodes: &[CanonicalDataNode],
    uninit_data_nodes: &[CanonicalDataNode],
) -> Result<AssembleResultForDataNodes, AssembleError> {
    let read_only_data_entries = read_only_data_nodes
        .iter()
        .map(|node| match &node.data_kind {
            DataDetailNode::ReadOnly(src) => InitedDataEntry {
                memory_data_type: src.memory_data_type,
                data: src.value.clone(),
                length: src.length,
                align: src.align,
            },
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();

    let read_write_data_entries = read_write_data_nodes
        .iter()
        .map(|node| match &node.data_kind {
            DataDetailNode::ReadWrite(src) => InitedDataEntry {
                memory_data_type: src.memory_data_type,
                data: src.value.clone(),
                length: src.length,
                align: src.align,
            },
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();

    let uninit_data_entries = uninit_data_nodes
        .iter()
        .map(|node| match &node.data_kind {
            DataDetailNode::Uninit(src) => UninitDataEntry {
                memory_data_type: src.memory_data_type,
                length: src.length,
                align: src.align,
            },
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();

    Ok((
        read_only_data_entries,
        read_write_data_entries,
        uninit_data_entries,
    ))
}

struct AssembleResultForDependItems {
    import_module_entries: Vec<ImportModuleEntry>,
    import_module_ids: Vec<String>,
    external_library_entries: Vec<ExternalLibraryEntry>,
    external_library_ids: Vec<String>,
}

fn assemble_depend_items(
    depend_items: &[DependItem],
) -> Result<AssembleResultForDependItems, AssembleError> {
    let mut import_module_entries: Vec<ImportModuleEntry> = vec![];
    let mut import_module_ids: Vec<String> = vec![];
    let mut external_library_entries: Vec<ExternalLibraryEntry> = vec![];
    let mut external_library_ids: Vec<String> = vec![];

    // check duplicate items?

    for item in depend_items {
        match item {
            DependItem::DependentModule(module) => {
                let import_module_entry = ImportModuleEntry {
                    name: module.name.clone(),
                    module_share_type: module.module_share_type,
                    module_version: module.module_version,
                };
                import_module_entries.push(import_module_entry);
                import_module_ids.push(module.id.clone());
            }
            DependItem::DependentLibrary(library) => {
                let external_library_entry = ExternalLibraryEntry {
                    name: library.name.clone(),
                    external_library_type: library.external_library_type,
                };
                external_library_entries.push(external_library_entry);
                external_library_ids.push(library.id.clone());
            }
        }
    }

    Ok(AssembleResultForDependItems {
        import_module_entries,
        import_module_ids,
        external_library_entries,
        external_library_ids,
    })
}

struct AssembleResultForImportNodes {
    // import_module_entries: Vec<ImportModuleEntry>,
    import_function_entries: Vec<ImportFunctionEntry>,
    import_data_entries: Vec<ImportDataEntry>,
    import_function_ids: Vec<String>,
    import_read_only_data_ids: Vec<String>,
    import_read_write_data_ids: Vec<String>,
    import_uninit_data_ids: Vec<String>,
}

fn assemble_import_nodes(
    import_module_ids: &[String],
    import_nodes: &[ImportNode],
    type_entries: &mut Vec<TypeEntry>,
) -> Result<AssembleResultForImportNodes, AssembleError> {
    let mut import_function_entries: Vec<ImportFunctionEntry> = vec![];
    let mut import_data_entries: Vec<ImportDataEntry> = vec![];

    let mut import_function_ids: Vec<String> = vec![];
    let mut import_read_only_data_ids: Vec<String> = vec![];
    let mut import_read_write_data_ids: Vec<String> = vec![];
    let mut import_uninit_data_ids: Vec<String> = vec![];

    //    for (import_module_index, import_node) in import_nodes.iter().enumerate() {
    //         let import_module_node = &import_node.import_module_node;
    //
    //         // add import module entry
    //         let import_module_entry = ImportModuleEntry::new(
    //             import_module_node.name.clone(),
    //             import_module_node.module_share_type,
    //             import_module_node.version_major,
    //             import_module_node.version_minor,
    //         );
    //         import_module_entries.push(import_module_entry);

    for import_node in import_nodes {
        let import_module_index = import_module_ids
            .iter()
            .position(|id| id == &import_node.module_id)
            .unwrap();

        for import_item in &import_node.import_items {
            match import_item {
                ImportItem::ImportFunction(import_function_node) => {
                    // add function id
                    import_function_ids.push(import_function_node.id.clone());

                    // get type index
                    let type_index = find_existing_type_index_with_creating_when_not_found(
                        type_entries,
                        &import_function_node.params,
                        &import_function_node.results,
                    );

                    // add import function entry
                    let import_function_entry = ImportFunctionEntry::new(
                        import_function_node.name_path.clone(),
                        import_module_index,
                        type_index,
                    );
                    import_function_entries.push(import_function_entry);
                }
                ImportItem::ImportData(import_data_node) => {
                    // add data id
                    match import_data_node.data_section_type {
                        DataSectionType::ReadOnly => {
                            import_read_only_data_ids.push(import_data_node.id.clone());
                        }
                        DataSectionType::ReadWrite => {
                            import_read_write_data_ids.push(import_data_node.id.clone());
                        }
                        DataSectionType::Uninit => {
                            import_uninit_data_ids.push(import_data_node.id.clone());
                        }
                    };

                    // add import data entry
                    let import_data_entry = ImportDataEntry {
                        name_path: import_data_node.name_path.clone(),
                        import_module_index,
                        data_section_type: import_data_node.data_section_type,
                        memory_data_type: import_data_node.memory_data_type,
                    };
                    import_data_entries.push(import_data_entry);
                }
            }
        }
    }

    let result = AssembleResultForImportNodes {
        // import_module_entries,
        import_function_entries,
        import_data_entries,
        import_function_ids,
        import_read_only_data_ids,
        import_read_write_data_ids,
        import_uninit_data_ids,
    };

    Ok(result)
}

type AssembleResultForExternNode = (
    // Vec<ExternalLibraryEntry>,
    Vec<ExternalFunctionEntry>,
    Vec<String>,
);

fn assemble_external_nodes(
    external_library_ids: &[String],
    external_nodes: &[ExternalNode],
    type_entries: &mut Vec<TypeEntry>,
) -> Result<AssembleResultForExternNode, AssembleError> {
    // let mut external_library_entries: Vec<ExternalLibraryEntry> = vec![];
    let mut external_function_entries: Vec<ExternalFunctionEntry> = vec![];
    let mut external_function_ids: Vec<String> = vec![];

    for external_node in external_nodes {
        // // build ExternalLibraryEntry
        // let external_library_node = &external_node.external_library_node;
        // let external_library_entry = ExternalLibraryEntry {
        //     name: external_library_node.name.clone(),
        //     external_library_type: external_library_node.external_library_type,
        // };
        // let external_library_index = external_library_entries.len();
        // external_library_entries.push(external_library_entry);

        let external_library_index = external_library_ids
            .iter()
            .position(|id| id == &external_node.library_id)
            .unwrap();

        for external_item in &external_node.external_items {
            let ExternalItem::ExternalFunction(external_function) = external_item;

            // add external function id
            external_function_ids.push(external_function.id.clone());

            // get type index
            let type_index = find_existing_type_index_with_creating_when_not_found(
                type_entries,
                &external_function.params,
                &external_function.results,
            );

            // build ExternalFunctionEntry
            let external_function_entry = ExternalFunctionEntry {
                name: external_function.name.clone(),
                external_library_index,
                type_index,
            };

            external_function_entries.push(external_function_entry);
        }
    }

    Ok((
        // external_library_entries,
        external_function_entries,
        external_function_ids,
    ))
}

#[cfg(test)]
mod tests {
    use ancvm_binary::bytecode_reader::format_bytecode_as_text;
    use ancvm_types::{
        entry::{
            DataNameEntry, ExternalFunctionEntry, ExternalLibraryEntry, FunctionNameEntry,
            ImportDataEntry, ImportFunctionEntry, ImportModuleEntry, InitedDataEntry,
            LocalListEntry, LocalVariableEntry, TypeEntry,
        },
        DataSectionType, DataType, EffectiveVersion, ExternalLibraryType, MemoryDataType,
        ModuleShareType,
    };
    use pretty_assertions::assert_eq;

    use ancasm_parser::{
        lexer::{filter, lex},
        parser::parse,
        peekable_iterator::PeekableIterator,
    };

    use crate::{
        assembler::assemble_merged_module_node,
        preprocessor::merge_and_canonicalize_submodule_nodes,
    };

    #[test]
    fn test_assemble() {
        let submodule_sources = &[
            r#"
        (module $myapp
            (compiler_version "1.0")
            (constructor $init)
            (destructor $exit)
            (depend
                (module $math share "math" "1.0")
                (library $libc system "libc.so.6")
            )
            (data $SUCCESS (read_only i64 0))
            (data $FAILURE (read_only i64 1))
            (function $entry
                (result i64)
                (code
                    (call $module::utils::add
                        (extcall $module::utils::getuid)
                        (data.load32_i32 $module::utils::seed)
                    )
                    (data.load64_i64 $SUCCESS)
                )
            )
            (function $init
                (code
                    (data.store32 $module::utils::buf (i32.imm 0))
                )
            )
            (function $exit
                (code
                    nop
                )
            )
        )
        "#,
            r#"
        (module $myapp::utils
            (import $math
                (function $wrap_add "wrap::add"
                    (params i32 i32)
                    (result i32)
                )
                (data $seed "seed" read_only i32)
            )
            (external $libc
                (function $getuid "getuid" (result i32))
            )
            (data $buf (read_write bytes h"11131719" 2))
            (function export $add
                (param $left i32) (param $right i32)
                (result i64)
                (code
                    (call $wrap_add
                        (local.load32_i32 $left)
                        (local.load32_i32 $right)
                    )
                )
            )
        )
        "#,
        ];

        let submodule_nodes = submodule_sources
            .iter()
            .map(|source| {
                let mut chars = source.chars();
                let mut char_iter = PeekableIterator::new(&mut chars, 3);
                let all_tokens = lex(&mut char_iter).unwrap();
                let effective_tokens = filter(&all_tokens);
                let mut token_iter = effective_tokens.into_iter();
                let mut peekable_token_iter = PeekableIterator::new(&mut token_iter, 2);
                parse(&mut peekable_token_iter, None).unwrap()
            })
            .collect::<Vec<_>>();

        let merged_module_node =
            merge_and_canonicalize_submodule_nodes(&submodule_nodes, None, None).unwrap();
        let module_entry = assemble_merged_module_node(&merged_module_node).unwrap();

        assert_eq!(module_entry.name, "myapp");
        // assert_eq!(module_entry.runtime_version_major, 1);
        // assert_eq!(module_entry.runtime_version_minor, 0);
        assert_eq!(module_entry.runtime_version, EffectiveVersion::new(1, 0));

        assert_eq!(module_entry.import_function_count, 1);
        assert_eq!(module_entry.import_read_only_data_count, 1);
        assert_eq!(module_entry.import_read_write_data_count, 0);
        assert_eq!(module_entry.import_uninit_data_count, 0);

        assert_eq!(module_entry.constructor_function_public_index, Some(2));
        assert_eq!(module_entry.destructor_function_public_index, Some(3));

        // check import entries

        assert_eq!(
            module_entry.import_module_entries,
            vec![ImportModuleEntry {
                name: "math".to_owned(),
                module_share_type: ModuleShareType::Share,
                // version_major: 1,
                // version_minor: 0
                module_version: EffectiveVersion::new(1, 0)
            }]
        );

        assert_eq!(
            module_entry.import_function_entries,
            vec![ImportFunctionEntry {
                name_path: "wrap::add".to_owned(),
                import_module_index: 0,
                type_index: 0
            }]
        );

        assert_eq!(
            module_entry.import_data_entries,
            vec![ImportDataEntry {
                name_path: "seed".to_owned(),
                import_module_index: 0,
                data_section_type: DataSectionType::ReadOnly,
                memory_data_type: MemoryDataType::I32
            }]
        );

        // check external entries

        assert_eq!(
            module_entry.external_library_entries,
            vec![ExternalLibraryEntry {
                name: "libc.so.6".to_owned(),
                external_library_type: ExternalLibraryType::System
            }]
        );

        assert_eq!(
            module_entry.external_function_entries,
            vec![ExternalFunctionEntry {
                name: "getuid".to_owned(),
                external_library_index: 0,
                type_index: 1
            }]
        );

        // check function entries
        assert_eq!(module_entry.function_entries.len(), 4);

        let function_entry0 = &module_entry.function_entries[0];
        assert_eq!(function_entry0.type_index, 2);
        assert_eq!(function_entry0.local_list_index, 0);
        assert_eq!(
            format_bytecode_as_text(&function_entry0.code),
            "\
0x0000  04 0b 00 00  00 00 00 00    extcall           idx:0
0x0008  02 03 00 00  00 00 00 00    data.load32_i32   off:0x00  idx:0
0x0010  00 0b 00 00  04 00 00 00    call              idx:4
0x0018  00 03 00 00  01 00 00 00    data.load64_i64   off:0x00  idx:1
0x0020  00 0a                       end"
        );

        let function_entry1 = &module_entry.function_entries[1];
        assert_eq!(function_entry1.type_index, 3);
        assert_eq!(function_entry1.local_list_index, 0);
        assert_eq!(
            format_bytecode_as_text(&function_entry1.code),
            "\
0x0000  80 01 00 00  00 00 00 00    i32.imm           0x00000000
0x0008  09 03 00 00  03 00 00 00    data.store32      off:0x00  idx:3
0x0010  00 0a                       end"
        );

        let function_entry2 = &module_entry.function_entries[2];
        assert_eq!(function_entry2.type_index, 3);
        assert_eq!(function_entry2.local_list_index, 0);
        assert_eq!(
            format_bytecode_as_text(&function_entry2.code),
            "\
0x0000  00 01                       nop
0x0002  00 0a                       end"
        );

        let function_entry3 = &module_entry.function_entries[3];
        assert_eq!(function_entry3.type_index, 4);
        assert_eq!(function_entry3.local_list_index, 1);
        assert_eq!(
            format_bytecode_as_text(&function_entry3.code),
            "\
0x0000  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x0008  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x0010  00 0b 00 00  00 00 00 00    call              idx:0
0x0018  00 0a                       end"
        );

        // check data entries

        assert_eq!(
            module_entry.read_only_data_entries,
            vec![
                InitedDataEntry {
                    memory_data_type: MemoryDataType::I64,
                    data: 0u64.to_le_bytes().to_vec(),
                    length: 8,
                    align: 8
                },
                InitedDataEntry {
                    memory_data_type: MemoryDataType::I64,
                    data: 1u64.to_le_bytes().to_vec(),
                    length: 8,
                    align: 8
                },
            ]
        );

        assert_eq!(
            module_entry.read_write_data_entries,
            vec![InitedDataEntry {
                memory_data_type: MemoryDataType::Bytes,
                data: vec![0x11u8, 0x13, 0x17, 0x19],
                length: 4,
                align: 2
            },]
        );

        assert_eq!(module_entry.uninit_data_entries.len(), 0);

        // check type entries

        assert_eq!(
            module_entry.type_entries,
            vec![
                TypeEntry {
                    params: vec![DataType::I32, DataType::I32],
                    results: vec![DataType::I32]
                },
                TypeEntry {
                    params: vec![],
                    results: vec![DataType::I32]
                },
                TypeEntry {
                    params: vec![],
                    results: vec![DataType::I64]
                },
                TypeEntry {
                    params: vec![],
                    results: vec![]
                },
                TypeEntry {
                    params: vec![DataType::I32, DataType::I32],
                    results: vec![DataType::I64]
                },
            ]
        );

        // check local list entries

        assert_eq!(
            module_entry.local_list_entries,
            vec![
                LocalListEntry {
                    local_variable_entries: vec![]
                },
                LocalListEntry {
                    local_variable_entries: vec![
                        LocalVariableEntry {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4
                        },
                        LocalVariableEntry {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4
                        }
                    ]
                }
            ]
        );

        // check function names

        assert_eq!(
            module_entry.function_name_entries,
            vec![
                FunctionNameEntry {
                    name_path: "entry".to_owned(),
                    function_public_index: 1,
                    export: false
                },
                FunctionNameEntry {
                    name_path: "init".to_owned(),
                    function_public_index: 2,
                    export: false
                },
                FunctionNameEntry {
                    name_path: "exit".to_owned(),
                    function_public_index: 3,
                    export: false
                },
                FunctionNameEntry {
                    name_path: "utils::add".to_owned(),
                    function_public_index: 4,
                    export: true
                },
            ]
        );

        // check data names

        assert_eq!(
            module_entry.data_name_entries,
            vec![
                DataNameEntry {
                    name_path: "SUCCESS".to_owned(),
                    data_public_index: 1,
                    export: false
                },
                DataNameEntry {
                    name_path: "FAILURE".to_owned(),
                    data_public_index: 2,
                    export: false
                },
                DataNameEntry {
                    name_path: "utils::buf".to_owned(),
                    data_public_index: 3,
                    export: false
                }
            ]
        )
    }
}
