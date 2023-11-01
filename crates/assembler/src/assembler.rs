// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_assembly_parser::ast::{
    FuncNode, Instruction, LocalNode, ModuleElementNode, ModuleNode, ParamNode,
};
use ancvm_binary::{
    module_image::{
        func_index_section::FuncIndexItem,
        func_name_section::FuncNameEntry,
        func_section::FuncEntry,
        local_variable_section::{LocalListEntry, LocalVariableEntry},
        type_section::TypeEntry,
    },
    utils::BytecodeWriter,
};
use ancvm_types::{opcode::Opcode, DataType};

use crate::AssembleError;

pub struct ModuleEntry {
    pub name: String,
    pub runtime_version_major: u16,
    pub runtime_version_minor: u16,

    // pub shared_packages: Vec<String>,
    pub type_entries: Vec<TypeEntry>,
    pub local_list_entries: Vec<LocalListEntry>,
    pub func_entries: Vec<FuncEntry>,

    pub func_name_entries: Vec<FuncNameEntry>,
}

pub struct ModuleIndexEntry {
    // essential
    pub func_index_items: Vec<FuncIndexItem>,
    // optional
    // pub data_index_items: Vec<DataIndexItem>,
}

// struct LocalNameMapStack {
//     // levels example:
//     //
//     // - level 0: (name0, name1 ...)
//     // - level 1: (name0, name1, name2 ...)
//     // - level 2: (name0 ...)
//     name_levels: Vec<Vec<String>>,
// }
//
// impl LocalNameMapStack {
//     pub fn new() -> Self {
//         Self {
//             name_levels: vec![],
//         }
//     }
//
//     pub fn get_local_level_and_index_by_name(
//         &self,
//         local_variable_name: &str,
//     ) -> Result<(usize, usize), AssembleError> {
//         for (level_index, level) in self.name_levels.iter().enumerate() {
//             if let Some(name_index) = level.iter().position(|name| name == local_variable_name) {
//                 return Ok((level_index, name_index));
//             }
//         }
//
//         Err(AssembleError::new(&format!(
//             "Can not find the local variable: {}",
//             local_variable_name
//         )))
//     }
// }

enum FlowKind {
    Function,
    Block,

    // opcode:u16, padding:u16,
    // local_list_index:u32,
    // next_inst_offset:u32
    BlockNez,

    // opcode:u16, padding:u16,
    // type_index:u32,
    // local_list_index:u32,
    // alt_inst_offset:u32
    BlockAlt,
}

struct FlowItem {
    addr: usize, // the address of instruction
    kind: FlowKind,

    // opcode:u16, param reversed_index:u16,
    // next_inst_offset:i32
    //
    // push this stub to stack ONLY if the break target is a block
    // not a function.
    // and the instruction break may come from another blocks, e.g.
    // the 'break reversed-index:1' in child blocks.
    //
    // instruction 'recur' doesn't need the stub stack
    breaks: Vec<usize>,

    local_names: Vec<String>,
}

// the stack of control flow
struct FlowStack {
    flow_items: Vec<FlowItem>,
}

impl FlowStack {
    pub fn new() -> Self {
        Self { flow_items: vec![] }
    }

    pub fn push(&mut self, addr: usize, kind: FlowKind, local_names: Vec<String>) {
        let stub_item = FlowItem {
            addr,
            kind,
            breaks: vec![],
            local_names,
        };
        self.flow_items.push(stub_item);
    }

    pub fn pop(&mut self) -> Option<FlowItem> {
        self.flow_items.pop()
    }

    pub fn add_break(&mut self, addr: usize, reversed_index: usize) {
        let flow_item = self.get_flow_item(reversed_index);
        flow_item.breaks.push(addr);
    }

    pub fn get_flow_item(&mut self, reversed_index: usize) -> &mut FlowItem {
        let index = self.flow_items.len() - reversed_index - 1;
        &mut self.flow_items[index]
    }

