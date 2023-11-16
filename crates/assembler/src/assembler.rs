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
        external_func_name_section::ExternalFuncNameEntry,
        func_name_section::FuncNameEntry,
        func_section::FuncEntry,
        local_variable_section::{LocalListEntry, LocalVariableEntry},
        type_section::TypeEntry,
    },
};
use ancvm_parser::ast::{DataKindNode, DataNode};
use ancvm_parser::ast::{
    FuncNode, ImmF32, ImmF64, Instruction, LocalNode, ModuleElementNode, ModuleNode, ParamNode,
};
use ancvm_types::{opcode::Opcode, DataType};

use crate::{AssembleError, ModuleEntry};

// the names of functions and datas
struct SymbolNameBook<'a> {
    func_name_entries: &'a [FuncNameEntry],
    data_name_entries: &'a [DataNameEntry],
    external_func_name_entries: &'a [ExternalFuncNameEntry],
}

impl<'a> SymbolNameBook<'a> {
    pub fn new(
        func_name_entries: &'a [FuncNameEntry],
        data_name_entries: &'a [DataNameEntry],
        external_func_name_entries: &'a [ExternalFuncNameEntry],
    ) -> Self {
        Self {
            func_name_entries,
            data_name_entries,
            external_func_name_entries,
        }
    }

    pub fn get_func_pub_index(&self, name: &str) -> Result<usize, AssembleError> {
        self.func_name_entries
            .iter()
            .position(|entry| entry.name == name)
            .ok_or(AssembleError::new(&format!(
                "Can not find the function: {}",
                name
            )))
    }

    pub fn get_data_pub_index(&self, name: &str) -> Result<usize, AssembleError> {
        self.data_name_entries
            .iter()
            .position(|entry| entry.name == name)
            .ok_or(AssembleError::new(&format!(
                "Can not find the data: {}",
                name
            )))
    }

    pub fn get_external_func_index(&self, name: &str) -> Result<usize, AssembleError> {
        self.func_name_entries
            .iter()
            .position(|entry| entry.name == name)
            .ok_or(AssembleError::new(&format!(
                "Can not find the external function: {}",
                name
            )))
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
    flow_items: Vec<FlowItem>,
}

struct FlowItem {
    addr: usize, // the address of instruction
    flow_kind: FlowKind,
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
        Self { flow_items: vec![] }
    }

    pub fn push(&mut self, addr: usize, flow_kind: FlowKind, local_names: Vec<String>) {
        let stub_item = FlowItem {
            addr,
            flow_kind,
            break_items: vec![],
            local_names,
        };
        self.flow_items.push(stub_item);
    }

