// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_binary::{
    bytecode_writer::BytecodeWriter,
    module_image::{
        data_name_section::DataNameEntry,
        data_section::{InitedDataEntry, UninitDataEntry},
        external_func_section::ExternalFuncEntry,
        external_library_section::ExternalLibraryEntry,
        func_name_section::FuncNameEntry,
        func_section::FuncEntry,
        local_variable_section::{LocalListEntry, LocalVariableEntry},
        type_section::TypeEntry,
    },
};
use ancvm_parser::ast::{DataKindNode, DataNode, ExternNode, ExternalItem};
use ancvm_parser::ast::{FuncNode, ImmF32, ImmF64, Instruction, LocalNode, ParamNode};
use ancvm_types::{opcode::Opcode, DataType};

use crate::{
    preprocessor::MergedModuleNode, AssembleError, ModuleEntry, UNREACHABLE_CODE_NO_DEFAULT_ARM,
};

// the identifier of functions and datas
struct SymbolIdentifierLookupTable {
    func_identifiers: Vec<IdentifierIndex>,
    data_identifiers: Vec<IdentifierIndex>,
    external_func_identifiers: Vec<IdentifierIndex>,
}

struct IdentifierIndex {
    identifier: String,
    public_index: usize,
}

impl SymbolIdentifierLookupTable {
    pub fn new(
        func_name_entries: &[FuncNameEntry],
        data_name_entries: &[DataNameEntry],
        extern_nodes: &[ExternNode],
    ) -> Self {
        let func_identifiers = func_name_entries
            .iter()
            .map(|entry| IdentifierIndex {
                identifier: entry.name.clone(),
                public_index: entry.func_pub_index,
            })
            .collect::<Vec<_>>();

        let data_identifiers = data_name_entries
            .iter()
            .map(|entry| IdentifierIndex {
                identifier: entry.name.clone(),
                public_index: entry.data_pub_index,
            })
            .collect::<Vec<_>>();

        let external_func_identifiers =
            SymbolIdentifierLookupTable::build_external_function_identifier_indices(extern_nodes);

        Self {
            func_identifiers,
            data_identifiers,
            external_func_identifiers,
        }
    }

    fn build_external_function_identifier_indices(
        extern_nodes: &[ExternNode],
    ) -> Vec<IdentifierIndex> {
        let mut external_func_identifiers: Vec<IdentifierIndex> = vec![];

        let mut idx: usize = 0;

        for extern_node in extern_nodes {
            for external_item in &extern_node.external_items {
                let ExternalItem::ExternalFunc(external_func) = external_item;
                external_func_identifiers.push(IdentifierIndex {
                    identifier: external_func.name.clone(),
                    public_index: idx,
                });
                idx += 1;
            }
        }

        external_func_identifiers
    }

    pub fn get_func_pub_index(&self, identifier: &str) -> Result<usize, AssembleError> {
        match self
            .func_identifiers
            .iter()
            .find(|entry| entry.identifier == identifier)
        {
            Some(ii) => Ok(ii.public_index),
            None => Err(AssembleError::new(&format!(
                "Can not find the function: {}",
                identifier
            ))),
        }
    }

    pub fn get_data_pub_index(&self, identifier: &str) -> Result<usize, AssembleError> {
        match self
            .data_identifiers
            .iter()
            .find(|entry| entry.identifier == identifier)
        {
            Some(ii) => Ok(ii.public_index),
            None => Err(AssembleError::new(&format!(
                "Can not find the data: {}",
                identifier
            ))),
        }
    }