    pub fn get_local_variable_reversed_index_and_index(
        &self,
        local_variable_name: &str,
    ) -> Result<(usize, usize), AssembleError> {
        for (level_index, flow_item) in self.flow_items.iter().enumerate() {
            if let Some(name_index) = flow_item
                .local_names
                .iter()
                .position(|name| name == local_variable_name)
            {
                let reversed_index = self.flow_items.len() - level_index - 1;
                return Ok((reversed_index, name_index));
            }
        }

        Err(AssembleError::new(&format!(
            "Can not find the local variable: {}",
            local_variable_name
        )))
    }
}

pub fn assemble_module_index(module_entries: &[ModuleEntry]) -> ModuleIndexEntry {
    todo!()
}

pub fn assemble_module_node(module_node: &ModuleNode) -> Result<ModuleEntry, AssembleError> {
    let name = module_node.name.clone();
    let runtime_version_major = module_node.runtime_version_major;
    let runtime_version_minor = module_node.runtime_version_minor;

    let func_name_entries = assemble_func_name_entries(module_node);

    let func_nodes = module_node
        .element_nodes
        .iter()
        .filter_map(|node| match node {
            ModuleElementNode::FuncNode(func_node) => Some(func_node),
            _ => None,
        })
        .collect::<Vec<_>>();

    let (type_entries, local_list_entries, func_entries) =
        assemble_func_nodes(&func_nodes, &func_name_entries)?;

    let module_entry = ModuleEntry {
        name,
        runtime_version_major,
        runtime_version_minor,
        type_entries,
        local_list_entries,
        func_entries,
        func_name_entries,
    };

    Ok(module_entry)
}

fn assemble_func_name_entries(module_node: &ModuleNode) -> Vec<FuncNameEntry> {
    let mut func_name_entries = vec![];

    // todo add names of imported functions

    let imported_func_count: usize = 0; // todo

    for (idx, element_node) in module_node.element_nodes.iter().enumerate() {
        if let ModuleElementNode::FuncNode(func_node) = element_node {
            if let Some(func_name) = &func_node.name {
                let entry = FuncNameEntry {
                    name: func_name.clone(),
                    func_pub_index: idx + imported_func_count,
                    exported: func_node.exported,
                };
                func_name_entries.push(entry);
            }
        }
    }

    func_name_entries
}

fn get_func_pub_index_by_name(func_name_entries: &[FuncNameEntry], name: &str) -> Option<usize> {
    func_name_entries
        .iter()
        .position(|entry| entry.name == name)
}

fn assemble_func_nodes(
    func_nodes: &[&FuncNode],
    func_name_entries: &[FuncNameEntry],
) -> Result<(Vec<TypeEntry>, Vec<LocalListEntry>, Vec<FuncEntry>), AssembleError> {
    let mut type_entries = vec![];
    let mut local_list_entries = vec![];
    let mut func_entries = vec![];

    for (func_idx, func_node) in func_nodes.iter().enumerate() {
        let type_index =
            get_type_index_with_creation(&mut type_entries, &func_node.params, &func_node.results);

        let local_list_index = get_local_index_with_creation(
            &mut local_list_entries,
            &func_node.params,
            &func_node.locals,
        );

        let local_names = get_local_names(&func_node.params, &func_node.locals);

        let mut flow_stack = FlowStack::new();
        flow_stack.push(0, FlowKind::Function, local_names);

        let code = assemble_func_code(
            &func_node.code,
            func_name_entries,
            &mut type_entries,
            &mut local_list_entries,
            &mut flow_stack,
        )?;

        // check control flow stack
        if flow_stack.flow_items.len() != 1 {
            return Err(AssembleError::new(&format!(
                "There is a control flow error in the function \"{}\"",
                func_node.name.clone().unwrap_or(func_idx.to_string())
            )));
        }

        func_entries.push(FuncEntry {
            type_index,
            local_list_index,
            code,
        });
    }

    Ok((type_entries, local_list_entries, func_entries))
}

