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
    ArgumentValue, DataNode, DataSection, DataTypeValuePair, DataValue, DeclareDataType,
    ExpressionNode, ExternalNode, FixedDeclareDataType, FunctionNode, ImportNode, InstructionNode,
    LiteralNumber, LocalVariable, ModuleNode, NamedParameter,
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
    let mut read_only_name_paths: Vec<DataNamePathEntry> = vec![];
    let mut read_only_data_ids: Vec<String> = vec![];

    let mut read_write_name_paths: Vec<DataNamePathEntry> = vec![];
    let mut read_write_data_ids: Vec<String> = vec![];

    let mut uninit_name_paths: Vec<DataNamePathEntry> = vec![];
    let mut uninit_data_ids: Vec<String> = vec![];

    for data_node in data_nodes {
        let name_path = if module_name_path.is_empty() {
            data_node.name.to_owned()
        } else {
            format!("{}::{}", module_name_path, data_node.name)
        };

        let data_name_path_entry = DataNamePathEntry::new(name_path, data_node.export);
        let id = data_node.name.to_owned();

        match data_node.data_section {
            DataSection::ReadOnly(_) => {
                read_only_name_paths.push(data_name_path_entry);
                read_only_data_ids.push(id);
            }
            DataSection::ReadWrite(_) => {
                read_write_name_paths.push(data_name_path_entry);
                read_write_data_ids.push(id);
            }
            DataSection::Uninit(_) => {
                uninit_name_paths.push(data_name_path_entry);
                uninit_data_ids.push(id);
            }
        }
    }

    let mut data_name_path_entries = vec![];
    data_name_path_entries.append(&mut read_only_name_paths);
    data_name_path_entries.append(&mut read_write_name_paths);
    data_name_path_entries.append(&mut uninit_name_paths);

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
    match expression_node {
        ExpressionNode::Group(items) => {
            for item in items {
                emit_expression(
                    function_name,
                    item,
                    identifier_public_index_lookup_table,
                    type_entries,
                    local_variable_list_entries,
                    control_flow_stack,
                    bytecode_writer,
                )?;
            }
        }
        ExpressionNode::Instruction(instruction_node) => emit_instruction(
            function_name,
            instruction_node,
            identifier_public_index_lookup_table,
            type_entries,
            local_variable_list_entries,
            control_flow_stack,
            bytecode_writer,
        )?,
        ExpressionNode::When(when_node) => todo!(),
        ExpressionNode::If(if_node) => todo!(),
        ExpressionNode::For(for_node) => todo!(),
        ExpressionNode::Break(break_node) => todo!(),
        ExpressionNode::Recur(break_node) => todo!(),
    }

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
    let inst_name = &instruction_node.name;
    let args = &instruction_node.positional_args;
    let named_args = &instruction_node.named_args;

    match inst_name.as_str() {
        "nop" => {
            bytecode_writer.write_opcode(Opcode::nop);
        }
        "imm_i32" => {
            bytecode_writer.write_opcode_i32(
                Opcode::imm_i32,
                read_argument_value_as_i32(inst_name, &args[0])?,
            );
        }
        "imm_i64" => {
            bytecode_writer.write_opcode_i64(
                Opcode::imm_i64,
                read_argument_value_as_i64(inst_name, &args[0])?,
            );
        }
        "imm_f32" => {
            bytecode_writer.write_opcode_f32(
                Opcode::imm_f32,
                read_argument_value_as_f32(inst_name, &args[0])?,
            );
        }
        "imm_f64" => {
            bytecode_writer.write_opcode_f64(
                Opcode::imm_f64,
                read_argument_value_as_f64(inst_name, &args[0])?,
            );
        }
        "end" => {
            bytecode_writer.write_opcode(Opcode::end);
        }
        _ => {
            return Err(AssembleError {
                message: format!("Unknown instruction \"{}\".", inst_name),
            })
        }
    }

    Ok(())

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
) -> Result<AssembleResultForDataNodes, AssembleError> {
    let mut read_only_data_entries: Vec<InitedDataEntry> = vec![];
    let mut read_write_data_entries: Vec<InitedDataEntry> = vec![];
    let mut uninit_data_entries: Vec<UninitDataEntry> = vec![];

    for data_node in data_nodes {
        match &data_node.data_section {
            DataSection::ReadOnly(data_type_value_pair) => {
                read_only_data_entries.push(conver_data_type_value_pair_to_inited_data_entry(
                    data_type_value_pair,
                )?);
            }
            DataSection::ReadWrite(data_type_value_pair) => {
                read_write_data_entries.push(conver_data_type_value_pair_to_inited_data_entry(
                    data_type_value_pair,
                )?);
            }
            DataSection::Uninit(fixed_declare_data_type) => uninit_data_entries.push(
                convert_fixed_declare_data_type_to_uninit_data_entry(fixed_declare_data_type),
            ),
        }
    }

    Ok(AssembleResultForDataNodes {
        read_only_data_entries,
        read_write_data_entries,
        uninit_data_entries,
    })
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

    let mut import_read_only_data_entries: Vec<ImportDataEntry> = vec![];
    let mut import_read_only_data_ids: Vec<String> = vec![];

    let mut import_read_write_data_entries: Vec<ImportDataEntry> = vec![];
    let mut import_read_write_data_ids: Vec<String> = vec![];

    let mut import_uninit_data_entries: Vec<ImportDataEntry> = vec![];
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

                // import data entry
                let import_data_entry = ImportDataEntry::new(
                    name_path.to_owned(),
                    import_module_index,
                    import_data_node.data_section_type,
                    import_data_node.data_type,
                );

                // add data id
                match import_data_node.data_section_type {
                    DataSectionType::ReadOnly => {
                        import_read_only_data_entries.push(import_data_entry);
                        import_read_only_data_ids.push(identifier);
                    }
                    DataSectionType::ReadWrite => {
                        import_read_write_data_entries.push(import_data_entry);
                        import_read_write_data_ids.push(identifier);
                    }
                    DataSectionType::Uninit => {
                        import_uninit_data_entries.push(import_data_entry);
                        import_uninit_data_ids.push(identifier);
                    }
                };
            }
        }
    }

    let mut import_data_entries: Vec<ImportDataEntry> = vec![];
    import_data_entries.append(&mut import_read_only_data_entries);
    import_data_entries.append(&mut import_read_write_data_entries);
    import_data_entries.append(&mut import_uninit_data_entries);

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