    pub fn get_external_func_index(&self, identifier: &str) -> Result<usize, AssembleError> {
        match self
            .external_func_identifiers
            .iter()
            .find(|entry| entry.identifier == identifier)
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

pub fn assemble_merged_module_node(
    merged_module_node: &MergedModuleNode,
) -> Result<ModuleEntry, AssembleError> {
    let name = merged_module_node.name.clone();
    let runtime_version_major = merged_module_node.runtime_version_major;
    let runtime_version_minor = merged_module_node.runtime_version_minor;

    let imported_func_count = 0;
    let imported_ro_data_count = 0;
    let imported_rw_data_count = 0;
    let imported_uninit_data_count = 0;

    let func_name_entries =
        build_func_name_entries(&merged_module_node.func_nodes, imported_func_count);
    let data_name_entries = build_data_name_entries(
        &merged_module_node.read_only_data_nodes,
        &merged_module_node.read_write_data_nodes,
        &merged_module_node.uninit_data_nodes,
        imported_ro_data_count,
        imported_rw_data_count,
        imported_uninit_data_count,
    );

    let symbol_identifier_lookup_table = SymbolIdentifierLookupTable::new(
        &func_name_entries,
        &data_name_entries,
        &merged_module_node.extern_nodes,
    );

    let (mut type_entries, local_list_entries, func_entries) = assemble_func_nodes(
        &merged_module_node.func_nodes,
        &symbol_identifier_lookup_table,
    )?;

    let (read_only_data_entries, read_write_data_entries, uninit_data_entries) =
        assemble_data_nodes(
            &merged_module_node.read_only_data_nodes,
            &merged_module_node.read_write_data_nodes,
            &merged_module_node.uninit_data_nodes,
        )?;

    let (external_library_entries, external_func_entries) =
        assemble_extern_nodes(&merged_module_node.extern_nodes, &mut type_entries)?;

    let module_entry = ModuleEntry {
        name,
        runtime_version_major,
        runtime_version_minor,
        //
        type_entries,
        local_list_entries,
        func_entries,
        read_only_data_entries,
        read_write_data_entries,
        uninit_data_entries,
        //
        external_library_entries,
        external_func_entries,
        //
        func_name_entries,
        data_name_entries,
    };

    Ok(module_entry)
}

fn build_func_name_entries(
    func_nodes: &[FuncNode],
    imported_func_count: usize,
) -> Vec<FuncNameEntry> {
    let mut func_name_entries = vec![];
    let mut func_pub_idx = imported_func_count;

    for func_node in func_nodes {
        let entry = FuncNameEntry {
            name: func_node.name.clone(),
            func_pub_index: func_pub_idx,
            exported: func_node.exported,
        };

        func_name_entries.push(entry);
        func_pub_idx += 1;
    }

    func_name_entries
}

fn build_data_name_entries(
    ro_data_nodes: &[DataNode],
    rw_data_nodes: &[DataNode],
    uninit_data_nodes: &[DataNode],
    imported_ro_data_count: usize,
    imported_rw_data_count: usize,
    imported_uninit_data_count: usize,
) -> Vec<DataNameEntry> {
    let mut data_name_entries = vec![];
    let mut data_pub_idx = 0;

    data_pub_idx += imported_ro_data_count;

    for data_node in ro_data_nodes {
        let entry = DataNameEntry {
            name: data_node.name.clone(),
            data_pub_index: data_pub_idx,
            exported: data_node.exported,
        };
        data_name_entries.push(entry);
        data_pub_idx += 1;
    }

    data_pub_idx += imported_rw_data_count;

    for data_node in rw_data_nodes {
        let entry = DataNameEntry {
            name: data_node.name.clone(),
            data_pub_index: data_pub_idx,
            exported: data_node.exported,
        };
        data_name_entries.push(entry);
        data_pub_idx += 1;
    }

    data_pub_idx += imported_uninit_data_count;

    for data_node in uninit_data_nodes {
        let entry = DataNameEntry {
            name: data_node.name.clone(),
            data_pub_index: data_pub_idx,
            exported: data_node.exported,
        };
        data_name_entries.push(entry);
        data_pub_idx += 1;
    }

    data_name_entries
}

type AssembleResultForFuncNode = (Vec<TypeEntry>, Vec<LocalListEntry>, Vec<FuncEntry>);

fn assemble_func_nodes(
    func_nodes: &[FuncNode],
    symbol_identifier_lookup_table: &SymbolIdentifierLookupTable,
) -> Result<AssembleResultForFuncNode, AssembleError> {
    let mut type_entries = vec![];
    let mut local_list_entries = vec![];
    let mut func_entries = vec![];

    for func_node in func_nodes {
        let type_index = find_existing_type_index_with_creating_when_not_found_by_param_nodes(
            &mut type_entries,
            &func_node.params,
            &func_node.results,
        );

        let local_list_index = find_existing_local_index_with_creating_when_not_found(
            &mut local_list_entries,
            &func_node.params,
            &func_node.locals,
        );

        let local_names =
            get_local_names_with_params_and_locals(&func_node.params, &func_node.locals);

        let code = assemble_func_code(
            &func_node.name,
            local_names,
            &func_node.code,
            symbol_identifier_lookup_table,
            &mut type_entries,
            &mut local_list_entries,
            // &mut flow_stack,
        )?;

        func_entries.push(FuncEntry {
            type_index,
            local_list_index,
            code,
        });
    }

    Ok((type_entries, local_list_entries, func_entries))
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
        .position(|entry| entry.variable_entries == variable_entries);

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

fn assemble_func_code(
    func_name: &str,
    local_names: Vec<String>,
    instructions: &[Instruction],
    symbol_identifier_lookup_table: &SymbolIdentifierLookupTable,
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
            symbol_identifier_lookup_table,
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
            func_name
        )));
    }

    Ok(bytecode_writer.to_bytes())
}