    pub fn pop(&mut self) -> FlowItem {
        self.flow_items.pop().unwrap()
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

    fn get_flow_item_by_reversed_index(&mut self, reversed_index: usize) -> &mut FlowItem {
        let idx = self.flow_items.len() - reversed_index - 1;
        &mut self.flow_items[idx]
    }

    pub fn get_block_addr(&self, reversed_index: usize) -> usize {
        let idx = self.flow_items.len() - reversed_index - 1;
        self.flow_items[idx].addr
    }

    pub fn get_reversed_index_to_function(&self) -> usize {
        self.flow_items.len() - 1
    }

    pub fn get_reversed_index_to_nearest_block(&self) -> usize {
        let idx = self
            .flow_items
            .iter()
            .rposition(|item| item.flow_kind == FlowKind::Block)
            .expect("Can't find \"block\" on the control flow stack.");
        self.flow_items.len() - idx - 1
    }

    /// return (reversed_index, variable_index)
    pub fn get_local_variable_reversed_index_and_variable_index(
        &self,
        local_variable_name: &str,
    ) -> Result<(usize, usize), AssembleError> {
        for (level_index, flow_item) in self.flow_items.iter().enumerate() {
            if let Some(variable_index) = flow_item
                .local_names
                .iter()
                .position(|name| name == local_variable_name)
            {
                let reversed_index = self.flow_items.len() - level_index - 1;
                return Ok((reversed_index, variable_index));
            }
        }

        Err(AssembleError::new(&format!(
            "Can not find the local variable: {}",
            local_variable_name
        )))
    }
}

pub fn assemble_module_node(module_node: &ModuleNode) -> Result<ModuleEntry, AssembleError> {
    let name = module_node.name.clone();
    let runtime_version_major = module_node.runtime_version_major;
    let runtime_version_minor = module_node.runtime_version_minor;

    let func_nodes = module_node
        .element_nodes
        .iter()
        .filter_map(|node| match node {
            ModuleElementNode::FuncNode(func_node) => Some(func_node),
            _ => None,
        })
        .collect::<Vec<_>>();

    let read_only_data_nodes = module_node
        .element_nodes
        .iter()
        .filter_map(|node| match node {
            ModuleElementNode::DataNode(data_node)
                if matches!(data_node.data_kind, DataKindNode::ReadOnly(_)) =>
            {
                Some(data_node)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    let read_write_data_nodes = module_node
        .element_nodes
        .iter()
        .filter_map(|node| match node {
            ModuleElementNode::DataNode(data_node)
                if matches!(data_node.data_kind, DataKindNode::ReadWrite(_)) =>
            {
                Some(data_node)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    let uninit_data_nodes = module_node
        .element_nodes
        .iter()
        .filter_map(|node| match node {
            ModuleElementNode::DataNode(data_node)
                if matches!(data_node.data_kind, DataKindNode::Uninit(_)) =>
            {
                Some(data_node)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    let func_name_entries = assemble_func_name_entries(&func_nodes);
    let data_name_entries = assemble_data_name_entries(
        &read_only_data_nodes,
        &read_write_data_nodes,
        &uninit_data_nodes,
    );
    let external_func_name_entries = assemble_external_function_name_entries(module_node);

    let symbol_name_book = SymbolNameBook::new(
        &func_name_entries,
        &data_name_entries,
        &external_func_name_entries,
    );

    let (type_entries, local_list_entries, func_entries) =
        assemble_func_nodes(&func_nodes, &symbol_name_book)?;

    let (read_only_data_entries, read_write_data_entries, uninit_data_entries) =
        assemble_data_nodes(
            &read_only_data_nodes,
            &read_write_data_nodes,
            &uninit_data_nodes,
        )?;

    let module_entry = ModuleEntry {
        name,
        runtime_version_major,
        runtime_version_minor,
        type_entries,
        local_list_entries,
        func_entries,
        read_only_data_entries,
        read_write_data_entries,
        uninit_data_entries,
        func_name_entries,
        data_name_entries,
        external_func_name_entries,
    };

    Ok(module_entry)
}

fn assemble_func_name_entries(func_nodes: &[&FuncNode]) -> Vec<FuncNameEntry> {
    let mut func_name_entries = vec![];

    // todo:: add names of imported functions

    let imported_func_count: usize = 0; // todo
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

fn assemble_data_name_entries(
    ro_data_nodes: &[&DataNode],
    rw_data_nodes: &[&DataNode],
    uninit_data_nodes: &[&DataNode],
) -> Vec<DataNameEntry> {
    let mut data_name_entries = vec![];

    // todo:: add names of imported datas

    let imported_data_count: usize = 0; // todo

    let mut data_pub_idx = imported_data_count;

    for data_node in ro_data_nodes {
        let entry = DataNameEntry {
            name: data_node.name.clone(),
            data_pub_index: data_pub_idx,
            exported: data_node.exported,
        };
        data_name_entries.push(entry);
        data_pub_idx += 1;
    }

    for data_node in rw_data_nodes {
        let entry = DataNameEntry {
            name: data_node.name.clone(),
            data_pub_index: data_pub_idx,
            exported: data_node.exported,
        };
        data_name_entries.push(entry);
        data_pub_idx += 1;
    }

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

fn assemble_external_function_name_entries(
    _module_node: &ModuleNode,
) -> Vec<ExternalFuncNameEntry> {
    // let mut external_func_name_entries = vec![];
    // todo
    // external_func_name_entries
    vec![]
}

type AssembleResultForFuncNode = (Vec<TypeEntry>, Vec<LocalListEntry>, Vec<FuncEntry>);

fn assemble_func_nodes(
    func_nodes: &[&FuncNode],
    symbol_name_book: &SymbolNameBook,
) -> Result<AssembleResultForFuncNode, AssembleError> {
    let mut type_entries = vec![];
    let mut local_list_entries = vec![];
    let mut func_entries = vec![];

    for func_node in func_nodes {
        let type_index = find_existing_type_index_with_creating_when_not_found(
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
            symbol_name_book,
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

fn find_existing_type_index_with_creating_when_not_found(
    type_entries: &mut Vec<TypeEntry>,
    param_nodes: &[ParamNode],
    results: &[DataType],
) -> usize {
    let params = param_nodes
        .iter()
        .map(|node| node.data_type)
        .collect::<Vec<_>>();

    let opt_idx = type_entries
        .iter()
        .position(|entry| entry.params == params && entry.results == results);

    if let Some(idx) = opt_idx {
        idx
    } else {
        let idx = type_entries.len();
        type_entries.push(TypeEntry {
            params,
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
    symbol_name_book: &SymbolNameBook,
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
            symbol_name_book,
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
    if !flow_stack.flow_items.is_empty() {
        return Err(AssembleError::new(&format!(
            "Control flow does not end in the function \"{}\"",
            func_name
        )));
    }

    Ok(bytecode_writer.to_bytes())
}

fn assemble_instruction(
    instruction: &Instruction,
    symbol_name_book: &SymbolNameBook,
    type_entries: &mut Vec<TypeEntry>,
    local_list_entries: &mut Vec<LocalListEntry>,
    flow_stack: &mut FlowStack,
    bytecode_writer: &mut BytecodeWriter,
) -> Result<(), AssembleError> {
    match instruction {
        Instruction::NoParams { opcode, operands } => assemble_instruction_kind_no_params(
            opcode,
            operands,
            symbol_name_book,
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
                symbol_name_book,
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
                symbol_name_book,
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
                symbol_name_book,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            assemble_instruction(
                value,
                symbol_name_book,
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
            name,
            offset,
        } => {
            let data_pub_index = symbol_name_book.get_data_pub_index(name)?;

            // bytecode: (param offset_bytes:i16 data_public_index:i32)
            bytecode_writer.write_opcode_i16_i32(*opcode, *offset, data_pub_index as u32);
        }
        Instruction::DataStore {
            opcode,
            name,
            offset,
            value,
        } => {
            assemble_instruction(
                value,
                symbol_name_book,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            let data_pub_index = symbol_name_book.get_data_pub_index(name)?;

            // bytecode: (param offset_bytes:i16 data_public_index:i32)
            bytecode_writer.write_opcode_i16_i32(*opcode, *offset, data_pub_index as u32);
        }
        Instruction::DataLongLoad {
            opcode,
            name,
            offset,
        } => {
            assemble_instruction(
                offset,
                symbol_name_book,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            let data_pub_index = symbol_name_book.get_data_pub_index(name)?;

            // bytecode: (param data_public_index:i32)
            bytecode_writer.write_opcode_i32(*opcode, data_pub_index as u32);
        }
        Instruction::DataLongStore {
            opcode,
            name,
            offset,
            value,
        } => {
            assemble_instruction(
                offset,
                symbol_name_book,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            assemble_instruction(
                value,
                symbol_name_book,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            let data_pub_index = symbol_name_book.get_data_pub_index(name)?;

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
                symbol_name_book,
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
                symbol_name_book,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            assemble_instruction(
                value,
                symbol_name_book,
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
                symbol_name_book,
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
                symbol_name_book,
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
                symbol_name_book,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            assemble_instruction(
                right,
                symbol_name_book,
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
                symbol_name_book,
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
                symbol_name_book,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            // write inst 'end'
            bytecode_writer.write_opcode(Opcode::end);
            let addr_of_next_to_end = bytecode_writer.get_addr();

            // pop flow stck
            let flow_item = flow_stack.pop();

            // fill stubs
            fill_stubs_for_block_end(&flow_item, bytecode_writer, addr_of_next_to_end);
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
                symbol_name_book,
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
                symbol_name_book,
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

            // fill the stub of inst 'block_alt'
            bytecode_writer.fill_block_alt_stub(addr_of_block_alt, addr_of_next_to_break as u32);

            // add break item
            flow_stack.add_break(addr_of_break, 0);

            // assemble node 'alternate'
            assemble_instruction(
                alternate,
                symbol_name_book,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            // write inst 'end'
            bytecode_writer.write_opcode(Opcode::end);
            let addr_of_next_to_end = bytecode_writer.get_addr();

            // pop flow stck
            let flow_item = flow_stack.pop();

            // fill stubs
            fill_stubs_for_block_end(&flow_item, bytecode_writer, addr_of_next_to_end);
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
                    symbol_name_book,
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
                    symbol_name_book,
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

                // pop flow stck
                let flow_item = flow_stack.pop();

                // fill stubs
                fill_stubs_for_block_end(&flow_item, bytecode_writer, addr_of_next_to_end);
            }

            // write node 'default'
            if let Some(default_instruction) = default {
                // assemble node 'consequent'
                assemble_instruction(
                    default_instruction,
                    symbol_name_book,
                    type_entries,
                    local_list_entries,
                    flow_stack,
                    bytecode_writer,
                )?;
            } else {
                // write the inst 'unreachable'
                bytecode_writer.write_opcode(Opcode::unreachable);
            }

            // write inst 'end'
            bytecode_writer.write_opcode(Opcode::end);
            let addr_of_next_to_end = bytecode_writer.get_addr();

            // pop flow stack
            let flow_item = flow_stack.pop();

            // fill stubs
            fill_stubs_for_block_end(&flow_item, bytecode_writer, addr_of_next_to_end);
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
            let type_index = find_existing_type_index_with_creating_when_not_found(
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
                symbol_name_book,
                type_entries,
                local_list_entries,
                flow_stack,
                bytecode_writer,
            )?;

            // write inst 'end'
            bytecode_writer.write_opcode(Opcode::end);
            let addr_of_next_to_end = bytecode_writer.get_addr();

            // pop flow stack
            let flow_item = flow_stack.pop();

            // fill stubs
            fill_stubs_for_block_end(&flow_item, bytecode_writer, addr_of_next_to_end);
        }
        // Instruction::Code(_) => {
        //     unreachable!("Only node \"function\" can have child node \"code\".")
        // },
        Instruction::Do(instructions) => {
            for instruction in instructions {
                assemble_instruction(
                    instruction,
                    symbol_name_book,
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
                    symbol_name_book,
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
                    symbol_name_book,
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
            let start_inst_offset = (addr_of_recur - addr_of_block) as u32;
            bytecode_writer.fill_recur_stub(addr_of_recur, start_inst_offset);
        }
        Instruction::Return(instructions) => {
            // break to the function
            for instruction in instructions {
                assemble_instruction(
                    instruction,
                    symbol_name_book,
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
        Instruction::TailCall(instructions) => {
            // recur to the function

            for instruction in instructions {
                assemble_instruction(
                    instruction,
                    symbol_name_book,
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
        Instruction::Call { name: _, args: _ } => todo!(),
        Instruction::DynCall { num: _, args: _ } => todo!(),
        Instruction::EnvCall { num: _, args: _ } => todo!(),
        Instruction::SysCall { num: _, args: _ } => todo!(),
        Instruction::ExtCall { name: _, args: _ } => todo!(),
    }

    Ok(())
}

fn fill_stubs_for_block_end(
    flow_item: &FlowItem,
    bytecode_writer: &mut BytecodeWriter,
    addr_of_next_to_end: usize,
) {
    let addr_of_block = flow_item.addr;
    let next_inst_offset_of_block = (addr_of_next_to_end - addr_of_block) as u32;

    match flow_item.flow_kind {
        FlowKind::BlockNez => {
            bytecode_writer.fill_block_nez_stub(addr_of_block, next_inst_offset_of_block);
        }
        _ => {
            // only inst 'block_nez' has stub 'next_inst_offset'.
        }
    }

    // fill stubs of insts 'break'
    for break_item in &flow_item.break_items {
        let addr_of_break = break_item.addr;
        let next_inst_offset_of_break = (addr_of_next_to_end - addr_of_break) as u32;
        bytecode_writer.fill_break_stub(break_item.addr, next_inst_offset_of_break);
    }
}

fn assemble_instruction_kind_no_params(
    opcode: &Opcode,
    operands: &[Instruction],
    symbol_name_book: &SymbolNameBook,
    type_entries: &mut Vec<TypeEntry>,
    local_list_entries: &mut Vec<LocalListEntry>,
    flow_stack: &mut FlowStack,
    bytecode_writer: &mut BytecodeWriter,
) -> Result<(), AssembleError> {
    for instruction in operands {
        assemble_instruction(
            instruction,
            symbol_name_book,
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
    read_only_data_nodes: &[&DataNode],
    read_write_data_nodes: &[&DataNode],
    uninit_data_nodes: &[&DataNode],
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