fn read_data_value_as_i32(data_value: &DataValue) -> Result<u32, AssembleError> {
    match data_value {
        DataValue::I8(v) => Ok(*v as u32),
        DataValue::I16(v) => Ok(*v as u32),
        DataValue::I64(v) => Ok(*v as u32),
        DataValue::I32(v) => Ok(*v),
        DataValue::F64(v) => Err(AssembleError {
            message: format!("Can not convert f64 \"{}\" into i32", v),
        }),
        DataValue::F32(v) => Err(AssembleError {
            message: format!("Can not convert f32 \"{}\" into i32", v),
        }),
        DataValue::String(_) => Err(AssembleError::new("Can not convert string into i32.")),
        DataValue::ByteData(_) => Err(AssembleError::new("Can not convert byte data into i32.")),
        DataValue::List(_) => Err(AssembleError::new("Can not convert list data into i32.")),
    }
}

fn read_data_value_as_i64(data_value: &DataValue) -> Result<u64, AssembleError> {
    match data_value {
        DataValue::I8(v) => Ok(*v as u64),
        DataValue::I16(v) => Ok(*v as u64),
        DataValue::I64(v) => Ok(*v),
        DataValue::I32(v) => Ok(*v as u64),
        DataValue::F64(v) => Err(AssembleError {
            message: format!("Can not convert f64 \"{}\" into i64", v),
        }),
        DataValue::F32(v) => Err(AssembleError {
            message: format!("Can not convert f32 \"{}\" into i64", v),
        }),
        DataValue::String(_) => Err(AssembleError::new("Can not convert string into i64.")),
        DataValue::ByteData(_) => Err(AssembleError::new("Can not convert byte data into i64.")),
        DataValue::List(_) => Err(AssembleError::new("Can not convert list data into i64.")),
    }
}

