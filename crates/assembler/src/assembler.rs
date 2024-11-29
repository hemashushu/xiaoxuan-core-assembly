// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use anc_image::{
    bytecode_writer::BytecodeWriter,
    entry::{
        DataNamePathEntry, ExternalFunctionEntry, ExternalLibraryEntry, FunctionEntry,
        FunctionNamePathEntry, ImportDataEntry, ImportFunctionEntry, ImportModuleEntry,
        InitedDataEntry, LocalVariableEntry, LocalVariableListEntry, TypeEntry, UninitDataEntry,
    },
    module_image::ImageType,
};
use anc_isa::{
    opcode::Opcode, DataSectionType, DependencyLocal, MemoryDataType, ModuleDependency,
    OperandDataType,
};
use anc_parser_asm::ast::{
    DataNode, DataSection, ExpressionNode, ExternalNode, FixedDeclareDataType, FunctionNode,
    ImportNode, InstructionNode, LocalVariable, ModuleNode, NamedParameter,
};

use crate::{entry::ImageCommonEntry, AssembleError};

/// Get the "module name" and "name path" from a "full name",
/// note that the "name path" may be empty if the "full name"
/// does not include that part.
///
/// about the "full_name" and "name_path"
/// -------------------------------------
/// - "full_name" = "module_name::name_path"
/// - "name_path" = "namespace::identifier"
/// - "namespace" = "sub_module_name"{0,N}
///
fn get_module_name_and_name_path(full_name: &str) -> (&str, &str) {
    full_name.split_once("::").unwrap_or((full_name, ""))
}

/// Get the "namespace" and "identifier" from a "name path",
/// note that the "namespace" may be empty if the "name path"
/// does not include that part.
///
/// about the "full_name" and "name_path"
/// -------------------------------------
/// - "full_name" = "module_name::name_path"
/// - "name_path" = "namespace::identifier"
/// - "namespace" = "sub_module_name"{0,N}
fn get_namespace_and_identifier(name_path: &str) -> (&str, &str) {
    name_path.rsplit_once("::").unwrap_or(("", name_path))
}

/// about library "full_name"
/// -------------------------
/// "full_name" = "library_name::identifier"
fn get_library_name_and_identifier(full_name: &str) -> (&str, &str) {
    full_name.split_once("::").unwrap()
}

fn create_virtual_dependency_module() -> ImportModuleEntry {
    ImportModuleEntry::new(
        "module".to_owned(),
        Box::new(ModuleDependency::Local(Box::new(DependencyLocal {
            path: "".to_owned(),
            values: None,
            condition: None,
        }))),
    )
}

/// parameter 'module_full_name' is the full name of a sub-module.
///
/// e.g.
/// consider there is a module/project named 'network'.
///
/// | full name               | file path             |
/// |-------------------------|-----------------------|
/// | "network"               | "/lib.ancasm"         |
/// | "network::http"         | "/http.ancasm"        |
/// | "network::http::client" | "/http/client.ancasm" |
///
/// the root module, such as 'app.anc' and 'lib.{anc|ancir|ancasm}' has
/// the empty ("") name path.
///
/// about the "full_name" and "name_path"
/// -------------------------------------
/// - "full_name" = "module_name::name_path"
/// - "name_path" = "namespace::identifier"
/// - "namespace" = "sub_module_name"{0,N}
pub fn assemble_module_node(
    module_node: &ModuleNode,
    module_full_name: &str,
    import_module_entries: &[ImportModuleEntry],
    external_library_entries: &[ExternalLibraryEntry],
) -> Result<ImageCommonEntry, AssembleError> {
    // the `import_module_entries` should not be empty.
    // it must contains a virtual module named "module" with is
    // refer to the current module.
    assert!(!import_module_entries.is_empty());
    assert_eq!(
        import_module_entries.first(),
        Some(&create_virtual_dependency_module())
    );

    let (module_name, module_name_path) = get_module_name_and_name_path(module_full_name);

    let mut type_entries: Vec<TypeEntry> = vec![];
    let mut local_variable_list_entries: Vec<LocalVariableListEntry> = vec![];

    // add an empty params and results type record.
    type_entries.push(TypeEntry {
        params: vec![],
        results: vec![],
    });

    // add an empty local variable list record.
    local_variable_list_entries.push(LocalVariableListEntry::new(vec![]));

    let AssembleResultForDependencies {
        import_module_entries,
        import_module_ids,
        external_library_entries,
        external_library_ids,
    } = assemble_dependencies(import_module_entries, external_library_entries)?;

    let AssembleResultForImportNodes {
        import_function_entries,
        import_function_ids,
        import_data_entries,
        import_read_only_data_ids,
        import_read_write_data_ids,
        import_uninit_data_ids,
    } = assemble_import_nodes(&import_module_ids, &module_node.imports, &mut type_entries)?;

    let AssembleResultForExternalNode {
        external_function_entries,
        external_function_ids,
    } = assemble_external_nodes(
        &external_library_ids,
        &module_node.externals,
        &mut type_entries,
    )?;

    let (function_name_path_entries, function_ids) =
        assemble_function_name_entries(&module_node.functions, module_name_path);

    let AssembleResultForDataNameEntry {
        data_name_path_entries,
        read_only_data_ids,
        read_write_data_ids,
        uninit_data_ids,
    } = assemble_data_name_entries(&module_node.datas, module_name_path);

    let identifier_public_index_lookup_table =
        IdentifierPublicIndexLookupTable::build(IdentifierSource {
            import_function_ids,
            function_ids,
            //
            import_read_only_data_ids,
            import_read_write_data_ids,
            import_uninit_data_ids,
            //
            read_only_data_ids,
            read_write_data_ids,
            uninit_data_ids,
            //
            external_function_ids,
        });

    let function_entries = assemble_function_nodes(
        &module_node.functions,
        &mut type_entries,
        &mut local_variable_list_entries,
        &identifier_public_index_lookup_table,
    )?;

    let AssembleResultForDataNodes {
        read_only_data_entries,
        read_write_data_entries,
        uninit_data_entries,
    } = assemble_data_nodes(&module_node.datas)?;

    let module_entry = ImageCommonEntry {
        name: module_name.to_owned(),
        image_type: ImageType::ObjectFile,
        //
        import_module_entries,
        import_function_entries,
        import_data_entries,
        //
        type_entries,
        local_variable_list_entries,
        function_entries,
        //
        read_only_data_entries,
        read_write_data_entries,
        uninit_data_entries,
        //
        function_name_path_entries,
        data_name_path_entries,
        //
        external_library_entries,
        external_function_entries,
    };

    Ok(module_entry)
}

fn assemble_function_name_entries(
    function_nodes: &[FunctionNode],
    module_name_path: &str,
) -> (Vec<FunctionNamePathEntry>, Vec<String>) {
    let mut function_name_path_entries = vec![];
    // let mut function_public_index = import_function_count;
    let mut function_ids: Vec<String> = vec![];

    for function_node in function_nodes {
        // add function id
        function_ids.push(function_node.name.to_owned());

        let name_path = if module_name_path.is_empty() {
            function_node.name.to_owned()
        } else {
            format!("{}::{}", module_name_path, function_node.name)
        };

        // add function name entry
        let function_name_path_entry = FunctionNamePathEntry::new(name_path, function_node.export);

        function_name_path_entries.push(function_name_path_entry);
    }

    (function_name_path_entries, function_ids)
}

struct AssembleResultForDataNameEntry {
    data_name_path_entries: Vec<DataNamePathEntry>,
    read_only_data_ids: Vec<String>,
    read_write_data_ids: Vec<String>,
    uninit_data_ids: Vec<String>,
}

fn assemble_data_name_entries(
    data_nodes: &[DataNode],
    module_name_path: &str,
) -> AssembleResultForDataNameEntry {
    // the data name paths in `DataNamePathSection` follow these order:
    // 1. internal read-only data
    // 2. internal read-write data
    // 3. internal uninit data
    let mut data_name_path_entries = vec![];

    let mut read_only_data_ids: Vec<String> = vec![];
    let mut read_write_data_ids: Vec<String> = vec![];
    let mut uninit_data_ids: Vec<String> = vec![];

    for data_node in data_nodes {
        let id = data_node.name.to_owned();
        match data_node.data_section {
            DataSection::ReadOnly(_) => {
                read_only_data_ids.push(id);
            }
            DataSection::ReadWrite(_) => {
                read_write_data_ids.push(id);
            }
            DataSection::Uninit(_) => {
                uninit_data_ids.push(id);
            }
        }

        let name_path = if module_name_path.is_empty() {
            data_node.name.to_owned()
        } else {
            format!("{}::{}", module_name_path, data_node.name)
        };

        let data_name_path_entry = DataNamePathEntry::new(name_path, data_node.export);
        data_name_path_entries.push(data_name_path_entry);
    }

    AssembleResultForDataNameEntry {
        data_name_path_entries,
        read_only_data_ids,
        read_write_data_ids,
        uninit_data_ids,
    }
}

/// this table only contains function/data names,
/// does NOT contain name path or fullname.
///
/// about the "full_name" and "name_path"
/// -------------------------------------
/// - "full_name" = "module_name::name_path"
/// - "name_path" = "namespace::identifier"
/// - "namespace" = "sub_module_name"{0,N}
struct IdentifierSource {
    import_function_ids: Vec<String>,
    function_ids: Vec<String>,
    //
    import_read_only_data_ids: Vec<String>,
    import_read_write_data_ids: Vec<String>,
    import_uninit_data_ids: Vec<String>,
    //
    read_only_data_ids: Vec<String>,
    read_write_data_ids: Vec<String>,
    uninit_data_ids: Vec<String>,
    //
    external_function_ids: Vec<String>,
}