fn assemble_instruction(
    instruction: &Instruction,
    symbol_identifier_lookup_table: &SymbolIdentifierLookupTable,
    type_entries: &mut Vec<TypeEntry>,
    local_list_entries: &mut Vec<LocalListEntry>,
    flow_stack: &mut FlowStack,
    bytecode_writer: &mut BytecodeWriter,
) -> Result<(), AssembleError> {
    match instruction {
        Instruction::NoParams { opcode, operands } => assemble_instruction_kind_no_params(
            opcode,
            operands,
            symbol_identifier_lookup_table,
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
        Instruction::ImmF32(imm_f32) => match imm_f32 {
            ImmF32::Float(value) => {
                bytecode_writer.write_opcode_pesudo_f32(Opcode::f32_imm, *value);
            }
            ImmF32::Hex(value) => {
                bytecode_writer.write_opcode_i32(Opcode::f32_imm, *value);
            }
        },
        Instruction::ImmF64(imm_f64) => match imm_f64 {
            ImmF64::Float(value) => {
                bytecode_writer.write_opcode_pesudo_f64(Opcode::f64_imm, *value);
            }
            ImmF64::Hex(value) => {
                bytecode_writer.write_opcode_pesudo_i64(Opcode::f64_imm, *value);
            }
        },
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
                symbol_identifier_lookup_table,
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
                symbol_identifier_lookup_table,
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
                symbol_identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            assemble_instruction(
                value,
                symbol_identifier_lookup_table,
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
            name_path: name,
            offset,
        } => {
            let data_pub_index = symbol_identifier_lookup_table.get_data_pub_index(name)?;

            // bytecode: (param offset_bytes:i16 data_public_index:i32)
            bytecode_writer.write_opcode_i16_i32(*opcode, *offset, data_pub_index as u32);
        }
        Instruction::DataStore {
            opcode,
            name_path: name,
            offset,
            value,
        } => {
            assemble_instruction(
                value,
                symbol_identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            let data_pub_index = symbol_identifier_lookup_table.get_data_pub_index(name)?;

            // bytecode: (param offset_bytes:i16 data_public_index:i32)
            bytecode_writer.write_opcode_i16_i32(*opcode, *offset, data_pub_index as u32);
        }
        Instruction::DataLongLoad {
            opcode,
            name_path: name,
            offset,
        } => {
            assemble_instruction(
                offset,
                symbol_identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            let data_pub_index = symbol_identifier_lookup_table.get_data_pub_index(name)?;

            // bytecode: (param data_public_index:i32)
            bytecode_writer.write_opcode_i32(*opcode, data_pub_index as u32);
        }
        Instruction::DataLongStore {
            opcode,
            name_path: name,
            offset,
            value,
        } => {
            assemble_instruction(
                offset,
                symbol_identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            assemble_instruction(
                value,
                symbol_identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            let data_pub_index = symbol_identifier_lookup_table.get_data_pub_index(name)?;

            // bytecode: (param data_public_index:i32)
            bytecode_writer.write_opcode_i32(*opcode, data_pub_index as u32);
        }
        Instruction::HeapLoad {
            opcode,
            offset,
            addr,
        } => {
            assemble_instruction(
                addr,
                symbol_identifier_lookup_table,
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
                symbol_identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            assemble_instruction(
                value,
                symbol_identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            // bytecode: (param offset_bytes:i16)
            bytecode_writer.write_opcode_i16(*opcode, *offset);
        }
        Instruction::UnaryOp { opcode, number } => {
            assemble_instruction(
                number,
                symbol_identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            bytecode_writer.write_opcode(*opcode);
        }
        Instruction::UnaryOpParamI16 {
            opcode,
            amount,
            number,
        } => {
            assemble_instruction(
                number,
                symbol_identifier_lookup_table,
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
                symbol_identifier_lookup_table,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            assemble_instruction(
                right,
                symbol_identifier_lookup_table,
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
                symbol_identifier_lookup_table,
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
                symbol_identifier_lookup_table,
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
                symbol_identifier_lookup_table,
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
                symbol_identifier_lookup_table,
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
                symbol_identifier_lookup_table,
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
                    symbol_identifier_lookup_table,
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
                    symbol_identifier_lookup_table,
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
                    symbol_identifier_lookup_table,
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
                symbol_identifier_lookup_table,
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
                    symbol_identifier_lookup_table,
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
                    symbol_identifier_lookup_table,
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
                    symbol_identifier_lookup_table,
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
                    symbol_identifier_lookup_table,
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
        Instruction::Rerun(instructions) => {
            // recur to the function

            for instruction in instructions {
                assemble_instruction(
                    instruction,
                    symbol_identifier_lookup_table,
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
        Instruction::Call {
            name_path: name,
            args,
        } => {
            for instruction in args {
                assemble_instruction(
                    instruction,
                    symbol_identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            }

            let func_pub_idx = symbol_identifier_lookup_table.get_func_pub_index(name)?;
            bytecode_writer.write_opcode_i32(Opcode::call, func_pub_idx as u32);
        }
        Instruction::DynCall { num, args } => {
            for instruction in args {
                assemble_instruction(
                    instruction,
                    symbol_identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            }

            // assemble the function public index operand
            assemble_instruction(
                num,
                symbol_identifier_lookup_table,
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
                    symbol_identifier_lookup_table,
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
                    symbol_identifier_lookup_table,
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
        Instruction::ExtCall {
            name,
            args,
        } => {
            for instruction in args {
                assemble_instruction(
                    instruction,
                    symbol_identifier_lookup_table,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            }

            let external_func_idx = symbol_identifier_lookup_table.get_external_func_index(name)?;
            bytecode_writer.write_opcode_i32(Opcode::extcall, external_func_idx as u32);
        }
        // macro
        Instruction::MacroGetFuncPubIndex(name_path) => {
            let func_pub_idx = symbol_identifier_lookup_table.get_func_pub_index(name_path)?;
            bytecode_writer.write_opcode_i32(Opcode::i32_imm, func_pub_idx as u32);
        }
        Instruction::Debug(code) => {
            bytecode_writer.write_opcode_i32(Opcode::debug, *code);
        }
        Instruction::Unreachable(code) => {
            bytecode_writer.write_opcode_i32(Opcode::unreachable, *code);
        }
        Instruction::HostAddrFunc(name) => {
            let func_pub_idx = symbol_identifier_lookup_table.get_func_pub_index(name)?;
            bytecode_writer.write_opcode_i32(Opcode::host_addr_func, func_pub_idx as u32);
        }
    }

    Ok(())
}

fn assemble_instruction_kind_no_params(
    opcode: &Opcode,
    operands: &[Instruction],
    symbol_identifier_lookup_table: &SymbolIdentifierLookupTable,
    type_entries: &mut Vec<TypeEntry>,
    local_list_entries: &mut Vec<LocalListEntry>,
    flow_stack: &mut FlowStack,
    bytecode_writer: &mut BytecodeWriter,
) -> Result<(), AssembleError> {
    for instruction in operands {
        assemble_instruction(
            instruction,
            symbol_identifier_lookup_table,
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
    read_only_data_nodes: &[DataNode],
    read_write_data_nodes: &[DataNode],
    uninit_data_nodes: &[DataNode],
) -> Result<AssembleResultForDataNodes, AssembleError> {
    let read_only_data_entries = read_only_data_nodes
        .iter()
        .map(|node| match &node.data_kind {
            DataKindNode::ReadOnly(src) => InitedDataEntry {
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
            DataKindNode::ReadWrite(src) => InitedDataEntry {
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
            DataKindNode::Uninit(src) => UninitDataEntry {
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

type AssembleResultForExternNode = (Vec<ExternalLibraryEntry>, Vec<ExternalFuncEntry>);

fn assemble_extern_nodes(
    extern_nodes: &[ExternNode],
    type_entries: &mut Vec<TypeEntry>,
) -> Result<AssembleResultForExternNode, AssembleError> {
    let mut external_library_entries: Vec<ExternalLibraryEntry> = vec![];
    let mut external_func_entries: Vec<ExternalFuncEntry> = vec![];

    for extern_node in extern_nodes {
        // build ExternalLibraryEntry
        let external_library_node = &extern_node.external_library_node;
        let external_library_entry = ExternalLibraryEntry {
            name: external_library_node.name.clone(),
            external_library_type: external_library_node.external_library_type,
        };
        let external_library_index = external_func_entries.len();
        external_library_entries.push(external_library_entry);

        for extern_item in &extern_node.external_items {
            let ExternalItem::ExternalFunc(external_func) = extern_item;

            // get type index
            let type_index = find_existing_type_index_with_creating_when_not_found(
                type_entries,
                &external_func.params,
                &external_func.results,
            );

            // build ExternalFuncEntry
            let external_func_entry = ExternalFuncEntry {
                name: external_func.symbol.clone(),
                external_library_index,
                type_index,
            };

            external_func_entries.push(external_func_entry);
        }
    }

    Ok((external_library_entries, external_func_entries))
}