fn read_data_value_as_f32(data_value: &DataValue) -> Result<f32, AssembleError> {
    match data_value {
        DataValue::I8(v) => Ok(*v as f32),
        DataValue::I16(v) => Ok(*v as f32),
        DataValue::I64(v) => Ok(*v as f32),
        DataValue::I32(v) => Ok(*v as f32),
        DataValue::F64(v) => Ok(*v as f32),
        DataValue::F32(v) => Ok(*v),
        DataValue::String(_) => Err(AssembleError::new("Can not convert string into f32.")),
        DataValue::ByteData(_) => Err(AssembleError::new("Can not convert byte data into f32.")),
        DataValue::List(_) => Err(AssembleError::new("Can not convert list data into f32.")),
    }
}

fn read_data_value_as_f64(data_value: &DataValue) -> Result<f64, AssembleError> {
    match data_value {
        DataValue::I8(v) => Ok(*v as f64),
        DataValue::I16(v) => Ok(*v as f64),
        DataValue::I64(v) => Ok(*v as f64),
        DataValue::I32(v) => Ok(*v as f64),
        DataValue::F64(v) => Ok(*v),
        DataValue::F32(v) => Ok(*v as f64),
        DataValue::String(_) => Err(AssembleError::new("Can not convert string into f64.")),
        DataValue::ByteData(_) => Err(AssembleError::new("Can not convert byte data into f64.")),
        DataValue::List(_) => Err(AssembleError::new("Can not convert list data into f64.")),
    }
}

fn read_data_value_as_bytes(data_value: &DataValue) -> Result<Vec<u8>, AssembleError> {
    let bytes = match data_value {
        DataValue::I8(v) => v.to_le_bytes().to_vec(),
        DataValue::I16(v) => v.to_le_bytes().to_vec(),
        DataValue::I64(v) => v.to_le_bytes().to_vec(),
        DataValue::I32(v) => v.to_le_bytes().to_vec(),
        DataValue::F64(v) => v.to_le_bytes().to_vec(),
        DataValue::F32(v) => v.to_le_bytes().to_vec(),
        DataValue::String(v) => v.as_bytes().to_vec(),
        DataValue::ByteData(v) => v.to_owned(),
        DataValue::List(v) => {
            let mut bytes: Vec<u8> = vec![];
            for item in v {
                let mut b = read_data_value_as_bytes(item)?;
                bytes.append(&mut b);
            }
            bytes
        }
    };

    Ok(bytes)
}

fn read_argument_value_as_i16(inst_name: &str, v: &ArgumentValue) -> Result<u16, AssembleError> {
    match v {
        ArgumentValue::Identifier(_) => Err(AssembleError{
            message: format!("Expect an i16 value for parameter of instruction \"{}\", but it is actually an identifier.", inst_name)
        }),
        ArgumentValue::LiteralNumber(literal_number) => match literal_number {
            LiteralNumber::I8(v) => Ok(*v as u16),
            LiteralNumber::I16(v) => Ok(*v),
            LiteralNumber::I32(v) => Ok(*v as u16),
            LiteralNumber::I64(v) => Ok(*v as u16),
            LiteralNumber::F32(_) | LiteralNumber::F64(_)  => Err(AssembleError{
                message: format!("Expect an i16 value for parameter of instruction \"{}\", but it is actually a floating-point number.", inst_name)
            }),
        },
        ArgumentValue::Expression(_) => Err(AssembleError{
            message: format!("Expect an i16 value for parameter of instruction \"{}\", but it is actually an expression.", inst_name)
        }),
    }
}