/// this table only contains function/data names,
/// does NOT contain name path or fullname.
///
/// about the "full_name" and "name_path"
/// -------------------------------------
/// - "full_name" = "module_name::name_path"
/// - "name_path" = "namespace::identifier"
/// - "namespace" = "sub_module_name"{0,N}
struct IdentifierPublicIndexLookupTable {
    functions: Vec<NameIndexPair>,
    datas: Vec<NameIndexPair>,
    external_functions: Vec<NameIndexPair>,
}

struct NameIndexPair {
    // the identifier/name of function or data.
    // for the import and external items, the id may also be the alias name.
    //
    // this id should only be the function/data names,
    // is NOT name path or fullname.
    //
    // about the "full_name" and "name_path"
    // -------------------------------------
    // - "full_name" = "module_name::name_path"
    // - "name_path" = "namespace::identifier"
    // - "namespace" = "sub_module_name"{0,N}
    id: String,

    // the function public index is mixed by the following items (and are sorted by the following order):
    // - the imported functions
    // - the internal functions
    //
    // the data public index is mixed by the following items (and are sorted by the following order):
    //
    // - imported read-only data items
    // - imported read-write data items
    // - imported uninitilized data items
    //
    // - internal read-only data items
    // - internal read-write data items
    // - internal uninitilized data items
    public_index: usize,
}

impl IdentifierPublicIndexLookupTable {
    pub fn build(identifier_source: IdentifierSource) -> Self {
        let mut functions: Vec<NameIndexPair> = vec![];
        let mut datas: Vec<NameIndexPair> = vec![];
        let mut external_functions: Vec<NameIndexPair> = vec![];

        // fill function ids
        for function_ids in [
            &identifier_source.import_function_ids,
            &identifier_source.function_ids,
        ] {
            functions.extend(
                function_ids
                    .iter()
                    .enumerate()
                    .map(|(idx, id)| NameIndexPair {
                        id: id.to_owned(),
                        public_index: idx,
                    }),
            );
        }

        // fill data ids
        for data_ids in [
            &identifier_source.import_read_only_data_ids,
            &identifier_source.import_read_write_data_ids,
            &identifier_source.import_uninit_data_ids,
            &identifier_source.read_only_data_ids,
            &identifier_source.read_write_data_ids,
            &identifier_source.uninit_data_ids,
        ] {
            datas.extend(data_ids.iter().enumerate().map(|(idx, id)| NameIndexPair {
                id: id.to_owned(),
                public_index: idx,
            }));
        }

        // full external function ids
        external_functions.extend(
            identifier_source
                .external_function_ids
                .iter()
                .enumerate()
                .map(|(idx, id)| NameIndexPair {
                    id: id.to_owned(),
                    public_index: idx,
                }),
        );

        Self {
            functions,
            datas,
            external_functions,
        }
    }

    pub fn get_function_public_index(&self, identifier: &str) -> Result<usize, AssembleError> {
        match self.functions.iter().find(|entry| entry.id == identifier) {
            Some(p) => Ok(p.public_index),
            None => Err(AssembleError::new(&format!(
                "Can not find the function: {}",
                identifier
            ))),
        }
    }

    pub fn get_data_public_index(&self, identifier: &str) -> Result<usize, AssembleError> {
        match self.datas.iter().find(|entry| entry.id == identifier) {
            Some(p) => Ok(p.public_index),
            None => Err(AssembleError::new(&format!(
                "Can not find the data: {}",
                identifier
            ))),
        }
    }

    pub fn get_external_function_index(&self, identifier: &str) -> Result<usize, AssembleError> {
        match self
            .external_functions
            .iter()
            .find(|entry| entry.id == identifier)
        {
            Some(p) => Ok(p.public_index),
            None => Err(AssembleError::new(&format!(
                "Can not find the external function: {}",
                identifier
            ))),
        }
    }
}

fn assemble_function_nodes(
    function_nodes: &[FunctionNode],
    type_entries: &mut Vec<TypeEntry>,
    local_variable_list_entries: &mut Vec<LocalVariableListEntry>,
    identifier_public_index_lookup_table: &IdentifierPublicIndexLookupTable,
) -> Result<Vec<FunctionEntry>, AssembleError> {
    let mut function_entries = vec![];

    for function_node in function_nodes {
        let type_index = find_or_create_function_type_index2(
            type_entries,
            &function_node.params,
            &function_node.results,
        );

        let local_variable_list_index = find_or_create_local_variable_list_index(
            local_variable_list_entries,
            &function_node.params,
            &function_node.locals,
        );

        let local_variable_names_include_params =
            build_local_variable_names_by_params_and_local_variables(
                &function_node.params,
                &function_node.locals,
            );

        let code = assemble_function_code(
            &function_node.name, // for building error message
            local_variable_names_include_params,
            &function_node.body,
            identifier_public_index_lookup_table,
            type_entries,
            local_variable_list_entries,
        )?;

        function_entries.push(FunctionEntry {
            type_index,
            local_list_index: local_variable_list_index,
            code,
        });
    }

    Ok(function_entries)
}