fn get_type_index_with_creation(
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

fn get_local_index_with_creation(
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
        .position(|entry| entry.variables == variable_entries);

    if let Some(idx) = opt_idx {
        idx
    } else {
        let idx = local_list_entries.len();
        local_list_entries.push(LocalListEntry::new(variable_entries));
        idx
    }
}

fn get_local_names(param_nodes: &[ParamNode], local_nodes: &[LocalNode]) -> Vec<String> {
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
    code: &Instruction,
    func_name_entries: &[FuncNameEntry],
    type_entries: &mut [TypeEntry],
    local_list_entries: &mut [LocalListEntry],
    flow_stack: &mut FlowStack,
) -> Result<Vec<u8>, AssembleError> {
    if let Instruction::Code(instructions) = code {
        let mut bytecode_writer = BytecodeWriter::new();

        for instruction in instructions {
            assemble_instruction(
                instruction,
                func_name_entries,
                type_entries,
                local_list_entries,
                flow_stack,
                &mut bytecode_writer,
            )?;
        }

        // write the implied instruction 'end'
        let bc = bytecode_writer.write_opcode(Opcode::end).to_bytes();
        Ok(bc)
    } else {
        Err(AssembleError::new("Expect function code."))
    }
}

fn process_end_instruction() {
    // todo
}

fn assemble_instruction(
    instruction: &Instruction,
    func_name_entries: &[FuncNameEntry],
    type_entries: &mut [TypeEntry],
    local_list_entries: &mut [LocalListEntry],
    flow_stack: &mut FlowStack,
    bytecode_writer: &mut BytecodeWriter,
) -> Result<(), AssembleError> {
    todo!()
}

#[cfg(test)]
mod tests {
    // fn parse_from_str(s: &str) -> Result<ModuleNode, CompileError> {
    //     init_instruction_kind_table();
    //
    //     let mut chars = s.chars();
    //     let mut char_iter = PeekableIterator::new(&mut chars, 2);
    //     let mut tokens = lex(&mut char_iter)?.into_iter();
    //     let mut token_iter = PeekableIterator::new(&mut tokens, 2);
    //     parse(&mut token_iter)
    // }

    /*
    fn test_multithread_thread_id() {
        let code0 = BytecodeWriter::new()
            .write_opcode_i32(Opcode::envcall, EnvCallCode::thread_id as u32)
            .write_opcode(Opcode::end)
            .to_bytes();

        let binary0 = build_module_binary_with_single_function(
            vec![DataType::I32], // params
            vec![DataType::I32], // results
            vec![],              // local vars
            code0,
        );

        let program_source0 = InMemoryProgramSource::new(vec![binary0]);
        let multithread_program0 = MultithreadProgram::new(program_source0);
        let child_thread_id0 = create_thread(&multithread_program0, 0, 0, vec![]);

        const FIRST_CHILD_THREAD_ID: u32 = 1;

        CHILD_THREADS.with(|child_threads_cell| {
            let mut child_threads = child_threads_cell.borrow_mut();
            let opt_child_thread = child_threads.remove(&child_thread_id0);
            let child_thread = opt_child_thread.unwrap();

            let result0 = child_thread.join_handle.join().unwrap();

            assert_eq!(
                result0.unwrap(),
                vec![ForeignValue::UInt32(FIRST_CHILD_THREAD_ID)]
            );
        });
    }

    fn test_module_common_sections_save_and_load() {

        // build ModuleImage instance
        let section_entries: Vec<&dyn SectionEntry> =
            vec![&type_section, &func_section, &local_var_section];
        let (section_items, sections_data) = ModuleImage::convert_from_entries(&section_entries);
        let module_image = ModuleImage {
            name: "main",
            items: &section_items,
            sections_data: &sections_data,
        };

        // save
        let mut image_data: Vec<u8> = Vec::new();
        module_image.save(&mut image_data).unwrap();

        assert_eq!(&image_data[0..8], IMAGE_MAGIC_NUMBER);

        // load
        let module_image_restore = ModuleImage::load(&image_data).unwrap();
        assert_eq!(module_image_restore.items.len(), 3);

    }
    */
}