fn read_argument_value_as_i32(inst_name: &str, v: &ArgumentValue) -> Result<u32, AssembleError> {
    match v {
        ArgumentValue::Identifier(_) => Err(AssembleError{
            message: format!("Expect an i32 value for parameter of instruction \"{}\", but it is actually an identifier.", inst_name)
        }),
        ArgumentValue::LiteralNumber(literal_number) => match literal_number {
            LiteralNumber::I8(v) => Ok(*v as u32),
            LiteralNumber::I16(v) => Ok(*v as u32),
            LiteralNumber::I32(v) => Ok(*v),
            LiteralNumber::I64(v) => Ok(*v as u32),
            LiteralNumber::F32(_) | LiteralNumber::F64(_)  => Err(AssembleError{
                message: format!("Expect an i32 value for parameter of instruction \"{}\", but it is actually a floating-point number.", inst_name)
            }),
        },
        ArgumentValue::Expression(_) => Err(AssembleError{
            message: format!("Expect an i32 value for parameter of instruction \"{}\", but it is actually an expression.", inst_name)
        }),
    }
}

fn read_argument_value_as_i64(inst_name: &str, v: &ArgumentValue) -> Result<u64, AssembleError> {
    match v {
        ArgumentValue::Identifier(_) => Err(AssembleError{
            message: format!("Expect an i64 value for parameter of instruction \"{}\", but it is actually an identifier.", inst_name)
        }),
        ArgumentValue::LiteralNumber(literal_number) => match literal_number {
            LiteralNumber::I8(v) => Ok(*v as u64),
            LiteralNumber::I16(v) => Ok(*v as u64),
            LiteralNumber::I32(v) => Ok(*v as u64),
            LiteralNumber::I64(v) => Ok(*v),
            LiteralNumber::F32(_) | LiteralNumber::F64(_)  => Err(AssembleError{
                message: format!("Expect an i64 value for parameter of instruction \"{}\", but it is actually a floating-point number.", inst_name)
            }),
        },
        ArgumentValue::Expression(_) => Err(AssembleError{
            message: format!("Expect an i64 value for parameter of instruction \"{}\", but it is actually an expression.", inst_name)
        }),
    }
}

fn read_argument_value_as_f32(inst_name: &str, v: &ArgumentValue) -> Result<f32, AssembleError> {
    match v {
        ArgumentValue::Identifier(_) => Err(AssembleError{
            message: format!("Expect an f32 value for parameter of instruction \"{}\", but it is actually an identifier.", inst_name)
        }),
        ArgumentValue::LiteralNumber(literal_number) => match literal_number {
            LiteralNumber::I8(v) => Ok(*v as f32),
            LiteralNumber::I16(v) => Ok(*v as f32),
            LiteralNumber::I32(v) => Ok(*v as f32),
            LiteralNumber::I64(v) => Ok(*v as f32),
            LiteralNumber::F32(v)  => Ok(*v),
            LiteralNumber::F64(v)  => Ok(*v as f32),
        },
        ArgumentValue::Expression(_) => Err(AssembleError{
            message: format!("Expect an f32 value for parameter of instruction \"{}\", but it is actually an expression.", inst_name)
        }),
    }
}

fn read_argument_value_as_f64(inst_name: &str, v: &ArgumentValue) -> Result<f64, AssembleError> {
    match v {
        ArgumentValue::Identifier(_) => Err(AssembleError{
            message: format!("Expect an f64 value for parameter of instruction \"{}\", but it is actually an identifier.", inst_name)
        }),
        ArgumentValue::LiteralNumber(literal_number) => match literal_number {
            LiteralNumber::I8(v) => Ok(*v as f64),
            LiteralNumber::I16(v) => Ok(*v as f64),
            LiteralNumber::I32(v) => Ok(*v as f64),
            LiteralNumber::I64(v) => Ok(*v as f64),
            LiteralNumber::F32(v)  => Ok(*v as f64),
            LiteralNumber::F64(v)  => Ok(*v),
        },
        ArgumentValue::Expression(_) => Err(AssembleError{
            message: format!("Expect an f64 value for parameter of instruction \"{}\", but it is actually an expression.", inst_name)
        }),
    }
}