/// function type = params + results
fn find_or_create_function_type_index(
    type_entries: &mut Vec<TypeEntry>,
    params: &[OperandDataType],
    results: &[OperandDataType],
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

/// function type = params + results
fn find_or_create_function_type_index2(
    type_entries: &mut Vec<TypeEntry>,
    named_params: &[NamedParameter],
    results: &[OperandDataType],
) -> usize {
    let params: Vec<OperandDataType> = named_params.iter().map(|item| item.data_type).collect();
    find_or_create_function_type_index(type_entries, &params, results)
}

/// local variable list type = function params + local vars
fn find_or_create_local_variable_list_index(
    local_variable_list_entries: &mut Vec<LocalVariableListEntry>,
    named_params: &[NamedParameter],
    local_variables: &[LocalVariable],
) -> usize {
    let entries_from_params = named_params
        .iter()
        .map(|item| match item.data_type {
            OperandDataType::I32 => LocalVariableEntry::from_i32(),
            OperandDataType::I64 => LocalVariableEntry::from_i64(),
            OperandDataType::F32 => LocalVariableEntry::from_f32(),
            OperandDataType::F64 => LocalVariableEntry::from_f64(),
        })
        .collect::<Vec<_>>();

    let entries_from_local_variables = local_variables
        .iter()
        .map(|item| match item.data_type {
            FixedDeclareDataType::I64 => LocalVariableEntry::from_i64(),
            FixedDeclareDataType::I32 => LocalVariableEntry::from_i32(),
            FixedDeclareDataType::F64 => LocalVariableEntry::from_f64(),
            FixedDeclareDataType::F32 => LocalVariableEntry::from_f32(),
            FixedDeclareDataType::FixedBytes(length, align) => LocalVariableEntry {
                memory_data_type: MemoryDataType::Bytes,
                length: length as u32,
                align: if let Some(value) = align {
                    value as u16
                } else {
                    1_u16
                },
            },
        })
        .collect::<Vec<_>>();

    let mut entries = vec![];
    entries.extend_from_slice(&entries_from_params);
    entries.extend_from_slice(&entries_from_local_variables);

    let opt_idx = local_variable_list_entries
        .iter()
        .position(|item| item.local_variable_entries == entries);

    if let Some(idx) = opt_idx {
        idx
    } else {
        let idx = local_variable_list_entries.len();
        local_variable_list_entries.push(LocalVariableListEntry::new(entries));
        idx
    }
}

fn build_local_variable_names_by_params_and_local_variables(
    named_params: &[NamedParameter],
    local_variables: &[LocalVariable],
) -> Vec<String> {
    let names_from_params = named_params
        .iter()
        .map(|item| item.name.clone())
        .collect::<Vec<_>>();

    let names_from_local_variables = local_variables
        .iter()
        .map(|item| item.name.clone())
        .collect::<Vec<_>>();

    let mut names = vec![];
    names.extend_from_slice(&names_from_params);
    names.extend_from_slice(&names_from_local_variables);

    names
}

fn assemble_function_code(
    function_name: &str, // for building error message
    local_variable_names_include_params: Vec<String>,
    expression_node: &ExpressionNode,
    identifier_public_index_lookup_table: &IdentifierPublicIndexLookupTable,
    type_entries: &mut Vec<TypeEntry>,
    local_variable_list_entries: &mut Vec<LocalVariableListEntry>,
) -> Result<Vec<u8>, AssembleError> {
    let mut bytecode_writer = BytecodeWriter::new();
    let mut control_flow_stack = ControlFlowStack::new();

    control_flow_stack.push_layer(
        0,
        ControlFlowKind::Function,
        local_variable_names_include_params,
    );

    emit_expression(
        function_name,
        expression_node,
        identifier_public_index_lookup_table,
        type_entries,
        local_variable_list_entries,
        &mut control_flow_stack,
        &mut bytecode_writer,
    )?;

    // write the implied instruction 'end'
    bytecode_writer.write_opcode(Opcode::end);

    // pop flow stack
    control_flow_stack.pop_layer_and_fill_stubs(&mut bytecode_writer, 0);

    // check control flow stack
    if !control_flow_stack.control_flow_items.is_empty() {
        return Err(AssembleError::new(&format!(
            "Not all control flows closed in the function \"{}\"",
            function_name
        )));
    }

    Ok(bytecode_writer.to_bytes())
}

fn emit_expression(
    function_name: &str, // for building error message
    expression_node: &ExpressionNode,
    identifier_public_index_lookup_table: &IdentifierPublicIndexLookupTable,
    type_entries: &mut Vec<TypeEntry>,
    local_variable_list_entries: &mut Vec<LocalVariableListEntry>,
    control_flow_stack: &mut ControlFlowStack,
    bytecode_writer: &mut BytecodeWriter,
) -> Result<(), AssembleError> {
    // match expression_node {
    //     ExpressionNode::Group(vec) => todo!(),
    //     ExpressionNode::Instruction(instruction_node) => todo!(),
    //     ExpressionNode::When(when_node) => todo!(),
    //     ExpressionNode::If(if_node) => todo!(),
    //     ExpressionNode::For(for_node) => todo!(),
    //     ExpressionNode::Break(break_node) => todo!(),
    //     ExpressionNode::Recur(break_node) => todo!(),
    // }

    Ok(())
}

fn emit_instruction(
    function_name: &str, // for building error message
    instruction_node: &InstructionNode,
    identifier_public_index_lookup_table: &IdentifierPublicIndexLookupTable,
    type_entries: &mut Vec<TypeEntry>,
    local_variable_list_entries: &mut Vec<LocalVariableListEntry>,
    control_flow_stack: &mut ControlFlowStack,
    bytecode_writer: &mut BytecodeWriter,
) -> Result<(), AssembleError> {
    //     match instruction {
    //         Instruction::NoParams { opcode, operands } => assemble_instruction_kind_no_params(
    //             opcode,
    //             operands,
    //             identifier_public_index_lookup_table,
    //             type_entries,
    //             local_list_entries,
    //             control_flow_stack,
    //             bytecode_writer,
    //         )?,
    //         Instruction::ImmI32(value) => {
    //             bytecode_writer.write_opcode_i32(Opcode::i32_imm, *value);
    //         }
    //         Instruction::ImmI64(value) => {
    //             bytecode_writer.write_opcode_pesudo_i64(Opcode::i64_imm, *value);
    //         }
    //         Instruction::ImmF32(value) => {
    //             bytecode_writer.write_opcode_pesudo_f32(Opcode::f32_imm, *value);
    //         }
    //         Instruction::ImmF64(value) => {
    //             bytecode_writer.write_opcode_pesudo_f64(Opcode::f64_imm, *value);
    //         }
    //         Instruction::LocalLoad {
    //             opcode,
    //             name,
    //             offset,
    //         } => {
    //             let (reversed_index, variable_index) =
    //                 control_flow_stack.get_local_variable_reversed_index_and_variable_index(name)?;
    //
    //             // bytecode: (param reversed_index:i16 offset_bytes:i16 local_variable_index:i16)
    //             bytecode_writer.write_opcode_i16_i16_i16(
    //                 *opcode,
    //                 reversed_index as u16,
    //                 *offset,
    //                 variable_index as u16,
    //             );
    //         }
    //         Instruction::LocalStore {
    //             opcode,
    //             name,
    //             offset,
    //             value,
    //         } => {
    //             assemble_instruction(
    //                 value,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             let (reversed_index, variable_index) =
    //                 control_flow_stack.get_local_variable_reversed_index_and_variable_index(name)?;
    //
    //             // bytecode: (param reversed_index:i16 offset_bytes:i16 local_variable_index:i16)
    //             bytecode_writer.write_opcode_i16_i16_i16(
    //                 *opcode,
    //                 reversed_index as u16,
    //                 *offset,
    //                 variable_index as u16,
    //             );
    //         }
    //         Instruction::LocalLongLoad {
    //             opcode,
    //             name,
    //             offset,
    //         } => {
    //             assemble_instruction(
    //                 offset,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             let (reversed_index, variable_index) =
    //                 control_flow_stack.get_local_variable_reversed_index_and_variable_index(name)?;
    //
    //             // bytecode: (param reversed_index:i16 local_variable_index:i32)
    //             bytecode_writer.write_opcode_i16_i32(
    //                 *opcode,
    //                 reversed_index as u16,
    //                 variable_index as u32,
    //             );
    //         }
    //         Instruction::LocalLongStore {
    //             opcode,
    //             name,
    //             offset,
    //             value,
    //         } => {
    //             // assemble 'offset' first, then 'value'
    //             assemble_instruction(
    //                 offset,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             assemble_instruction(
    //                 value,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             let (reversed_index, variable_index) =
    //                 control_flow_stack.get_local_variable_reversed_index_and_variable_index(name)?;
    //
    //             // bytecode: (param reversed_index:i16 local_variable_index:i32)
    //             bytecode_writer.write_opcode_i16_i32(
    //                 *opcode,
    //                 reversed_index as u16,
    //                 variable_index as u32,
    //             );
    //         }
    //         Instruction::DataLoad {
    //             opcode,
    //             id: name,
    //             offset,
    //         } => {
    //             let data_public_index = identifier_public_index_lookup_table.get_data_public_index(name)?;
    //
    //             // bytecode: (param offset_bytes:i16 data_public_index:i32)
    //             bytecode_writer.write_opcode_i16_i32(*opcode, *offset, data_public_index as u32);
    //         }
    //         Instruction::DataStore {
    //             opcode,
    //             id: name,
    //             offset,
    //             value,
    //         } => {
    //             assemble_instruction(
    //                 value,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             let data_public_index = identifier_public_index_lookup_table.get_data_public_index(name)?;
    //
    //             // bytecode: (param offset_bytes:i16 data_public_index:i32)
    //             bytecode_writer.write_opcode_i16_i32(*opcode, *offset, data_public_index as u32);
    //         }
    //         Instruction::DataLongLoad {
    //             opcode,
    //             id: name,
    //             offset,
    //         } => {
    //             assemble_instruction(
    //                 offset,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             let data_public_index = identifier_public_index_lookup_table.get_data_public_index(name)?;
    //
    //             // bytecode: (param data_public_index:i32)
    //             bytecode_writer.write_opcode_i32(*opcode, data_public_index as u32);
    //         }
    //         Instruction::DataLongStore {
    //             opcode,
    //             id: name,
    //             offset,
    //             value,
    //         } => {
    //             assemble_instruction(
    //                 offset,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             assemble_instruction(
    //                 value,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             let data_public_index = identifier_public_index_lookup_table.get_data_public_index(name)?;
    //
    //             // bytecode: (param data_public_index:i32)
    //             bytecode_writer.write_opcode_i32(*opcode, data_public_index as u32);
    //         }
    //         Instruction::HeapLoad {
    //             opcode,
    //             offset,
    //             addr,
    //         } => {
    //             assemble_instruction(
    //                 addr,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             // bytecode: (param offset_bytes:i16)
    //             bytecode_writer.write_opcode_i16(*opcode, *offset);
    //         }
    //         Instruction::HeapStore {
    //             opcode,
    //             offset,
    //             addr,
    //             value,
    //         } => {
    //             assemble_instruction(
    //                 addr,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             assemble_instruction(
    //                 value,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             // bytecode: (param offset_bytes:i16)
    //             bytecode_writer.write_opcode_i16(*opcode, *offset);
    //         }
    //         Instruction::UnaryOp {
    //             opcode,
    //             source: number,
    //         } => {
    //             assemble_instruction(
    //                 number,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             bytecode_writer.write_opcode(*opcode);
    //         }
    //         Instruction::UnaryOpWithImmI16 {
    //             opcode,
    //             imm: amount,
    //             source: number,
    //         } => {
    //             assemble_instruction(
    //                 number,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             bytecode_writer.write_opcode_i16(*opcode, *amount);
    //         }
    //         Instruction::BinaryOp {
    //             opcode,
    //             left,
    //             right,
    //         } => {
    //             // assemble 'left' first, then 'right'
    //             assemble_instruction(
    //                 left,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             assemble_instruction(
    //                 right,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             bytecode_writer.write_opcode(*opcode);
    //         }
    //         Instruction::When {
    //             // locals,
    //             test,
    //             consequent,
    //         } => {
    //             // | structure         | assembly          | instruction(s)     |
    //             // |-------------------|-------------------|--------------------|
    //             // |                   |                   | ..a..              |
    //             // | if ..a.. {        | (when (a)         | block_nez -\       |
    //             // |    ..b..          |       (b)         |   ..b..    |       |
    //             // | }                 | )                 | end        |       |
    //             // |                   |                   | ...    <---/       |
    //             // |-------------------|-------------------|--------------------|
    //
    //             // bytecode:
    //             // - block_nez (param local_list_index:i32, next_inst_offset:i32)
    //
    //             // assemble node 'test'
    //             assemble_instruction(
    //                 test,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             // local index and names
    //             let local_list_index =
    //                 find_or_create_local_variable_list_index(local_list_entries, &[], &[]);
    //
    //             // write inst 'block_nez'
    //             let addr_of_block_nez = bytecode_writer.write_opcode_i32_i32(
    //                 Opcode::block_nez,
    //                 local_list_index as u32,
    //                 0, // stub for 'next_inst_offset'
    //             );
    //
    //             // push flow stack
    //             control_flow_stack.push(addr_of_block_nez, ControlFlowKind::BlockNez, vec![]);
    //
    //             // assemble node 'consequent'
    //             assemble_instruction(
    //                 consequent,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             // write inst 'end'
    //             bytecode_writer.write_opcode(Opcode::end);
    //             let addr_of_next_to_end = bytecode_writer.get_addr();
    //
    //             // pop flow stck and fill stubs
    //             control_flow_stack.pop_and_fill_stubs(bytecode_writer, addr_of_next_to_end);
    //         }
    //         Instruction::If {
    //             results,
    //             // locals,
    //             test,
    //             consequent,
    //             alternate,
    //         } => {
    //             // | structure         | assembly          | instruction(s)     |
    //             // |-------------------|-------------------|--------------------|
    //             // |                   |                   | ..a..              |
    //             // | if ..a.. {        | (if (a)           | block_alt ---\     |
    //             // |    ..b..          |     (b)           |   ..b..      |     |
    //             // | } else {          |     (c)           |   break 0  --|-\   |
    //             // |    ..c..          | )                 |   ..c..  <---/ |   |
    //             // | }                 |                   | end            |   |
    //             // |                   |                   | ...      <-----/   |
    //             // |-------------------|-------------------|--------------------|
    //             // |                   |                   | ..a..              |
    //             // | if ..a.. {        | (if (a)           | block_alt ---\     |
    //             // |    ..b..          |     (b)           |   ..b..      |     |
    //             // | } else if ..c.. { |     (if (c)       |   break 0 ---|---\ |
    //             // |    ..d..          |         (d)       |   ..c..  <---/   | |
    //             // | } else {          |         (e)       |   block_alt --\  | |
    //             // |    ..e..          |     )             |     ..d..     |  | |
    //             // | }                 | )                 |     break 0 --|-\| |
    //             // |                   |                   |     ..e..  <--/ || |
    //             // |                   |                   |   end           || |
    //             // |                   |                   | end        <----/| |
    //             // |                   |                   | ...        <-----/ |
    //             // |                   |                   |                    |
    //             // |                   | ----------------- | ------------------ |
    //
    //             // bytecode:
    //             // - block_alt (param type_index:i32, local_list_index:i32, alt_inst_offset:i32)
    //             // - break (param reversed_index:i16, next_inst_offset:i32)
    //
    //             // assemble node 'test'
    //             assemble_instruction(
    //                 test,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             // type index
    //             let type_index = find_or_create_function_type_index(type_entries, &[], results);
    //
    //             // local index
    //             let local_list_index =
    //                 find_or_create_local_variable_list_index(local_list_entries, &[], &[]);
    //
    //             // write inst 'block_alt'
    //             let addr_of_block_alt = bytecode_writer.write_opcode_i32_i32_i32(
    //                 Opcode::block_alt,
    //                 type_index as u32,
    //                 local_list_index as u32,
    //                 0, // stub for 'alt_inst_offset'
    //             );
    //
    //             // push flow stack
    //             control_flow_stack.push(addr_of_block_alt, ControlFlowKind::BlockAlt, vec![]);
    //
    //             // assemble node 'consequent'
    //             assemble_instruction(
    //                 consequent,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             // write inst 'break'
    //             let addr_of_break = bytecode_writer.write_opcode_i16_i32(
    //                 Opcode::break_,
    //                 0, // reversed_index
    //                 0, // next_inst_offset
    //             );
    //             let addr_of_next_to_break = bytecode_writer.get_addr();
    //             let alt_inst_offset = (addr_of_next_to_break - addr_of_block_alt) as u32;
    //
    //             // fill the stub of inst 'block_alt'
    //             bytecode_writer.fill_block_alt_stub(addr_of_block_alt, alt_inst_offset);
    //
    //             // add break item
    //             control_flow_stack.add_break(addr_of_break, 0);
    //
    //             // assemble node 'alternate'
    //             assemble_instruction(
    //                 alternate,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             // write inst 'end'
    //             bytecode_writer.write_opcode(Opcode::end);
    //             let addr_of_next_to_end = bytecode_writer.get_addr();
    //
    //             // pop flow stack and fill stubs
    //             control_flow_stack.pop_and_fill_stubs(bytecode_writer, addr_of_next_to_end);
    //         }
    //         Instruction::Branch {
    //             results,
    //             // locals,
    //             cases,
    //             default,
    //         } => {
    //             // | structure         | assembly          | instruction(s)     |
    //             // |-------------------|-------------------|--------------------|
    //             // |                   |                   |                    |
    //             // |                   | (branch           | block              |
    //             // |                   |   (case (a) (b))  |   ..a..            |
    //             // |                   |   (case (c) (d))  |   block_nez -\     |
    //             // |                   |   (default (e))   |     ..b..    |     |
    //             // |                   | )                 |     break 1 -|--\  |
    //             // |                   |                   |   end        |  |  |
    //             // |                   |                   |   ..c..  <---/  |  |
    //             // |                   |                   |   block_nez -\  |  |
    //             // |                   |                   |     ..d..    |  |  |
    //             // |                   |                   |     break 1 -|--|  |
    //             // |                   |                   |   end        |  |  |
    //             // |                   |                   |   ..e..  <---/  |  |
    //             // |                   |                   | end             |  |
    //             // |                   |                   | ...        <----/  |
    //             // |-------------------|-------------------|--------------------|
    //
    //             // bytecode:
    //             // - block (param type_index:i32, local_list_index:i32)
    //             // - block_nez (param local_list_index:i32, next_inst_offset:i32)
    //             // - break (param reversed_index:i16, next_inst_offset:i32)
    //
    //             // type index
    //             let type_index = find_or_create_function_type_index(type_entries, &[], results);
    //
    //             // local index and names
    //             let local_list_index =
    //                 find_or_create_local_variable_list_index(local_list_entries, &[], &[]);
    //
    //             // write inst 'block'
    //             let addr_of_block = bytecode_writer.write_opcode_i32_i32(
    //                 Opcode::block,
    //                 type_index as u32,
    //                 local_list_index as u32,
    //             );
    //
    //             // push flow stack
    //             control_flow_stack.push(addr_of_block, ControlFlowKind::Block, vec![]);
    //
    //             // write branches
    //             for case in cases {
    //                 // assemble node 'test'
    //                 assemble_instruction(
    //                     &case.test,
    //                     identifier_public_index_lookup_table,
    //                     type_entries,
    //                     local_list_entries,
    //                     control_flow_stack,
    //                     bytecode_writer,
    //                 )?;
    //
    //                 // local index and names
    //                 let case_local_list_index =
    //                     find_or_create_local_variable_list_index(local_list_entries, &[], &[]);
    //
    //                 // write inst 'block_nez'
    //                 let addr_of_block_nez = bytecode_writer.write_opcode_i32_i32(
    //                     Opcode::block_nez,
    //                     case_local_list_index as u32,
    //                     0, // stub for 'next_inst_offset'
    //                 );
    //
    //                 // push flow stack
    //                 control_flow_stack.push(addr_of_block_nez, ControlFlowKind::BlockNez, vec![]);
    //
    //                 // assemble node 'consequent'
    //                 assemble_instruction(
    //                     &case.consequent,
    //                     identifier_public_index_lookup_table,
    //                     type_entries,
    //                     local_list_entries,
    //                     control_flow_stack,
    //                     bytecode_writer,
    //                 )?;
    //
    //                 // write inst 'break 1'
    //
    //                 let addr_of_break = bytecode_writer.write_opcode_i16_i32(
    //                     Opcode::break_,
    //                     1,
    //                     0, // stub for 'next_inst_offset'
    //                 );
    //
    //                 // add 'break' item to control flow stack
    //                 control_flow_stack.add_break(addr_of_break, 1);
    //
    //                 // write inst 'end'
    //                 bytecode_writer.write_opcode(Opcode::end);
    //                 let addr_of_next_to_end = bytecode_writer.get_addr();
    //
    //                 // pop flow stack and fill stubs
    //                 control_flow_stack.pop_and_fill_stubs(bytecode_writer, addr_of_next_to_end);
    //             }
    //
    //             // write node 'default'
    //             if let Some(default_instruction) = default {
    //                 // assemble node 'consequent'
    //                 assemble_instruction(
    //                     default_instruction,
    //                     identifier_public_index_lookup_table,
    //                     type_entries,
    //                     local_list_entries,
    //                     control_flow_stack,
    //                     bytecode_writer,
    //                 )?;
    //             } else {
    //                 // write the inst 'unreachable'
    //                 bytecode_writer
    //                     .write_opcode_i32(Opcode::unreachable, UNREACHABLE_CODE_NO_DEFAULT_ARM);
    //             }
    //
    //             // write inst 'end'
    //             bytecode_writer.write_opcode(Opcode::end);
    //             let addr_of_next_to_end = bytecode_writer.get_addr();
    //
    //             // pop flow stack and fill stubs
    //             control_flow_stack.pop_and_fill_stubs(bytecode_writer, addr_of_next_to_end);
    //         }
    //         Instruction::For {
    //             params,
    //             results,
    //             locals,
    //             code,
    //         } => {
    //             // | structure         | assembly          | instructions(s)    |
    //             // |-------------------|-------------------|--------------------|
    //             // | loop {            | (for (code        | block              |
    //             // |    ...            |   ...             |   ...   <--\       |
    //             // | }                 |   (recur ...)     |   recur 0 -/       |
    //             // |                   | ))                | end                |
    //             // |-------------------|-------------------|--------------------|
    //             // |                   |                   |                    |
    //             // |                   | (for (code        | block              |
    //             // |                   |   (when (a)       |   ..a..    <---\   |
    //             // |                   |     (code ...     |   block_nez    |   |
    //             // |                   |       (recur ...) |     ...        |   |
    //             // |                   |     )             |     recur 1 ---/   |
    //             // |                   |   )               |   end              |
    //             // |                   | ))                | end                |
    //             // |                   |                   |                    |
    //             // |                   |                   |                    |
    //             // |-------------------|-------------------|--------------------|
    //             // |                   |                   |                    |
    //             // |                   | (for (code        | block              |
    //             // |                   |   ...             |   ...      <---\   |
    //             // |                   |   (when (a)       |   ..a..        |   |
    //             // |                   |     (recur ...)   |   block_nez    |   |
    //             // |                   |   )               |     recur 1 ---/   |
    //             // |                   | ))                |   end              |
    //             // |                   |                   | end                |
    //             // |                   |                   |                    |
    //             // |                   |                   |                    |
    //             // |-------------------|-------------------|--------------------|
    //
    //             // bytecode:
    //             // - block (param type_index:i32, local_list_index:i32)
    //             // - recur (param reversed_index:i16, start_inst_offset:i32)
    //
    //             // type index
    //             let type_index = find_existing_type_index_with_creating_when_not_found_by_param_nodes(
    //                 type_entries,
    //                 params,
    //                 results,
    //             );
    //
    //             // local index
    //             let local_list_index =
    //                 find_or_create_local_variable_list_index(local_list_entries, params, locals);
    //
    //             // local names
    //             let local_names =
    //                 build_local_variable_names_by_params_and_local_variables(params, locals);
    //
    //             // write inst 'block'
    //             let addr_of_block = bytecode_writer.write_opcode_i32_i32(
    //                 Opcode::block,
    //                 type_index as u32,
    //                 local_list_index as u32,
    //             );
    //
    //             // push flow stack
    //             control_flow_stack.push(addr_of_block, ControlFlowKind::Block, local_names);
    //
    //             // assemble node 'consequent'
    //             assemble_instruction(
    //                 code,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             // write inst 'end'
    //             bytecode_writer.write_opcode(Opcode::end);
    //             let addr_of_next_to_end = bytecode_writer.get_addr();
    //
    //             // pop flow stack and fill stubs
    //             control_flow_stack.pop_and_fill_stubs(bytecode_writer, addr_of_next_to_end);
    //         }
    //         Instruction::Do(instructions) => {
    //             for instruction in instructions {
    //                 assemble_instruction(
    //                     instruction,
    //                     identifier_public_index_lookup_table,
    //                     type_entries,
    //                     local_list_entries,
    //                     control_flow_stack,
    //                     bytecode_writer,
    //                 )?;
    //             }
    //         }
    //         Instruction::Break(instructions) => {
    //             // note that the statement 'break' is not the same as the instruction 'break',
    //             // the statement 'break' only break the nearest instruction 'block'.
    //
    //             for instruction in instructions {
    //                 assemble_instruction(
    //                     instruction,
    //                     identifier_public_index_lookup_table,
    //                     type_entries,
    //                     local_list_entries,
    //                     control_flow_stack,
    //                     bytecode_writer,
    //                 )?;
    //             }
    //
    //             let reversed_index = control_flow_stack.get_reversed_index_to_for();
    //
    //             // write inst 'break'
    //             let addr_of_break = bytecode_writer.write_opcode_i16_i32(
    //                 Opcode::break_,
    //                 reversed_index as u16,
    //                 0, // stub for 'next_inst_offset'
    //             );
    //
    //             control_flow_stack.add_break(addr_of_break, reversed_index);
    //         }
    //         Instruction::Recur(instructions) => {
    //             // note that the statement 'recur' is not the same as the instruction 'recur',
    //             // the statement 'recur' only recur to the nearest instruction 'block'.
    //
    //             for instruction in instructions {
    //                 assemble_instruction(
    //                     instruction,
    //                     identifier_public_index_lookup_table,
    //                     type_entries,
    //                     local_list_entries,
    //                     control_flow_stack,
    //                     bytecode_writer,
    //                 )?;
    //             }
    //
    //             let reversed_index = control_flow_stack.get_reversed_index_to_for();
    //
    //             // write inst 'recur'
    //             let addr_of_recur = bytecode_writer.write_opcode_i16_i32(
    //                 Opcode::recur,
    //                 reversed_index as u16,
    //                 0, // stub for 'start_inst_offset'
    //             );
    //
    //             let addr_of_block = control_flow_stack.get_block_addr(reversed_index);
    //
    //             // the length of inst 'block' is 12 bytes
    //             let addr_of_next_to_block = addr_of_block + 12;
    //             let start_inst_offset = (addr_of_recur - addr_of_next_to_block) as u32;
    //             bytecode_writer.fill_recur_stub(addr_of_recur, start_inst_offset);
    //         }
    //         Instruction::Return(instructions) => {
    //             // break to the function
    //             for instruction in instructions {
    //                 assemble_instruction(
    //                     instruction,
    //                     identifier_public_index_lookup_table,
    //                     type_entries,
    //                     local_list_entries,
    //                     control_flow_stack,
    //                     bytecode_writer,
    //                 )?;
    //             }
    //
    //             let reversed_index = control_flow_stack.get_reversed_index_to_function();
    //
    //             // write inst 'break'
    //             bytecode_writer.write_opcode_i16_i32(
    //                 Opcode::break_,
    //                 reversed_index as u16,
    //                 0, // 'next_inst_offset' is ignored when the target is the function
    //             );
    //         }
    //         Instruction::FnRecur(instructions) => {
    //             // recur to the function
    //
    //             for instruction in instructions {
    //                 assemble_instruction(
    //                     instruction,
    //                     identifier_public_index_lookup_table,
    //                     type_entries,
    //                     local_list_entries,
    //                     control_flow_stack,
    //                     bytecode_writer,
    //                 )?;
    //             }
    //
    //             let reversed_index = control_flow_stack.get_reversed_index_to_function();
    //
    //             // write inst 'recur'
    //             bytecode_writer.write_opcode_i16_i32(
    //                 Opcode::recur,
    //                 reversed_index as u16,
    //                 0, // 'start_inst_offset' is ignored when the target is function
    //             );
    //         }
    //         Instruction::Call { id, args } => {
    //             for instruction in args {
    //                 assemble_instruction(
    //                     instruction,
    //                     identifier_public_index_lookup_table,
    //                     type_entries,
    //                     local_list_entries,
    //                     control_flow_stack,
    //                     bytecode_writer,
    //                 )?;
    //             }
    //
    //             let function_public_index = identifier_public_index_lookup_table.get_function_public_index(id)?;
    //             bytecode_writer.write_opcode_i32(Opcode::call, function_public_index as u32);
    //         }
    //         Instruction::DynCall {
    //             public_index: num,
    //             args,
    //         } => {
    //             for instruction in args {
    //                 assemble_instruction(
    //                     instruction,
    //                     identifier_public_index_lookup_table,
    //                     type_entries,
    //                     local_list_entries,
    //                     control_flow_stack,
    //                     bytecode_writer,
    //                 )?;
    //             }
    //
    //             // assemble the function public index operand
    //             assemble_instruction(
    //                 num,
    //                 identifier_public_index_lookup_table,
    //                 type_entries,
    //                 local_list_entries,
    //                 control_flow_stack,
    //                 bytecode_writer,
    //             )?;
    //
    //             bytecode_writer.write_opcode(Opcode::dyncall);
    //         }
    //         Instruction::EnvCall { num, args } => {
    //             for instruction in args {
    //                 assemble_instruction(
    //                     instruction,
    //                     identifier_public_index_lookup_table,
    //                     type_entries,
    //                     local_list_entries,
    //                     control_flow_stack,
    //                     bytecode_writer,
    //                 )?;
    //             }
    //
    //             bytecode_writer.write_opcode_i32(Opcode::envcall, *num);
    //         }
    //         Instruction::SysCall { num, args } => {
    //             for instruction in args {
    //                 assemble_instruction(
    //                     instruction,
    //                     identifier_public_index_lookup_table,
    //                     type_entries,
    //                     local_list_entries,
    //                     control_flow_stack,
    //                     bytecode_writer,
    //                 )?;
    //             }
    //
    //             bytecode_writer.write_opcode_i32(Opcode::i32_imm, *num);
    //             bytecode_writer.write_opcode_i32(Opcode::i32_imm, args.len() as u32);
    //             bytecode_writer.write_opcode(Opcode::syscall);
    //         }
    //         Instruction::ExtCall { id, args } => {
    //             for instruction in args {
    //                 assemble_instruction(
    //                     instruction,
    //                     identifier_public_index_lookup_table,
    //                     type_entries,
    //                     local_list_entries,
    //                     control_flow_stack,
    //                     bytecode_writer,
    //                 )?;
    //             }
    //
    //             let external_function_idx = identifier_public_index_lookup_table.get_external_function_index(id)?;
    //             bytecode_writer.write_opcode_i32(Opcode::extcall, external_function_idx as u32);
    //         }
    //         // macro
    //         Instruction::MacroGetFunctionPublicIndex { id } => {
    //             let function_public_index = identifier_public_index_lookup_table.get_function_public_index(id)?;
    //             bytecode_writer.write_opcode_i32(Opcode::i32_imm, function_public_index as u32);
    //         }
    //         Instruction::Debug { code } => {
    //             bytecode_writer.write_opcode_i32(Opcode::debug, *code);
    //         }
    //         Instruction::Unreachable { code } => {
    //             bytecode_writer.write_opcode_i32(Opcode::unreachable, *code);
    //         }
    //         Instruction::HostAddrFunction { id } => {
    //             let function_public_index = identifier_public_index_lookup_table.get_function_public_index(id)?;
    //             bytecode_writer
    //                 .write_opcode_i32(Opcode::host_addr_function, function_public_index as u32);
    //         }
    //     }
    //
    Ok(())
}

// fn assemble_instruction_kind_no_params(
//     opcode: &Opcode,
//     operands: &[Instruction],
//     identifier_public_index_lookup_table: &IdentifierPublicIndexLookupTable,
//     type_entries: &mut Vec<TypeEntry>,
//     local_list_entries: &mut Vec<LocalListEntry>,
//     control_flow_stack: &mut ControlFlowStack,
//     bytecode_writer: &mut BytecodeWriter,
// ) -> Result<(), AssembleError> {
//     for instruction in operands {
//         assemble_instruction(
//             instruction,
//             identifier_public_index_lookup_table,
//             type_entries,
//             local_list_entries,
//             control_flow_stack,
//             bytecode_writer,
//         )?;
//     }
//
//     bytecode_writer.write_opcode(*opcode);
//
//     Ok(())
// }

/**
 * XiaoXuan Core instruction set includes the following instructions
 * containing the "next_inst_offset" parameter:
 *
 * - block_alt (param type_index:i32, next_inst_offset:i32)
 * - block_nez (param local_list_index:i32, next_inst_offset:i32)
 * - break (param reversed_index:i16, next_inst_offset:i32)
 * - break_alt (param next_inst_offset:i32)
 * - break_nez (param reversed_index:i16, next_inst_offset:i32)
 *
 * When emitting the binary code for these instructions, the values of
 * these parameters are UNKNOWN and are only determined when the "end"
 * instruction is generated.
 *
 * Therefore, when the assembler generates the binary code for these
 * instructions, it first fills the parameter with the number `0`
 * (this blank space are called "stubs") and records the addresses (positions)
 * of these instructions. Then, when generating the "end" (and the "break_alt")
 * instruction, the `0` in the stub is replaced with the actual number.
 *
 * The structure "ControlFlowStack" is designed to implement the above purpose
 * and needs to call the corresponding methods:
 *
 * - todo
 * - todo
 * - todo
 *
 * Note:
 *
 * 1. Generating the "recur*" instruction does not require
 * inserting stubs because the value of the parameter "start_inst_offset" can
 * be obtained immediately through the structure "ControlFlowStack".
 *
 * 2. If the target layer of "break" is "function", no stub needs to be inserted,
 * and the "ControlFlowStack" is not needed because the "next_inst_offset" in
 * this case is directly ignored by the VM.
 *
 * 3. If the target layer of "recur" is "function", no stub needs to be inserted,
 * and the "ControlFlowStack" is not needed because the "start_inst_offset" in
 * this case is directly ignored by the VM.
 */
struct ControlFlowStack {
    control_flow_items: Vec<ControlFlowItem>,
}

struct ControlFlowItem {
    control_flow_kind: ControlFlowKind,

    // the address of the instruction
    address: usize,

    // 'break' instructions of the CURRENT layer.
    //
    // note that if the target layer of "break" is not the
    // current layer, the "break" item wouldn't be here.
    break_items: Vec<BreakItem>,

    // used to find the index of local variables by name
    local_variable_names_include_params: Vec<String>,
}

#[derive(Debug, PartialEq)]
enum ControlFlowKind {
    // to form the layer '0'.
    // this layer is needed when calculate the layer index.
    //
    // NO stub.
    Function,

    // for structure: 'for'
    //
    // bytecode:
    // block (opcode:i16 padding:i16 type_index:i32, local_list_index:i32)
    //
    // NO stub.
    Block,

    // for structure: 'when'
    //
    // bytecode:
    // block_nez (opcode:i16 padding:i16 local_list_index:i32 next_inst_offset:i32)
    //
    // stub: next_inst_offset
    BlockNez,

    // for structure: 'if'
    //
    // bytecode:
    // block_alt (opcode:i16 padding:i16 type_index:i32 next_inst_offset:i32)
    //
    // stub: next_inst_offset
    BlockAlt,
}

// bytecode:
//
// - break     (opcode:i16 reversed_index:i16 next_inst_offset:i32)
// - break_alt (opcode:i16 padding:i16        next_inst_offset:i32)
// - break_nez (opcode:i16 reversed_index:i16 next_inst_offset:i32)
//
// stub: next_inst_offset
struct BreakItem {
    break_type: BreakType,

    // the address of the 'break' instruction
    address: usize,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum BreakType {
    Break,
    BreakAlt,
    BreakNez,
}

impl ControlFlowStack {
    pub fn new() -> Self {
        Self {
            control_flow_items: vec![],
        }
    }

    /// call this function when entering a block
    /// (includes instruction 'block', 'block_nez', 'blocl_alt')
    pub fn push_layer(
        &mut self,
        address: usize,
        control_flow_kind: ControlFlowKind,
        local_variable_names_include_params: Vec<String>,
    ) {
        let control_flow_item = ControlFlowItem {
            address,
            control_flow_kind,
            break_items: vec![],
            local_variable_names_include_params,
        };
        self.control_flow_items.push(control_flow_item);
    }

    /// call this function when encounting instruction 'break', 'break_alt', and 'break_nez'
    ///
    /// - 'break_alt' is equivalent to 'break 0, next_inst_offset'.
    /// - when the target layer is "function", the `break` does not need stub.
    ///
    /// the "break item" would be only inserted to corresponding layer.
    pub fn add_break(&mut self, break_type: BreakType, address: usize, reversed_index: usize) {
        let control_flow_item = self.get_control_flow_item_by_reversed_index(reversed_index);

        if control_flow_item.control_flow_kind == ControlFlowKind::Function {
            // when the target layer is function, the instruction 'break' does not need stub,
            // because the parameter 'next_inst_offset' is ignored.
        } else {
            control_flow_item.break_items.push(BreakItem {
                break_type,
                address,
            });
        }

        if break_type == BreakType::BreakAlt {
            todo!()
        }
    }

    // /// call this function when leaving a function
    // pub fn pop_function_layer(&mut self) -> ControlFlowItem {
    //     self.control_flow_items.pop().unwrap()
    // }

    pub fn pop_layer_and_fill_stubs(
        &mut self,
        bytecode_writer: &mut BytecodeWriter,
        address_next_to_instruction_end: usize,
    ) {
        // pop flow stack
        let control_flow_item = self.control_flow_items.pop().unwrap();

        // fill stubs of the instruction 'block_nez'.
        //
        // note that only the instruction 'block_nez' contains stub named 'next_inst_offset',
        // although other 'block' instrcutions contain 'next_inst_offset', but
        // they do not have stubs.
        match control_flow_item.control_flow_kind {
            ControlFlowKind::BlockNez => {
                let addr_of_block = control_flow_item.address;
                let next_inst_offset = (address_next_to_instruction_end - addr_of_block) as u32;
                bytecode_writer.fill_block_nez_stub(addr_of_block, next_inst_offset);
            }
            _ => {
                // only inst 'block_nez' need to stub 'next_inst_offset'.
            }
        }

        // fill stubs of the instruction 'break'.
        //
        // instruction 'break' contains stub named 'next_inst_offset'.
        for break_item in &control_flow_item.break_items {
            let address_of_break = break_item.address;
            let next_inst_offset = (address_next_to_instruction_end - address_of_break) as u32;
            bytecode_writer.fill_break_stub(break_item.address, next_inst_offset);
        }
    }

    fn get_control_flow_item_by_reversed_index(
        &mut self,
        reversed_index: usize,
    ) -> &mut ControlFlowItem {
        let idx = self.control_flow_items.len() - reversed_index - 1;
        &mut self.control_flow_items[idx]
    }

    /// this function is used for getting the block address
    /// when emit the 'recur*' instruction.
    pub fn get_block_address(&self, reversed_index: usize) -> usize {
        let idx = self.control_flow_items.len() - reversed_index - 1;
        self.control_flow_items[idx].address
    }

    /// calculate the number of layers to the function
    pub fn get_reversed_index_to_function(&self) -> usize {
        self.control_flow_items.len() - 1
    }

    /// calculate the number of layers to the nearest 'block'
    pub fn get_reversed_index_to_the_nearest_for(&self) -> usize {
        let idx = self
            .control_flow_items
            .iter()
            .rposition(|item| item.control_flow_kind == ControlFlowKind::Block)
            .expect("Can't find \"for\" statement on the control flow stack.");
        self.control_flow_items.len() - idx - 1
    }

    /// return (reversed_index, variable_index)
    ///
    /// all local variables, including the parameters of function
    /// should not have duplicate names in the scope. e.g.
    ///
    /// ```ancasm
    /// fn add(left:i32, right:i32) -> i32 {
    ///     block (
    ///         temp:i32       /* valid */
    ///         left:i32       /* invalid, duplicated with the fn parameter 'left' */
    ///     ) -> ()
    ///     [
    ///         count:i32      /* valid */
    ///         left:i32       /* invalid, duplicated with the fn parameter 'left' */
    ///         temp:i32       /* invalid, duplicated with the 1st block parameter 'temp' */
    ///     ]
    ///     {
    ///         block (
    ///             abc:i32    /* valid */
    ///             count:i32  /* invalid, duplicated with the local variable 'count' */
    ///         )
    ///         {
    ///             ...
    ///         }
    ///
    ///         block (
    ///             abc:i32    /* valid, since it is out of the scope of the first 'abc' */
    ///         ) {
    ///             ...
    ///         }
    ///     }
    /// }
    /// ```
    ///
    pub fn get_local_variable_reversed_index_and_variable_index_by_name(
        &self,
        local_variable_name: &str,
    ) -> Result<(usize, usize), AssembleError> {
        // used to check duplication
        let mut result: Option<(usize, usize)> = None;

        for (level_index, control_flow_item) in self.control_flow_items.iter().enumerate() {
            if let Some(variable_index) = control_flow_item
                .local_variable_names_include_params
                .iter()
                .position(|name| name == local_variable_name)
            {
                let reversed_index = self.control_flow_items.len() - level_index - 1;

                if result.is_none() {
                    result.replace((reversed_index, variable_index));
                } else {
                    return Err(AssembleError::new(&format!(
                        "Local variable name duplicated: {}",
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

struct AssembleResultForDataNodes {
    read_only_data_entries: Vec<InitedDataEntry>,
    read_write_data_entries: Vec<InitedDataEntry>,
    uninit_data_entries: Vec<UninitDataEntry>,
}

fn assemble_data_nodes(
    data_nodes: &[DataNode],
    // read_write_data_nodes: &[CanonicalDataNode],
    // uninit_data_nodes: &[CanonicalDataNode],
) -> Result<AssembleResultForDataNodes, AssembleError> {
    //     let read_only_data_entries = read_only_data_nodes
    //         .iter()
    //         .map(|node| match &node.data_kind {
    //             DataDetailNode::ReadOnly(src) => InitedDataEntry {
    //                 memory_data_type: src.memory_data_type,
    //                 data: src.value.clone(),
    //                 length: src.length,
    //                 align: src.align,
    //             },
    //             _ => unreachable!(),
    //         })
    //         .collect::<Vec<_>>();
    //
    //     let read_write_data_entries = read_write_data_nodes
    //         .iter()
    //         .map(|node| match &node.data_kind {
    //             DataDetailNode::ReadWrite(src) => InitedDataEntry {
    //                 memory_data_type: src.memory_data_type,
    //                 data: src.value.clone(),
    //                 length: src.length,
    //                 align: src.align,
    //             },
    //             _ => unreachable!(),
    //         })
    //         .collect::<Vec<_>>();
    //
    //     let uninit_data_entries = uninit_data_nodes
    //         .iter()
    //         .map(|node| match &node.data_kind {
    //             DataDetailNode::Uninit(src) => UninitDataEntry {
    //                 memory_data_type: src.memory_data_type,
    //                 length: src.length,
    //                 align: src.align,
    //             },
    //             _ => unreachable!(),
    //         })
    //         .collect::<Vec<_>>();

    Ok(AssembleResultForDataNodes {
        read_only_data_entries: vec![],
        read_write_data_entries: vec![],
        uninit_data_entries: vec![],
    })

    // Ok(AssembleResultForDataNodes{
    //     read_only_data_entries,
    //     read_write_data_entries,
    //     uninit_data_entries,
    // })
}

struct AssembleResultForDependencies {
    import_module_entries: Vec<ImportModuleEntry>,
    import_module_ids: Vec<String>,
    external_library_entries: Vec<ExternalLibraryEntry>,
    external_library_ids: Vec<String>,
}

fn assemble_dependencies(
    import_module_entries: &[ImportModuleEntry],
    external_library_entries: &[ExternalLibraryEntry],
) -> Result<AssembleResultForDependencies, AssembleError> {
    let import_module_ids: Vec<String> = import_module_entries
        .iter()
        .map(|item| item.name.to_owned())
        .collect();
    let external_library_ids: Vec<String> = external_library_entries
        .iter()
        .map(|item| item.name.to_owned())
        .collect();

    Ok(AssembleResultForDependencies {
        import_module_entries: import_module_entries.to_vec(),
        import_module_ids,
        external_library_entries: external_library_entries.to_vec(),
        external_library_ids,
    })
}

struct AssembleResultForImportNodes {
    import_function_entries: Vec<ImportFunctionEntry>,
    import_function_ids: Vec<String>,
    import_data_entries: Vec<ImportDataEntry>,
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
    let mut import_function_ids: Vec<String> = vec![];

    let mut import_data_entries: Vec<ImportDataEntry> = vec![];
    let mut import_read_only_data_ids: Vec<String> = vec![];
    let mut import_read_write_data_ids: Vec<String> = vec![];
    let mut import_uninit_data_ids: Vec<String> = vec![];

    let get_module_index_by_name = |module_ids: &[String], name: &str| -> usize {
        module_ids.iter().position(|id| id == name).unwrap()
    };

    for import_node in import_nodes {
        match import_node {
            ImportNode::Function(import_function_node) => {
                let (module_name, name_path) =
                    get_module_name_and_name_path(&import_function_node.full_name);
                let (_, function_name) = get_namespace_and_identifier(name_path);
                let import_module_index = get_module_index_by_name(import_module_ids, module_name);

                // use the alias name if it presents.
                let identifier = if let Some(alias_name) = &import_function_node.alias_name {
                    alias_name.to_owned()
                } else {
                    function_name.to_owned()
                };

                // add function identifier
                import_function_ids.push(identifier);

                // get type index
                let type_index = find_or_create_function_type_index(
                    type_entries,
                    &import_function_node.params,
                    &import_function_node.results,
                );

                // add import function entry
                let import_function_entry =
                    ImportFunctionEntry::new(name_path.to_owned(), import_module_index, type_index);
                import_function_entries.push(import_function_entry);
            }
            ImportNode::Data(import_data_node) => {
                let (module_name, name_path) =
                    get_module_name_and_name_path(&import_data_node.full_name);
                let (_, data_name) = get_namespace_and_identifier(name_path);
                let import_module_index = get_module_index_by_name(import_module_ids, module_name);

                // use the alias name if it presents.
                let identifier = if let Some(alias_name) = &import_data_node.alias_name {
                    alias_name.to_owned()
                } else {
                    data_name.to_owned()
                };

                // add data id
                match import_data_node.data_section_type {
                    DataSectionType::ReadOnly => {
                        import_read_only_data_ids.push(identifier);
                    }
                    DataSectionType::ReadWrite => {
                        import_read_write_data_ids.push(identifier);
                    }
                    DataSectionType::Uninit => {
                        import_uninit_data_ids.push(identifier);
                    }
                };

                // add import data entry
                let import_data_entry = ImportDataEntry::new(
                    name_path.to_owned(),
                    import_module_index,
                    import_data_node.data_section_type,
                    import_data_node.data_type,
                );

                import_data_entries.push(import_data_entry);
            }
        }
    }

    let result = AssembleResultForImportNodes {
        import_function_entries,
        import_data_entries,
        //
        import_function_ids,
        import_read_only_data_ids,
        import_read_write_data_ids,
        import_uninit_data_ids,
    };

    Ok(result)
}

struct AssembleResultForExternalNode {
    external_function_entries: Vec<ExternalFunctionEntry>,
    external_function_ids: Vec<String>,
}

fn assemble_external_nodes(
    external_library_ids: &[String],
    external_nodes: &[ExternalNode],
    type_entries: &mut Vec<TypeEntry>,
) -> Result<AssembleResultForExternalNode, AssembleError> {
    let mut external_function_entries: Vec<ExternalFunctionEntry> = vec![];
    let mut external_function_ids: Vec<String> = vec![];

    let get_library_index_by_name = |library_ids: &[String], name: &str| -> usize {
        library_ids.iter().position(|id| id == name).unwrap()
    };

    for external_node in external_nodes {
        match external_node {
            ExternalNode::Function(external_function_node) => {
                let (library_name, function_name) =
                    get_library_name_and_identifier(&external_function_node.full_name);
                let external_library_index =
                    get_library_index_by_name(external_library_ids, library_name);

                // use the alias name if it presents.
                let identifier = if let Some(alias_name) = &external_function_node.alias_name {
                    alias_name.to_owned()
                } else {
                    function_name.to_owned()
                };

                // add external function id
                external_function_ids.push(identifier);

                let results = if let Some(result) = external_function_node.result {
                    vec![result]
                } else {
                    vec![]
                };

                // get type index
                let type_index = find_or_create_function_type_index(
                    type_entries,
                    &external_function_node.params,
                    &results,
                );

                // build ExternalFunctionEntry
                let external_function_entry = ExternalFunctionEntry::new(
                    function_name.to_owned(),
                    external_library_index,
                    type_index,
                );

                external_function_entries.push(external_function_entry);
            }
            ExternalNode::Data(_) => {
                return Err(AssembleError {
                    message: "Does not support external data yet.".to_owned(),
                })
            }
        }
    }

    Ok(AssembleResultForExternalNode {
        external_function_entries,
        external_function_ids,
    })
}

#[cfg(test)]
mod tests {

    use anc_parser_asm::parser::parse_from_str;
    use pretty_assertions::assert_eq;

    use crate::entry::ImageCommonEntry;

    use super::{assemble_module_node, create_virtual_dependency_module};

    fn assemble(source: &str) -> ImageCommonEntry {
        let import_module_entries = vec![create_virtual_dependency_module()];
        let external_library_entries = vec![];

        let module_node = parse_from_str(source).unwrap();
        let image_common_entry = assemble_module_node(
            &module_node,
            "hello_module",
            &import_module_entries,
            &external_library_entries,
        )
        .unwrap();

        image_common_entry
    }

    #[test]
    fn test_assemble_module() {
        let entry = assemble(
            r#"
        fn add(left:i32, right:i32) -> i32 nop()
        "#,
        );

        println!("{:#?}", entry);
    }

    //     #[test]
    //     fn test_assemble() {
    //         let submodule_sources = &[
    //             r#"
    //         (module $myapp
    //             (runtime_version "1.0")
    //             (constructor $init)
    //             (destructor $exit)
    //             (depend
    //                 (module $math share "math" "1.0")
    //                 (library $libc system "libc.so.6")
    //             )
    //             (data $SUCCESS (read_only i64 0))
    //             (data $FAILURE (read_only i64 1))
    //             (function $entry
    //                 (result i64)
    //                 (code
    //                     (call $package::utils::add
    //                         (extcall $package::utils::getuid)
    //                         (data.load32_i32 $package::utils::seed)
    //                     )
    //                     (data.load64_i64 $SUCCESS)
    //                 )
    //             )
    //             (function $init
    //                 (code
    //                     (data.store32 $package::utils::buf (i32.imm 0))
    //                 )
    //             )
    //             (function $exit
    //                 (code
    //                     nop
    //                 )
    //             )
    //         )
    //         "#,
    //             r#"
    //         (module $myapp::utils
    //             (import $math
    //                 (function $wrap_add "wrap::add"
    //                     (params i32 i32)
    //                     (result i32)
    //                 )
    //                 (data $seed "seed" read_only i32)
    //             )
    //             (external $libc
    //                 (function $getuid "getuid" (result i32))
    //             )
    //             (data $buf (read_write bytes h"11131719" 2))
    //             (function export $add
    //                 (param $left i32) (param $right i32)
    //                 (result i64)
    //                 (code
    //                     (call $wrap_add
    //                         (local.load32_i32 $left)
    //                         (local.load32_i32 $right)
    //                     )
    //                 )
    //             )
    //         )
    //         "#,
    //         ];
    //
    //         let submodule_nodes = submodule_sources
    //             .iter()
    //             .map(|source| {
    //                 let mut chars = source.chars();
    //                 let mut char_iter = PeekableIterator::new(&mut chars, 3);
    //                 let all_tokens = lex(&mut char_iter).unwrap();
    //                 let effective_tokens = filter(&all_tokens);
    //                 let mut token_iter = effective_tokens.into_iter();
    //                 let mut peekable_token_iter = PeekableIterator::new(&mut token_iter, 2);
    //                 parse(&mut peekable_token_iter, None).unwrap()
    //             })
    //             .collect::<Vec<_>>();
    //
    //         let merged_module_node =
    //             merge_and_canonicalize_submodule_nodes(&submodule_nodes, None, None).unwrap();
    //         let (module_entry, _) = assemble_merged_module_node(&merged_module_node).unwrap();
    //
    //         assert_eq!(module_entry.name, "myapp");
    //         assert_eq!(module_entry.runtime_version, EffectiveVersion::new(1, 0));
    //
    //         assert_eq!(module_entry.import_function_count, 1);
    //         assert_eq!(module_entry.import_read_only_data_count, 1);
    //         assert_eq!(module_entry.import_read_write_data_count, 0);
    //         assert_eq!(module_entry.import_uninit_data_count, 0);
    //
    //         assert_eq!(module_entry.constructor_function_public_index, Some(2));
    //         assert_eq!(module_entry.destructor_function_public_index, Some(3));
    //
    //         // check import entries
    //
    //         assert_eq!(
    //             module_entry.import_module_entries,
    //             vec![ImportModuleEntry {
    //                 name: "math".to_owned(),
    //                 module_share_type: ModuleShareType::Share,
    //                 // version_major: 1,
    //                 // version_minor: 0
    //                 module_version: EffectiveVersion::new(1, 0)
    //             }]
    //         );
    //
    //         assert_eq!(
    //             module_entry.import_function_entries,
    //             vec![ImportFunctionEntry {
    //                 name_path: "wrap::add".to_owned(),
    //                 import_module_index: 0,
    //                 type_index: 0
    //             }]
    //         );
    //
    //         assert_eq!(
    //             module_entry.import_data_entries,
    //             vec![ImportDataEntry {
    //                 name_path: "seed".to_owned(),
    //                 import_module_index: 0,
    //                 data_section_type: DataSectionType::ReadOnly,
    //                 memory_data_type: MemoryDataType::I32
    //             }]
    //         );
    //
    //         // check external entries
    //
    //         assert_eq!(
    //             module_entry.external_library_entries,
    //             vec![ExternalLibraryEntry {
    //                 name: "libc.so.6".to_owned(),
    //                 external_library_type: ExternalLibraryType::System
    //             }]
    //         );
    //
    //         assert_eq!(
    //             module_entry.external_function_entries,
    //             vec![ExternalFunctionEntry {
    //                 name: "getuid".to_owned(),
    //                 external_library_index: 0,
    //                 type_index: 1
    //             }]
    //         );
    //
    //         // check function entries
    //         assert_eq!(module_entry.function_entries.len(), 4);
    //
    //         let function_entry0 = &module_entry.function_entries[0];
    //         assert_eq!(function_entry0.type_index, 2);
    //         assert_eq!(function_entry0.local_list_index, 0);
    //         assert_eq!(
    //             format_bytecode_as_text(&function_entry0.code),
    //             "\
    // 0x0000  04 0b 00 00  00 00 00 00    extcall           idx:0
    // 0x0008  02 03 00 00  00 00 00 00    data.load32_i32   off:0x00  idx:0
    // 0x0010  00 0b 00 00  04 00 00 00    call              idx:4
    // 0x0018  00 03 00 00  01 00 00 00    data.load64_i64   off:0x00  idx:1
    // 0x0020  00 0a                       end"
    //         );
    //
    //         let function_entry1 = &module_entry.function_entries[1];
    //         assert_eq!(function_entry1.type_index, 3);
    //         assert_eq!(function_entry1.local_list_index, 0);
    //         assert_eq!(
    //             format_bytecode_as_text(&function_entry1.code),
    //             "\
    // 0x0000  80 01 00 00  00 00 00 00    i32.imm           0x00000000
    // 0x0008  09 03 00 00  03 00 00 00    data.store32      off:0x00  idx:3
    // 0x0010  00 0a                       end"
    //         );
    //
    //         let function_entry2 = &module_entry.function_entries[2];
    //         assert_eq!(function_entry2.type_index, 3);
    //         assert_eq!(function_entry2.local_list_index, 0);
    //         assert_eq!(
    //             format_bytecode_as_text(&function_entry2.code),
    //             "\
    // 0x0000  00 01                       nop
    // 0x0002  00 0a                       end"
    //         );
    //
    //         let function_entry3 = &module_entry.function_entries[3];
    //         assert_eq!(function_entry3.type_index, 4);
    //         assert_eq!(function_entry3.local_list_index, 1);
    //         assert_eq!(
    //             format_bytecode_as_text(&function_entry3.code),
    //             "\
    // 0x0000  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
    // 0x0008  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
    // 0x0010  00 0b 00 00  00 00 00 00    call              idx:0
    // 0x0018  00 0a                       end"
    //         );
    //
    //         // check data entries
    //
    //         assert_eq!(
    //             module_entry.read_only_data_entries,
    //             vec![
    //                 InitedDataEntry {
    //                     memory_data_type: MemoryDataType::I64,
    //                     data: 0u64.to_le_bytes().to_vec(),
    //                     length: 8,
    //                     align: 8
    //                 },
    //                 InitedDataEntry {
    //                     memory_data_type: MemoryDataType::I64,
    //                     data: 1u64.to_le_bytes().to_vec(),
    //                     length: 8,
    //                     align: 8
    //                 },
    //             ]
    //         );
    //
    //         assert_eq!(
    //             module_entry.read_write_data_entries,
    //             vec![InitedDataEntry {
    //                 memory_data_type: MemoryDataType::Bytes,
    //                 data: vec![0x11u8, 0x13, 0x17, 0x19],
    //                 length: 4,
    //                 align: 2
    //             },]
    //         );
    //
    //         assert_eq!(module_entry.uninit_data_entries.len(), 0);
    //
    //         // check type entries
    //
    //         assert_eq!(
    //             module_entry.type_entries,
    //             vec![
    //                 TypeEntry {
    //                     params: vec![DataType::I32, DataType::I32],
    //                     results: vec![DataType::I32]
    //                 },
    //                 TypeEntry {
    //                     params: vec![],
    //                     results: vec![DataType::I32]
    //                 },
    //                 TypeEntry {
    //                     params: vec![],
    //                     results: vec![DataType::I64]
    //                 },
    //                 TypeEntry {
    //                     params: vec![],
    //                     results: vec![]
    //                 },
    //                 TypeEntry {
    //                     params: vec![DataType::I32, DataType::I32],
    //                     results: vec![DataType::I64]
    //                 },
    //             ]
    //         );
    //
    //         // check local list entries
    //
    //         assert_eq!(
    //             module_entry.local_list_entries,
    //             vec![
    //                 LocalListEntry {
    //                     local_variable_entries: vec![]
    //                 },
    //                 LocalListEntry {
    //                     local_variable_entries: vec![
    //                         LocalVariableEntry {
    //                             memory_data_type: MemoryDataType::I32,
    //                             length: 4,
    //                             align: 4
    //                         },
    //                         LocalVariableEntry {
    //                             memory_data_type: MemoryDataType::I32,
    //                             length: 4,
    //                             align: 4
    //                         }
    //                     ]
    //                 }
    //             ]
    //         );
    //
    //         // check function names
    //
    //         assert_eq!(
    //             module_entry.function_name_entries,
    //             vec![
    //                 FunctionNameEntry {
    //                     name_path: "entry".to_owned(),
    //                     function_public_index: 1,
    //                     export: false
    //                 },
    //                 FunctionNameEntry {
    //                     name_path: "init".to_owned(),
    //                     function_public_index: 2,
    //                     export: false
    //                 },
    //                 FunctionNameEntry {
    //                     name_path: "exit".to_owned(),
    //                     function_public_index: 3,
    //                     export: false
    //                 },
    //                 FunctionNameEntry {
    //                     name_path: "utils::add".to_owned(),
    //                     function_public_index: 4,
    //                     export: true
    //                 },
    //             ]
    //         );
    //
    //         // check data names
    //
    //         assert_eq!(
    //             module_entry.data_name_entries,
    //             vec![
    //                 DataNameEntry {
    //                     name_path: "SUCCESS".to_owned(),
    //                     data_public_index: 1,
    //                     export: false
    //                 },
    //                 DataNameEntry {
    //                     name_path: "FAILURE".to_owned(),
    //                     data_public_index: 2,
    //                     export: false
    //                 },
    //                 DataNameEntry {
    //                     name_path: "utils::buf".to_owned(),
    //                     data_public_index: 3,
    //                     export: false
    //                 }
    //             ]
    //         )
    //     }
}