fn read_argument_value_as_expression<'a>(
    inst_name: &str,
    v: &'a ArgumentValue,
) -> Result<&'a ExpressionNode, AssembleError> {
    match v {
        ArgumentValue::Identifier(_) => Err(AssembleError{
            message: format!("Expect an expression for parameter of instruction \"{}\", but it is actually an identifier.", inst_name)
        }),
        ArgumentValue::LiteralNumber(_) => Err(AssembleError{
            message: format!("Expect an expression for parameter of instruction \"{}\", but it is actually a number literal.", inst_name)
        }),

        ArgumentValue::Expression(exp) => Ok(exp.as_ref()),
    }
}

fn read_argument_value_as_identifer<'a>(
    inst_name: &str,
    v: &'a ArgumentValue,
) -> Result<&'a String, AssembleError> {
    match v {
        ArgumentValue::Identifier(id) => Ok(id),
        ArgumentValue::LiteralNumber(_) => Err(AssembleError{
            message: format!("Expect an identifier for parameter of instruction \"{}\", but it is actually a number literal.", inst_name)
        }),
        ArgumentValue::Expression(_) => Err(AssembleError{
            message: format!("Expect an identifier for parameter of instruction \"{}\", but it is actually an expression.", inst_name)
        }),
    }
}

fn conver_data_type_value_pair_to_inited_data_entry(
    data_type_value_pair: &DataTypeValuePair,
) -> Result<InitedDataEntry, AssembleError> {
    let entry = match data_type_value_pair.data_type {
        DeclareDataType::I64 => {
            InitedDataEntry::from_i64(read_data_value_as_i64(&data_type_value_pair.value)?)
        }
        DeclareDataType::I32 => {
            InitedDataEntry::from_i32(read_data_value_as_i32(&data_type_value_pair.value)?)
        }
        DeclareDataType::F64 => {
            InitedDataEntry::from_f64(read_data_value_as_f64(&data_type_value_pair.value)?)
        }
        DeclareDataType::F32 => {
            InitedDataEntry::from_f32(read_data_value_as_f32(&data_type_value_pair.value)?)
        }
        DeclareDataType::Bytes(opt_align) => InitedDataEntry::from_bytes(
            read_data_value_as_bytes(&data_type_value_pair.value)?,
            opt_align.unwrap_or(1) as u16,
        ),
        DeclareDataType::FixedBytes(length, opt_align) => {
            let mut bytes = read_data_value_as_bytes(&data_type_value_pair.value)?;
            bytes.resize(length, 0);
            InitedDataEntry::from_bytes(bytes, opt_align.unwrap_or(1) as u16)
        }
    };
    Ok(entry)
}

fn convert_fixed_declare_data_type_to_uninit_data_entry(
    fixed_declare_data_type: &FixedDeclareDataType,
) -> UninitDataEntry {
    match fixed_declare_data_type {
        FixedDeclareDataType::I64 => UninitDataEntry::from_i64(),
        FixedDeclareDataType::I32 => UninitDataEntry::from_i32(),
        FixedDeclareDataType::F64 => UninitDataEntry::from_f64(),
        FixedDeclareDataType::F32 => UninitDataEntry::from_f32(),
        FixedDeclareDataType::FixedBytes(length, opt_align) => {
            UninitDataEntry::from_bytes(*length as u32, opt_align.unwrap_or(1) as u16)
        }
    }
}

#[cfg(test)]
mod tests {

    use anc_image::{
        bytecode_reader::format_bytecode_as_text,
        entry::{
            DataNamePathEntry, FunctionEntry, FunctionNamePathEntry, InitedDataEntry,
            LocalVariableEntry, LocalVariableListEntry, TypeEntry, UninitDataEntry,
        },
    };
    use anc_isa::{MemoryDataType, OperandDataType};
    use anc_parser_asm::parser::parse_from_str;
    use pretty_assertions::assert_eq;

    use crate::entry::ImageCommonEntry;

    use super::{assemble_module_node, create_virtual_dependency_module};

    fn assemble(source: &str) -> ImageCommonEntry {
        let import_module_entries = vec![create_virtual_dependency_module()];
        let external_library_entries = vec![];

        let module_node = match parse_from_str(source) {
            Ok(node) => node,
            Err(parser_error) => {
                panic!("{}", parser_error.with_source(source));
            }
        };

        let image_common_entry = assemble_module_node(
            &module_node,
            "hello_module",
            &import_module_entries,
            &external_library_entries,
        )
        .unwrap();

        image_common_entry
    }

    fn codegen(source: &str) -> String {
        let entry = assemble(source);
        format_bytecode_as_text(&entry.function_entries[0].code)
    }

    #[test]
    fn test_assemble_import() {
        // todo
    }

    #[test]
    fn test_assemble_external() {
        // todo
    }

    #[test]
    fn test_assemble_data() {
        let entry = assemble(
            r#"
data foo:i32 = 42
uninit data bar:i64
pub readonly data msg:byte[] = "Hello world!"
pub data buf:byte[16] = h"11 13 17 19"
pub data obj:byte[align=8] = [
    "foo", 0_i8,
    [0x23_i32, 0x29_i32],
    [0x31_i16, 0x37_i16],
    0xff_i64
]"#,
        );

        // read-only data entries
        assert_eq!(entry.read_only_data_entries.len(), 1);

        assert_eq!(
            &entry.read_only_data_entries[0],
            &InitedDataEntry {
                memory_data_type: MemoryDataType::Bytes,
                data: vec![72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 33,],
                length: 12,
                align: 1,
            }
        );

        // read-write data entries
        assert_eq!(entry.read_write_data_entries.len(), 3);

        assert_eq!(
            &entry.read_write_data_entries[0],
            &InitedDataEntry {
                memory_data_type: MemoryDataType::I32,
                data: vec![42, 0, 0, 0],
                length: 4,
                align: 4
            }
        );

        assert_eq!(
            &entry.read_write_data_entries[1],
            &InitedDataEntry {
                memory_data_type: MemoryDataType::Bytes,
                data: vec![17, 19, 23, 25, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,],
                length: 16,
                align: 1
            }
        );

        assert_eq!(
            &entry.read_write_data_entries[2],
            &InitedDataEntry {
                memory_data_type: MemoryDataType::Bytes,
                data: vec![
                    102, 111, 111, 0, 35, 0, 0, 0, 41, 0, 0, 0, 49, 0, 55, 0, 255, 0, 0, 0, 0, 0,
                    0, 0,
                ],
                length: 24,
                align: 8
            }
        );

        // uninit data entries
        assert_eq!(entry.uninit_data_entries.len(), 1);

        assert_eq!(
            &entry.uninit_data_entries[0],
            &UninitDataEntry {
                memory_data_type: MemoryDataType::I64,
                length: 8,
                align: 8
            }
        );

        // data name path
        assert_eq!(
            &entry.data_name_path_entries,
            &[
                DataNamePathEntry::new("msg".to_owned(), true),
                DataNamePathEntry::new("foo".to_owned(), false),
                DataNamePathEntry::new("buf".to_owned(), true),
                DataNamePathEntry::new("obj".to_owned(), true),
                DataNamePathEntry::new("bar".to_owned(), false),
            ]
        );
    }

    #[test]
    fn test_assemble_function() {
        let entry = assemble(
            r#"
            fn add(left:i32, right:i32) -> i32
                nop()
            fn fib(num:i32) -> i32
                [count:i32, sum:i32]
                nop()
            pub fn inc(num:i32) -> i32
                [temp:i32]
                nop()
            "#,
        );

        // type entries
        assert_eq!(entry.type_entries.len(), 3);

        assert_eq!(
            &entry.type_entries[0],
            &TypeEntry {
                params: vec![],
                results: vec![]
            }
        );

        assert_eq!(
            &entry.type_entries[1],
            &TypeEntry {
                params: vec![OperandDataType::I32, OperandDataType::I32],
                results: vec![OperandDataType::I32]
            }
        );

        assert_eq!(
            &entry.type_entries[2],
            &TypeEntry {
                params: vec![OperandDataType::I32],
                results: vec![OperandDataType::I32]
            }
        );

        // local variable list entries
        assert_eq!(entry.local_variable_list_entries.len(), 3);

        assert_eq!(
            &entry.local_variable_list_entries[0],
            &LocalVariableListEntry::new(vec![])
        );

        assert_eq!(
            &entry.local_variable_list_entries[1],
            &LocalVariableListEntry::new(vec![
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_i32(),
            ])
        );

        assert_eq!(
            &entry.local_variable_list_entries[2],
            &LocalVariableListEntry::new(vec![
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_i32(),
            ])
        );

        // function entries
        assert_eq!(entry.function_entries.len(), 3);

        assert_eq!(
            &entry.function_entries[0],
            &FunctionEntry {
                type_index: 1,
                local_list_index: 1,
                code: vec![0, 1, 192, 3]
            }
        );

        assert_eq!(
            &entry.function_entries[1],
            &FunctionEntry {
                type_index: 2,
                local_list_index: 2,
                code: vec![0, 1, 192, 3]
            }
        );

        assert_eq!(
            &entry.function_entries[2],
            &FunctionEntry {
                type_index: 2,
                local_list_index: 1,
                code: vec![0, 1, 192, 3]
            }
        );

        // function name paths
        assert_eq!(
            &entry.function_name_path_entries,
            &vec![
                FunctionNamePathEntry::new("add".to_owned(), false),
                FunctionNamePathEntry::new("fib".to_owned(), false),
                FunctionNamePathEntry::new("inc".to_owned(), true),
            ]
        );
    }

    #[test]
    fn test_assemble_expression_group() {
        assert_eq!(
            codegen(
                r#"
        fn foo() {
            imm_i32(0x11)
            imm_i64(0x13)
            imm_f32(3.142)
            imm_f64(2.718)
            nop()
        }
        "#
            ),
            "\
0x0000  40 01 00 00  11 00 00 00    imm_i32           0x00000011
0x0008  41 01 00 00  13 00 00 00    imm_i64           low:0x00000013  high:0x00000000
        00 00 00 00
0x0014  42 01 00 00  87 16 49 40    imm_f32           0x40491687
0x001c  43 01 00 00  58 39 b4 c8    imm_f64           low:0xc8b43958  high:0x4005be76
        76 be 05 40
0x0028  00 01                       nop
0x002a  c0 03                       end"
        );
    }

    #[test]
    fn test_assemble_expression_when() {
        // todo
    }

    #[test]
    fn test_assemble_expression_if() {
        // todo
    }

    #[test]
    fn test_assemble_expression_for() {
        // todo
    }

    #[test]
    fn test_assemble_expression_break_fn() {
        // todo
    }

    #[test]
    fn test_assemble_expression_recur_fn() {
        // todo
    }

    #[test]
    fn test_assemble_instruction_base() {
        // todo
    }

    #[test]
    fn test_assemble_instruction_local_load_store() {
        // todo
    }

    #[test]
    fn test_assemble_instruction_data_load_store() {
        // todo
    }

    #[test]
    fn test_assemble_instruction_heap_load_store() {
        // todo
    }

    #[test]
    fn test_assemble_instruction_conversion() {
        // todo
    }

    #[test]
    fn test_assemble_instruction_comparison() {
        // todo
    }

    #[test]
    fn test_assemble_instruction_arithmetic() {
        // todo
    }

    #[test]
    fn test_assemble_instruction_bitwise() {
        // todo
    }

    #[test]
    fn test_assemble_instruction_math() {
        // todo
    }

    #[test]
    fn test_assemble_instruction_calling() {
        // todo
    }

    #[test]
    fn test_assemble_instruction_host() {
        // todo
    }
}
