// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use anc_assembly::ast::{
    ArgumentValue, BreakNode, DataNode, DataSection, DataTypeValuePair, DataValue, DeclareDataType,
    ExpressionNode, ExternalNode, FixedDeclareDataType, FunctionNode, ImportNode, InstructionNode,
    LiteralNumber, LocalVariable, ModuleNode, NamedArgument, NamedParameter,
};
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

use crate::{entry::ImageCommonEntry, AssembleError};

// the value of the stub for the instruction parameter 'next_inst_offset'
const INSTRUCTION_STUB_VALUE: u32 = 0;

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

/// The virtual which named "module" must be the first of imported modules.
pub fn create_virtual_dependency_module() -> ImportModuleEntry {
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
        let mut function_public_index: usize = 0;
        for function_ids in [
            &identifier_source.import_function_ids,
            &identifier_source.function_ids,
        ] {
            functions.extend(function_ids.iter().map(|id| {
                let pair = NameIndexPair {
                    id: id.to_owned(),
                    public_index: function_public_index,
                };
                function_public_index += 1;
                pair
            }));
        }

        // fill data ids
        let mut data_public_index: usize = 0;
        for data_ids in [
            &identifier_source.import_read_only_data_ids,
            &identifier_source.import_read_write_data_ids,
            &identifier_source.import_uninit_data_ids,
            &identifier_source.read_only_data_ids,
            &identifier_source.read_write_data_ids,
            &identifier_source.uninit_data_ids,
        ] {
            datas.extend(data_ids.iter().map(|id| {
                let pair = NameIndexPair {
                    id: id.to_owned(),
                    public_index: data_public_index,
                };
                data_public_index += 1;
                pair
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
        let type_index = find_or_create_function_type_index(
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
            local_variable_list_index,
            code,
        });
    }

    Ok(function_entries)
}

/// function type = params + results
fn find_or_create_import_function_type_index(
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
fn find_or_create_function_type_index(
    type_entries: &mut Vec<TypeEntry>,
    named_params: &[NamedParameter],
    results: &[OperandDataType],
) -> usize {
    let params: Vec<OperandDataType> = named_params.iter().map(|item| item.data_type).collect();
    find_or_create_import_function_type_index(type_entries, &params, results)
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
    control_flow_stack.pop_layer(&mut bytecode_writer, 0);

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
        ExpressionNode::When(when_node) => {
            //  asm: `when testing [locals] consequence`
            // code:  block_nez (param local_variable_list_index:i32, next_inst_offset:i32)

            // assemble 'testing'
            emit_expression(
                function_name,
                &when_node.testing,
                identifier_public_index_lookup_table,
                type_entries,
                local_variable_list_entries,
                control_flow_stack,
                bytecode_writer,
            )?;

            // local variable index and names
            let local_variable_list_index = find_or_create_local_variable_list_index(
                local_variable_list_entries,
                &[],
                &when_node.locals,
            );

            let local_variable_names =
                build_local_variable_names_by_params_and_local_variables(&[], &when_node.locals);

            // write inst 'block_nez'
            let address_of_block_nez = bytecode_writer.write_opcode_i32_i32(
                Opcode::block_nez,
                local_variable_list_index as u32,
                INSTRUCTION_STUB_VALUE,
            );

            // push flow stack
            control_flow_stack.push_layer(
                address_of_block_nez,
                ControlFlowKind::BlockNez,
                local_variable_names,
            );

            // assemble 'consequent'
            emit_expression(
                function_name,
                &when_node.consequence,
                identifier_public_index_lookup_table,
                type_entries,
                local_variable_list_entries,
                control_flow_stack,
                bytecode_writer,
            )?;

            // write inst 'end'
            bytecode_writer.write_opcode(Opcode::end);
            let address_next_to_end = bytecode_writer.get_addr();

            // pop flow stck and fill stubs
            control_flow_stack.pop_layer(bytecode_writer, address_next_to_end);
        }
        ExpressionNode::If(if_node) => {
            //  asm: `if params -> results tesing consequence alternative`
            // code: block_alt (param type_index:i32, next_inst_offset:i32)
            // code: break_alt (param next_inst_offset:i32)

            // assemble node 'test'
            emit_expression(
                function_name,
                &if_node.testing,
                identifier_public_index_lookup_table,
                type_entries,
                local_variable_list_entries,
                control_flow_stack,
                bytecode_writer,
            )?;

            // type index
            let type_index =
                find_or_create_function_type_index(type_entries, &[], &if_node.results);

            // local variable list index
            let local_variable_list_index =
                find_or_create_local_variable_list_index(local_variable_list_entries, &[], &[]);

            // local variable names
            let local_variable_names =
                build_local_variable_names_by_params_and_local_variables(&[], &[]);

            // write inst 'block_alt'
            let address_of_block_alt = bytecode_writer.write_opcode_i32_i32_i32(
                Opcode::block_alt,
                type_index as u32,
                local_variable_list_index as u32,
                INSTRUCTION_STUB_VALUE,
            );

            // push flow stack
            control_flow_stack.push_layer(
                address_of_block_alt,
                ControlFlowKind::BlockAlt,
                local_variable_names,
            );

            // assemble node 'consequent'
            emit_expression(
                function_name,
                &if_node.consequence,
                identifier_public_index_lookup_table,
                type_entries,
                local_variable_list_entries,
                control_flow_stack,
                bytecode_writer,
            )?;

            // write inst 'break_alt'
            let address_of_break_alt = bytecode_writer.write_opcode_i16_i32(
                Opcode::break_alt,
                0,                      // reversed_index
                INSTRUCTION_STUB_VALUE, // next_inst_offset
            );

            // add break item
            control_flow_stack.add_break(BreakType::BreakAlt, address_of_break_alt, 0);

            // assemble node 'alternate'
            emit_expression(
                function_name,
                &if_node.alternative,
                identifier_public_index_lookup_table,
                type_entries,
                local_variable_list_entries,
                control_flow_stack,
                bytecode_writer,
            )?;

            // write inst 'end'
            bytecode_writer.write_opcode(Opcode::end);
            let address_next_to_end = bytecode_writer.get_addr();

            // pop flow stack and fill stubs
            control_flow_stack.pop_layer(bytecode_writer, address_next_to_end);
        }
        ExpressionNode::Block(block_node) => {
            //  asm: `for param_values -> results [locals] body`
            // code: block (param type_index:i32, local_variable_list_index:i32)

            // assemble param values
            let values = block_node
                .param_values
                .iter()
                .map(|item| item.value.as_ref())
                .collect::<Vec<_>>();

            for value in values {
                emit_expression(
                    function_name,
                    &value,
                    identifier_public_index_lookup_table,
                    type_entries,
                    local_variable_list_entries,
                    control_flow_stack,
                    bytecode_writer,
                )?;
            }

            let named_params = block_node
                .param_values
                .iter()
                .map(|item| NamedParameter {
                    name: item.name.clone(),
                    data_type: item.data_type,
                })
                .collect::<Vec<NamedParameter>>();

            // type index
            let type_index = find_or_create_function_type_index(
                type_entries,
                &named_params,
                &block_node.results,
            );

            // local variable index
            let local_variable_list_index = find_or_create_local_variable_list_index(
                local_variable_list_entries,
                &named_params,
                &block_node.locals,
            );

            // local variable names
            let local_variable_names = build_local_variable_names_by_params_and_local_variables(
                &named_params,
                &block_node.locals,
            );

            // write inst 'block'
            let address_of_block = bytecode_writer.write_opcode_i32_i32(
                Opcode::block,
                type_index as u32,
                local_variable_list_index as u32,
            );

            // push flow stack
            control_flow_stack.push_layer(
                address_of_block,
                ControlFlowKind::Block,
                local_variable_names,
            );

            // assemble node 'body'
            emit_expression(
                function_name,
                &block_node.body,
                identifier_public_index_lookup_table,
                type_entries,
                local_variable_list_entries,
                control_flow_stack,
                bytecode_writer,
            )?;

            // write inst 'end'
            bytecode_writer.write_opcode(Opcode::end);
            let address_next_to_end = bytecode_writer.get_addr();

            // pop flow stack and fill stubs
            control_flow_stack.pop_layer(bytecode_writer, address_next_to_end);
        }
        ExpressionNode::Break(break_node) => {
            // asm:
            // `break (value0, value1, ...)`
            // // `break_if testing (value0, value1, ...)`
            // `break_fn (value0, value1, ...)`
            //
            // code:
            // break_ (param reversed_index:i16, next_inst_offset:i32)
            // // break_nez (param reversed_index:i16, next_inst_offset:i32)

            let (opcode, reversed_index, next_inst_offset, expressions) = match break_node {
                BreakNode::Break(expressions) => {
                    let reversed_index =
                        control_flow_stack.get_reversed_index_to_the_nearest_block();
                    (
                        Opcode::break_,
                        reversed_index,
                        INSTRUCTION_STUB_VALUE,
                        expressions,
                    )
                }
                // BreakNode::BreakIf(_, expressions) => {
                //     let reversed_index =
                //         control_flow_stack.get_reversed_index_to_the_nearest_block();
                //     (
                //         Opcode::break_nez,
                //         reversed_index,
                //         INSTRUCTION_STUB_VALUE,
                //         expressions,
                //     )
                // }
                BreakNode::BreakFn(expressions) => {
                    let reversed_index = control_flow_stack.get_reversed_index_to_function();
                    (Opcode::break_, reversed_index, 0, expressions)
                }
            };

            for expression in expressions {
                emit_expression(
                    function_name,
                    expression,
                    identifier_public_index_lookup_table,
                    type_entries,
                    local_variable_list_entries,
                    control_flow_stack,
                    bytecode_writer,
                )?;
            }

            // if let BreakNode::BreakIf(testing, _) = break_node {
            //     emit_expression(
            //         function_name,
            //         testing,
            //         identifier_public_index_lookup_table,
            //         type_entries,
            //         local_variable_list_entries,
            //         control_flow_stack,
            //         bytecode_writer,
            //     )?;
            // }

            // write inst 'break'
            let address_of_break = bytecode_writer.write_opcode_i16_i32(
                opcode,
                reversed_index as u16,
                next_inst_offset,
            );

            control_flow_stack.add_break(BreakType::Break, address_of_break, reversed_index);
        }
        ExpressionNode::Recur(break_node) => {
            // asm:
            // `recur (value0, value1, ...)`
            // // `recur_if testing (value0, value1, ...)`
            // `recur_fn (value0, value1, ...)`
            //
            // code:
            // recur (param reversed_index:i16, start_inst_offset:i32)
            // recur_nez (param reversed_index:i16, start_inst_offset:i32)

            let (opcode, expressions) = match break_node {
                BreakNode::Break(expressions) => (Opcode::recur, expressions),
                // BreakNode::BreakIf(_, expressions) => (Opcode::recur_nez, expressions),
                BreakNode::BreakFn(expressions) => (Opcode::recur, expressions),
            };

            for expression in expressions {
                emit_expression(
                    function_name,
                    expression,
                    identifier_public_index_lookup_table,
                    type_entries,
                    local_variable_list_entries,
                    control_flow_stack,
                    bytecode_writer,
                )?;
            }

            // if let BreakNode::BreakIf(testing, _) = break_node {
            //     emit_expression(
            //         function_name,
            //         testing,
            //         identifier_public_index_lookup_table,
            //         type_entries,
            //         local_variable_list_entries,
            //         control_flow_stack,
            //         bytecode_writer,
            //     )?;
            // }

            // 'start_inst_offset' is the address of the next instruction after 'block'.
            // 'start_inst_offset' = 'address_of_recur' - 'address_of_block' - INSTRUCTION_LENGTH('block')
            //
            // NOTE that the 'recur' instruction requires 4-byte align
            let address_of_recur = bytecode_writer.get_addr_with_align();
            let (reversed_index, start_inst_offset) =
                control_flow_stack.get_recur_to_nearest_block(address_of_recur);

            // write inst 'recur'
            //
            // note that there is no stub for the `recur` instruction.
            bytecode_writer.write_opcode_i16_i32(
                opcode,
                reversed_index as u16,
                start_inst_offset as u32,
            );
        }
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
            //  asm: nop()
            // code: ()

            let opcode = Opcode::from_name(inst_name);
            bytecode_writer.write_opcode(opcode);
        }
        "imm_i32" => {
            // asm:
            // imm_i32(literal_i32)
            // imm_i64(literal_i64)
            // imm_f32(literal_f32)
            // imm_f64(literal_f64)
            //
            // code:
            // imm_i32(param immediate_number:i32)
            // imm_i64(param number_low:i32, number_high:i32)
            // imm_f32(param number:i32)
            // imm_f64(param number_low:i32, number_high:i32)

            let num = read_argument_value_as_i32(inst_name, &args[0])?;
            bytecode_writer.write_opcode_i32(Opcode::imm_i32, num);
        }
        "imm_i64" => {
            let num = read_argument_value_as_i64(inst_name, &args[0])?;
            bytecode_writer.write_opcode_i64(Opcode::imm_i64,num);
        }
        "imm_f32" => {
            let num = read_argument_value_as_f32(inst_name, &args[0])?;
            bytecode_writer.write_opcode_f32(Opcode::imm_f32,num);
        }
        "imm_f64" => {
            let num = read_argument_value_as_f64(inst_name, &args[0])?;
            bytecode_writer.write_opcode_f64(Opcode::imm_f64, num);
        }
        "local_load_i64" | "local_load_i32_s" | "local_load_i32_u" | "local_load_i16_s"
        | "local_load_i16_u" | "local_load_i8_s" | "local_load_i8_u" | "local_load_f32"
        | "local_load_f64" | /* host */ "host_addr_local" => {
            //  asm: (identifier, offset=literal_i16)
            // code: (param reversed_index:i16 offset_bytes:i16 local_variable_index:i16)

            let identifier = read_argument_value_as_identifer(inst_name, &args[0])?;
            let (reversed_index, local_variable_index) = control_flow_stack
                .get_local_variable_reversed_index_and_variable_index_by_name(identifier)?;
            let offset = match get_named_argument_value(named_args, "offset") {
                Some(v) => read_argument_value_as_i16(inst_name, v)?,
                None => 0,
            };
            let opcode = Opcode::from_name(inst_name);

            bytecode_writer.write_opcode_i16_i16_i16(
                opcode,
                reversed_index as u16,
                offset,
                local_variable_index as u16,
            );
        }
        "local_store_i64" | "local_store_i32" | "local_store_i16" | "local_store_i8"
        | "local_store_f64" | "local_store_f32" => {
            //  asm: (identifier, value:i64, offset=literal_i16)
            // code: (param reversed_index:i16 offset_bytes:i16 local_variable_index:i16) (operand value:i64)

            let identifier = read_argument_value_as_identifer(inst_name, &args[0])?;
            let (reversed_index, local_variable_index) = control_flow_stack
                .get_local_variable_reversed_index_and_variable_index_by_name(identifier)?;
            let offset = match get_named_argument_value(named_args, "offset") {
                Some(v) => read_argument_value_as_i16(inst_name, v)?,
                None => 0,
            };
            let opcode = Opcode::from_name(inst_name);

            let value_expression_node = read_argument_value_as_expression(inst_name, &args[1])?;
            emit_expression(
                function_name,
                value_expression_node,
                identifier_public_index_lookup_table,
                type_entries,
                local_variable_list_entries,
                control_flow_stack,
                bytecode_writer,
            )?;

            bytecode_writer.write_opcode_i16_i16_i16(
                opcode,
                reversed_index as u16,
                offset,
                local_variable_index as u16,
            );
        }
        "local_load_extend_i64"
        | "local_load_extend_i32_s"
        | "local_load_extend_i32_u"
        | "local_load_extend_i16_s"
        | "local_load_extend_i16_u"
        | "local_load_extend_i8_s"
        | "local_load_extend_i8_u"
        | "local_load_extend_f64"
        | "local_load_extend_f32" |
        /* host */
        "host_addr_local_extend"=> {
            //  asm: (identifier, offset:i64)
            // code: (param reversed_index:i16 local_variable_index:i32) (operand offset_bytes:i64)

            let identifier = read_argument_value_as_identifer(inst_name, &args[0])?;
            let (reversed_index, local_variable_index) = control_flow_stack
                .get_local_variable_reversed_index_and_variable_index_by_name(identifier)?;
            let opcode = Opcode::from_name(inst_name);

            let offset_expression_node = read_argument_value_as_expression(inst_name, &args[1])?;
            emit_expression(function_name, offset_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            bytecode_writer.write_opcode_i16_i32(
                opcode,
                reversed_index as u16,
                local_variable_index as u32,
            );
        }
        "local_store_extend_i64"
        | "local_store_extend_i32"
        | "local_store_extend_i16"
        | "local_store_extend_i8"
        | "local_store_extend_f64"
        | "local_store_extend_f32" => {
            //  asm: (identifier, offset:i64, value:i64)
            // code: (param reversed_index:i16 local_variable_index:i32) (operand offset_bytes:i64 value:i64)

            let identifier = read_argument_value_as_identifer(inst_name, &args[0])?;
            let (reversed_index, local_variable_index) = control_flow_stack
                .get_local_variable_reversed_index_and_variable_index_by_name(identifier)?;
            let opcode = Opcode::from_name(inst_name);

            let offset_expression_node = read_argument_value_as_expression(inst_name, &args[1])?;
            emit_expression(function_name, offset_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            let value_expression_node = read_argument_value_as_expression(inst_name, &args[2])?;
            emit_expression(function_name, value_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            bytecode_writer.write_opcode_i16_i32(
                opcode,
                reversed_index as u16,
                local_variable_index as u32,
            );
        }
        "data_load_i64" | "data_load_i32_s" | "data_load_i32_u" | "data_load_i16_s"
        | "data_load_i16_u" | "data_load_i8_s" | "data_load_i8_u" | "data_load_f32"
        | "data_load_f64" |
        /* host */
        "host_addr_data"=> {
            //  asm: (identifier, offset=literal_i16)
            // code: (param offset_bytes:i16 data_public_index:i32)

            let identifier = read_argument_value_as_identifer(inst_name, &args[0])?;
            let data_public_index = identifier_public_index_lookup_table.get_data_public_index(identifier)?;

            let offset = match get_named_argument_value(named_args, "offset") {
                Some(v) => read_argument_value_as_i16(inst_name, v)?,
                None => 0,
            };
            let opcode = Opcode::from_name(inst_name);

            bytecode_writer.write_opcode_i16_i32(
                opcode,
                offset,
                data_public_index as u32,
            );

        }
        "data_store_i64" | "data_store_i32" | "data_store_i16" | "data_store_i8"
        | "data_store_f64" | "data_store_f32" => {
            //  asm: (identifier, value:i64, offset=literal_i16)
            // code: (param offset_bytes:i16 data_public_index:i32) (operand value:i64)

            let identifier = read_argument_value_as_identifer(inst_name, &args[0])?;
            let data_public_index = identifier_public_index_lookup_table.get_data_public_index(identifier)?;

            let offset = match get_named_argument_value(named_args, "offset") {
                Some(v) => read_argument_value_as_i16(inst_name, v)?,
                None => 0,
            };
            let opcode = Opcode::from_name(inst_name);

            let value_expression_node = read_argument_value_as_expression(inst_name, &args[1])?;
            emit_expression(function_name, value_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            bytecode_writer.write_opcode_i16_i32(
                opcode,
                offset,
                data_public_index as u32,
            );
        }
        "data_load_extend_i64" |
        "data_load_extend_i32_s" |
        "data_load_extend_i32_u" |
        "data_load_extend_i16_s" |
        "data_load_extend_i16_u" |
        "data_load_extend_i8_s" |
        "data_load_extend_i8_u" |
        "data_load_extend_f32" |
        "data_load_extend_f64" |
        /* host */
        "host_addr_data_extend" => {
            //  asm: (identifier, offset:i64)
            // code: (param data_public_index:i32) (operand offset_bytes:i64)

            let identifier = read_argument_value_as_identifer(inst_name, &args[0])?;
            let data_public_index = identifier_public_index_lookup_table.get_data_public_index(identifier)?;

            let opcode = Opcode::from_name(inst_name);

            let offset_expression_node = read_argument_value_as_expression(inst_name, &args[1])?;
            emit_expression(function_name, offset_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            bytecode_writer.write_opcode_i32(
                opcode,
                data_public_index as u32,
            );
        }
        "data_store_extend_i64" |
        "data_store_extend_i32" |
        "data_store_extend_i16" |
        "data_store_extend_i8" |
        "data_store_extend_f64" |
        "data_store_extend_f32" => {
            //  asm: (identifier, offset:i64, value:i64)
            // code: (param data_public_index:i32) (operand offset_bytes:i64 value:i64)

            let identifier = read_argument_value_as_identifer(inst_name, &args[0])?;
            let data_public_index = identifier_public_index_lookup_table.get_data_public_index(identifier)?;

            let opcode = Opcode::from_name(inst_name);

            let offset_expression_node = read_argument_value_as_expression(inst_name, &args[1])?;
            emit_expression(function_name, offset_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;
            let value_expression_node = read_argument_value_as_expression(inst_name, &args[2])?;
            emit_expression(function_name, value_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            bytecode_writer.write_opcode_i32(
                opcode,
                data_public_index as u32,
            );
        }
        "memory_load_i64" |
        "memory_load_i32_s" |
        "memory_load_i32_u" |
        "memory_load_i16_s" |
        "memory_load_i16_u" |
        "memory_load_i8_s" |
        "memory_load_i8_u" |
        "memory_load_f32" |
        "memory_load_f64" |
        /* host */ "host_addr_memory" => {
            //  asm: (addr:i64, offset=literal_i16)
            // code: (param offset_bytes:i16) (operand heap_addr:i64)
            let offset = match get_named_argument_value(named_args, "offset") {
                Some(v) => read_argument_value_as_i16(inst_name, v)?,
                None => 0,
            };
            let opcode = Opcode::from_name(inst_name);

            let addr_expression_node = read_argument_value_as_expression(inst_name, &args[0])?;
            emit_expression(function_name, addr_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            bytecode_writer.write_opcode_i16(
                opcode,
                offset,
            );
        }
        "memory_store_i64" |
        "memory_store_i32" |
        "memory_store_i16" |
        "memory_store_i8" |
        "memory_store_f64" |
        "memory_store_f32" => {
            //  asm: (addr:i64, value:i64, offset=literal_i16)
            // code: (param offset_bytes:i16) (operand heap_addr:i64 value:i64)
            let offset = match get_named_argument_value(named_args, "offset") {
                Some(v) => read_argument_value_as_i16(inst_name, v)?,
                None => 0,
            };
            let opcode = Opcode::from_name(inst_name);

            let addr_expression_node = read_argument_value_as_expression(inst_name, &args[0])?;
            emit_expression(function_name, addr_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            let value_expression_node = read_argument_value_as_expression(inst_name, &args[1])?;
            emit_expression(function_name, value_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            bytecode_writer.write_opcode_i16(
                opcode,
                offset,
            );
        }
        "memory_fill" | "memory_copy" |
        /* host */
        "host_copy_from_memory" | "host_copy_to_memory" | "host_external_memory_copy" => {
            // asm:
            // memory_fill(addr:i64, value:i8, count:i64)
            // memory_copy(dst_addr:i64, src_addr:i64, count:i64)
            // host_copy_from_memory(dst_pointer:i64, src_addr:i64, count:i64)
            // host_copy_to_memory(dst_addr:i64, src_pointer:i64, count:i64)
            //
            // code:
            // memory_fill() (operand addr:i64 value:i8 count:i64)
            // memory_copy() (operand dst_addr:i64 src_addr:i64 count:i64)
            // host_copy_from_memory() (operand dst_pointer:i64 src_addr:i64 count:i64)
            // host_copy_to_memory() (operand dst_addr:i64 src_pointer:i64 count:i64)
            let one_expression_node = read_argument_value_as_expression(inst_name, &args[0])?;
            emit_expression(function_name, one_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            let two_expression_node = read_argument_value_as_expression(inst_name, &args[1])?;
            emit_expression(function_name, two_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            let three_expression_node = read_argument_value_as_expression(inst_name, &args[2])?;
            emit_expression(function_name, three_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            let opcode = Opcode::from_name(inst_name);
            bytecode_writer.write_opcode(opcode);

        }
        "memory_capacity" => {
            //  asm: memory_capacity()
            // code: memory_capacity()
            bytecode_writer.write_opcode(Opcode::memory_capacity);
        }
        "memory_resize" => {
            //  asm: memory_resize(pages:i64)
            // code: memory_resize() (operand pages:i64)
            let pages_expression_node = read_argument_value_as_expression(inst_name, &args[0])?;
            emit_expression(function_name, pages_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            bytecode_writer.write_opcode(Opcode::memory_resize);
        }
        /* unary operations */
        "truncate_i64_to_i32" |
        "extend_i32_s_to_i64" |
        "extend_i32_u_to_i64" |
        "demote_f64_to_f32" |
        "promote_f32_to_f64" |
        "convert_f32_to_i32_s" |
        "convert_f32_to_i32_u" |
        "convert_f64_to_i32_s" |
        "convert_f64_to_i32_u" |
        "convert_f32_to_i64_s" |
        "convert_f32_to_i64_u" |
        "convert_f64_to_i64_s" |
        "convert_f64_to_i64_u" |
        "convert_i32_s_to_f32" |
        "convert_i32_u_to_f32" |
        "convert_i64_s_to_f32" |
        "convert_i64_u_to_f32" |
        "convert_i32_s_to_f64" |
        "convert_i32_u_to_f64" |
        "convert_i64_s_to_f64" |
        "convert_i64_u_to_f64" |
        "eqz_i32" |
        "nez_i32" |
        "eqz_i64" |
        "nez_i64" |
        "not" |
        "count_leading_zeros_i32" |
        "count_leading_ones_i32" |
        "count_trailing_zeros_i32" |
        "count_ones_i32" |
        "count_leading_zeros_i64" |
        "count_leading_ones_i64" |
        "count_trailing_zeros_i64" |
        "count_ones_i64" |
        "abs_i32" |
        "neg_i32" |
        "abs_i64" |
        "neg_i64" |
        "abs_f32" |
        "neg_f32" |
        "sqrt_f32" |
        "ceil_f32" |
        "floor_f32" |
        "round_half_away_from_zero_f32" |
        "round_half_to_even_f32" |
        "trunc_f32" |
        "fract_f32" |
        "cbrt_f32" |
        "exp_f32" |
        "exp2_f32" |
        "ln_f32" |
        "log2_f32" |
        "log10_f32" |
        "sin_f32" |
        "cos_f32" |
        "tan_f32" |
        "asin_f32" |
        "acos_f32" |
        "atan_f32" |
        "abs_f64" |
        "neg_f64" |
        "sqrt_f64" |
        "ceil_f64" |
        "floor_f64" |
        "round_half_away_from_zero_f64" |
        "round_half_to_even_f64" |
        "trunc_f64" |
        "fract_f64" |
        "cbrt_f64" |
        "exp_f64" |
        "exp2_f64" |
        "ln_f64" |
        "log2_f64" |
        "log10_f64" |
        "sin_f64" |
        "cos_f64" |
        "tan_f64" |
        "asin_f64" |
        "acos_f64" |
        "atan_f64" => {
            //  asm: (num:*)
            // code: () (operand num:*)
            let num_expression_node = read_argument_value_as_expression(inst_name, &args[0])?;
            emit_expression(function_name, num_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            let opcode = Opcode::from_name(inst_name);
            bytecode_writer.write_opcode(opcode);
        }
        /* binary operations */
        "eq_i32" |
        "ne_i32" |
        "lt_i32_s" |
        "lt_i32_u" |
        "gt_i32_s" |
        "gt_i32_u" |
        "le_i32_s" |
        "le_i32_u" |
        "ge_i32_s" |
        "ge_i32_u" |
        "eq_i64" |
        "ne_i64" |
        "lt_i64_s" |
        "lt_i64_u" |
        "gt_i64_s" |
        "gt_i64_u" |
        "le_i64_s" |
        "le_i64_u" |
        "ge_i64_s" |
        "ge_i64_u" |
        "eq_f32" |
        "ne_f32" |
        "lt_f32" |
        "gt_f32" |
        "le_f32" |
        "ge_f32" |
        "eq_f64" |
        "ne_f64" |
        "lt_f64" |
        "gt_f64" |
        "le_f64" |
        "ge_f64" |
        "add_i32" |
        "sub_i32" |
        "mul_i32" |
        "div_i32_s" |
        "div_i32_u" |
        "rem_i32_s" |
        "rem_i32_u" |
        "add_i64" |
        "sub_i64" |
        "mul_i64" |
        "div_i64_s" |
        "div_i64_u" |
        "rem_i64_s" |
        "rem_i64_u" |
        "add_f32" |
        "sub_f32" |
        "mul_f32" |
        "div_f32" |
        "add_f64" |
        "sub_f64" |
        "mul_f64" |
        "div_f64" |
        "and" |
        "or" |
        "xor" |
        "shift_left_i32" |
        "shift_right_i32_s" |
        "shift_right_i32_u" |
        "rotate_left_i32" |
        "rotate_right_i32" |
        "shift_left_i64" |
        "shift_right_i64_s" |
        "shift_right_i64_u" |
        "rotate_left_i64" |
        "rotate_right_i64" |
        "copysign_f32" |
        "min_f32" |
        "max_f32" |
        "pow_f32" |
        "log_f32" |
        "copysign_f64" |
        "min_f64" |
        "max_f64" |
        "pow_f64" |
        "log_f64" => {
            //  asm: (left:*, right:*)
            // code: () (operand left:* right:*)
            let left_expression_node = read_argument_value_as_expression(inst_name, &args[0])?;
            emit_expression(function_name, left_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            let right_expression_node = read_argument_value_as_expression(inst_name, &args[1])?;
            emit_expression(function_name, right_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            let opcode = Opcode::from_name(inst_name);
            bytecode_writer.write_opcode(opcode);

        }
        "add_imm_i32" |
        "sub_imm_i32" |
        "add_imm_i64" |
        "sub_imm_i64" => {
            //  asm: (imm:literal_i16, number:*)
            // code: (param imm:i16) (operand number:*)
            let imm = read_argument_value_as_i16(inst_name, &args[0])?;
            let num_expression_node = read_argument_value_as_expression(inst_name, &args[1])?;
            emit_expression(function_name, num_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;

            let opcode = Opcode::from_name(inst_name);
            bytecode_writer.write_opcode_i16(opcode, imm);
        }
        "call" | "extcall" => {
            // asm: (identifier, value0, value1, ...)
            // code: call(param function_public_index:i32) (operand args...)
            // code: extcall(param external_function_index:i32) (operand args...)
            let identifier = read_argument_value_as_identifer(inst_name, &args[0])?;
            let public_index = if inst_name == "call" {
                identifier_public_index_lookup_table.get_function_public_index(identifier)?
            }else {
                identifier_public_index_lookup_table.get_external_function_index(identifier)?
            };
            let opcode = Opcode::from_name(inst_name);

            for arg in &args[1..] {
                let arg_expression_node = read_argument_value_as_expression(inst_name, arg)?;
                emit_expression(function_name, arg_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;
            }

            bytecode_writer.write_opcode_i32(opcode, public_index as u32);
        }
        "dyncall" => {
            // asm: (fn_pub_index:i32, value0, value1, ...)
            // code: dyncall() (operand function_public_index:i32, args...)
            for arg in args {
                let arg_expression_node = read_argument_value_as_expression(inst_name, arg)?;
                emit_expression(function_name, arg_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;
            }

            bytecode_writer.write_opcode(Opcode::dyncall);
        }
        "envcall" => {
            // asm: (env_call_number:liter_i32, value0, value1, ...)
            // code: envcall(param envcall_num:i32) (operand args...)
            let num = read_argument_value_as_i32(inst_name, &args[0])?;

            for arg in &args[1..] {
                let arg_expression_node = read_argument_value_as_expression(inst_name, arg)?;
                emit_expression(function_name, arg_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;
            }

            bytecode_writer.write_opcode_i32(Opcode::envcall, num);
        }
        "syscall" => {
            // asm: (syscall_num:i32, value0, value1, ...)
            // code: syscall() (operand args..., syscall_num:i32, params_count: i32)
            let num = read_argument_value_as_i32(inst_name, &args[0])?;
            let params_count = args.len()-1;

            for arg in &args[1..] {
                let arg_expression_node = read_argument_value_as_expression(inst_name, arg)?;
                emit_expression(function_name, arg_expression_node, identifier_public_index_lookup_table, type_entries, local_variable_list_entries, control_flow_stack, bytecode_writer)?;
            }

            bytecode_writer.write_opcode_i32(Opcode::imm_i32, num);
            bytecode_writer.write_opcode_i32(Opcode::imm_i32, params_count as u32);
            bytecode_writer.write_opcode(Opcode::syscall);
        }
        "get_function" | "host_addr_function"=> {
            // asm: (identifier)
            // code: (param function_public_index:i32)
            let identifier = read_argument_value_as_identifer(inst_name, &args[0])?;
            let function_public_index =identifier_public_index_lookup_table.get_function_public_index(identifier)?;
            let opcode = Opcode::from_name(inst_name);
            bytecode_writer.write_opcode_i32(opcode, function_public_index as u32);
        }
        "panic" => {
            // asm: panic(code:literal_i32)
            // code: panic(param reason_code:u32)
            let num = read_argument_value_as_i32(inst_name, &args[0])?;
            bytecode_writer.write_opcode_i32(Opcode::panic, num);
        }
        _ => {
            return Err(AssembleError {
                message: format!("Encounters unknown instruction \"{}\" in the function \"{}\".", inst_name, function_name),
            })
        }
    }

    Ok(())
}

/**
 * XiaoXuan Core instruction set includes the following instructions
 * containing the "next_inst_offset" parameter:
 *
 * - block_alt (param type_index:i32, next_inst_offset:i32)
 * - block_nez (param local_variable_list_index:i32, next_inst_offset:i32)
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
 * The structure "ControlFlowStack" is designed to implement the above purpose.
 *
 * Note:
 *
 * 1. Generating the "recur*" instruction does not require
 *    inserting stubs because the value of the parameter "start_inst_offset" can
 *    be obtained immediately through the structure "ControlFlowStack".
 *
 * 2. If the target layer of "break" is "function", no stub needs to be inserted,
 *    and the "ControlFlowStack" is not needed because the "next_inst_offset" in
 *    this case is directly ignored by the VM.
 *
 * 3. If the target layer of "recur" is "function", no stub needs to be inserted,
 *    and the "ControlFlowStack" is not needed because the "start_inst_offset" in
 *    this case is directly ignored by the VM.
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
    // this layer is necessary for calculating the layer index.
    //
    // NO stub.
    Function,

    // for expression: 'block'
    //
    // bytecode:
    // block (opcode:i16 padding:i16 type_index:i32, local_variable_list_index:i32)
    //
    // NO stub.
    Block,

    // for expression: 'when'
    //
    // bytecode:
    // block_nez (opcode:i16 padding:i16 local_variable_list_index:i32 next_inst_offset:i32)
    //
    // stub: next_inst_offset
    BlockNez,

    // for expression: 'if'
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
    /// - if the target layer is "function", the `break` does not need stub.
    ///
    /// the "break item" would be only inserted to corresponding layer.
    pub fn add_break(&mut self, break_type: BreakType, address: usize, reversed_index: usize) {
        // get_control_flow_item_by_reversed_index
        let control_flow_item = {
            let idx = self.control_flow_items.len() - reversed_index - 1;
            &mut self.control_flow_items[idx]
        };

        if control_flow_item.control_flow_kind == ControlFlowKind::Function {
            // when the target layer is function, the instruction 'break' does not need stub,
            // because the parameter 'next_inst_offset' is ignored.
        } else {
            control_flow_item.break_items.push(BreakItem {
                break_type,
                address,
            });
        }
    }

    /// pops layer and fills all stubs
    pub fn pop_layer(&mut self, bytecode_writer: &mut BytecodeWriter, address_next_to_end: usize) {
        // pop flow stack
        let control_flow_item = self.control_flow_items.pop().unwrap();

        // patch 'next_inst_offset' of the instruction 'block_nez'.
        if control_flow_item.control_flow_kind == ControlFlowKind::BlockNez {
            let addr_of_block_nez = control_flow_item.address;
            let next_inst_offset = (address_next_to_end - addr_of_block_nez) as u32;
            bytecode_writer.fill_block_nez_stub(addr_of_block_nez, next_inst_offset);
        }

        // patch 'next_inst_offset' of the instructions 'break', 'break_alt', 'break_nez'.
        for break_item in &control_flow_item.break_items {
            let address_of_break = break_item.address;
            let next_inst_offset = (address_next_to_end - address_of_break) as u32;
            bytecode_writer.fill_break_stub(break_item.address, next_inst_offset);

            if break_item.break_type == BreakType::BreakAlt {
                // patch 'next_inst_offset' of the instruction 'block_alt'
                const LENGTH_OF_INSTRUCTION_BREAK_ALT: usize = 8; // 8 bytes
                let addr_of_block_alt = control_flow_item.address;
                let address_next_to_instruction_break_alt =
                    break_item.address + LENGTH_OF_INSTRUCTION_BREAK_ALT;
                let next_inst_offset =
                    (address_next_to_instruction_break_alt - addr_of_block_alt) as u32;
                bytecode_writer.fill_block_alt_stub(addr_of_block_alt, next_inst_offset);
            }
        }
    }

    /// calculate the number of layers to the function
    pub fn get_reversed_index_to_function(&self) -> usize {
        self.control_flow_items.len() - 1
    }

    /// calculate the number of layers to the nearest 'block'
    pub fn get_reversed_index_to_the_nearest_block(&self) -> usize {
        let idx = self
            .control_flow_items
            .iter()
            .rposition(|item| item.control_flow_kind == ControlFlowKind::Block)
            .expect("Can't find \"for\" statement on the control flow stack.");
        self.control_flow_items.len() - idx - 1
    }

    pub fn get_recur_to_nearest_block(
        &self,
        address_of_recur: usize,
    ) -> (
        /* reversed_index */ usize,
        /* start_inst_offset */ usize,
    ) {
        let reversed_index = self.get_reversed_index_to_the_nearest_block();

        // get_block_address(reversed_index);
        let address_of_block = {
            let idx = self.control_flow_items.len() - reversed_index - 1;
            self.control_flow_items[idx].address
        };

        // 'start_inst_offset' is the address of the next instruction after 'block'.
        // 'start_inst_offset' = 'address_of_recur' - 'address_of_block' - INSTRUCTION_LENGTH('block')
        const INSTRUCTION_BLOCK_LENGTH: usize = 12;
        let start_inst_offset = address_of_recur - address_of_block - INSTRUCTION_BLOCK_LENGTH;
        (reversed_index, start_inst_offset)
    }

    /// Get the (reversed_index, variable_index) by variable name.
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
                let type_index = find_or_create_import_function_type_index(
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
                let type_index = find_or_create_import_function_type_index(
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

fn get_named_argument_value<'a>(
    named_args: &'a [NamedArgument],
    name: &str,
) -> Option<&'a ArgumentValue> {
    named_args
        .iter()
        .find(|item| item.name == name)
        .map(|item| &item.value)
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
            DataNamePathEntry, ExternalLibraryEntry, FunctionEntry, FunctionNamePathEntry,
            ImportModuleEntry, InitedDataEntry, LocalVariableEntry, LocalVariableListEntry,
            TypeEntry, UninitDataEntry,
        },
    };
    use anc_isa::{DependencyLocal, ExternalLibraryDependency, OperandDataType};
    use anc_parser_asm::parser::parse_from_str;
    use pretty_assertions::assert_eq;

    use crate::entry::ImageCommonEntry;

    use super::{assemble_module_node, create_virtual_dependency_module};

    fn assemble(source: &str) -> ImageCommonEntry {
        assemble_with_imports_and_externals(source, vec![], vec![])
    }

    fn assemble_with_imports_and_externals(
        source: &str,
        mut import_module_entries_excludes_virtual: Vec<ImportModuleEntry>,
        external_library_entries: Vec<ExternalLibraryEntry>,
    ) -> ImageCommonEntry {
        let mut import_module_entries = vec![create_virtual_dependency_module()];
        import_module_entries.append(&mut import_module_entries_excludes_virtual);

        let module_node = match parse_from_str(source) {
            Ok(node) => node,
            Err(parser_error) => {
                panic!("{}", parser_error.with_source(source));
            }
        };

        assemble_module_node(
            &module_node,
            "mymodule",
            &import_module_entries,
            &external_library_entries,
        )
        .unwrap()
    }

    fn bytecode(source: &str) -> String {
        let entry = assemble(source);
        format_bytecode_as_text(&entry.function_entries[0].code)
    }

    fn bytecode_with_import_and_external(
        source: &str,
        import_module_entries_excludes_virtual: Vec<ImportModuleEntry>,
        external_library_entries: Vec<ExternalLibraryEntry>,
    ) -> String {
        let entry = assemble_with_imports_and_externals(
            source,
            import_module_entries_excludes_virtual,
            external_library_entries,
        );
        format_bytecode_as_text(&entry.function_entries[0].code)
    }

    fn assert_fn(
        source: &str,
        expected_byte_codes: &[&str],
        type_entries: &[TypeEntry],
        local_variable_list_entries: &[LocalVariableListEntry],
    ) {
        let entry = assemble(source);

        for (idx, function_entry) in entry.function_entries.iter().enumerate() {
            assert_eq!(
                format_bytecode_as_text(&function_entry.code),
                expected_byte_codes[idx]
            );
        }

        assert_eq!(&entry.type_entries, type_entries);
        assert_eq!(
            &entry.local_variable_list_entries,
            local_variable_list_entries
        );
    }

    #[test]
    fn test_assemble_import_statement() {
        // todo
    }

    #[test]
    fn test_assemble_external_statement() {
        // todo
    }

    #[test]
    fn test_assemble_data_statement() {
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
            &InitedDataEntry::from_bytes(
                vec![72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100, 33],
                1
            )
        );

        // read-write data entries
        assert_eq!(entry.read_write_data_entries.len(), 3);

        assert_eq!(
            &entry.read_write_data_entries[0],
            &InitedDataEntry::from_i32(42)
        );

        assert_eq!(
            &entry.read_write_data_entries[1],
            &InitedDataEntry::from_bytes(
                vec![17, 19, 23, 25, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                1
            )
        );

        assert_eq!(
            &entry.read_write_data_entries[2],
            &InitedDataEntry::from_bytes(
                vec![
                    102, 111, 111, 0, 35, 0, 0, 0, 41, 0, 0, 0, 49, 0, 55, 0, 255, 0, 0, 0, 0, 0,
                    0, 0,
                ],
                8
            )
        );

        // uninit data entries
        assert_eq!(entry.uninit_data_entries.len(), 1);
        assert_eq!(&entry.uninit_data_entries[0], &UninitDataEntry::from_i64());

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
    fn test_assemble_function_statement() {
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

        assert_eq!(&entry.type_entries[0], &TypeEntry::new(vec![], vec![]));

        assert_eq!(
            &entry.type_entries[1],
            &TypeEntry::new(
                vec![OperandDataType::I32, OperandDataType::I32],
                vec![OperandDataType::I32]
            )
        );

        assert_eq!(
            &entry.type_entries[2],
            &TypeEntry::new(vec![OperandDataType::I32], vec![OperandDataType::I32])
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
            &FunctionEntry::new(1, 1, vec![0, 1, 192, 3])
        );

        assert_eq!(
            &entry.function_entries[1],
            &FunctionEntry::new(2, 2, vec![0, 1, 192, 3])
        );

        assert_eq!(
            &entry.function_entries[2],
            &FunctionEntry::new(2, 1, vec![0, 1, 192, 3])
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
            bytecode(
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
        assert_fn(
            r#"
        fn foo() {
            when
                imm_i32(0x11)   // testing
                imm_i32(0x13)   // consequence
        }"#,
            &["\
0x0000  40 01 00 00  11 00 00 00    imm_i32           0x00000011
0x0008  c6 03 00 00  00 00 00 00    block_nez         local:0   off:0x16
        16 00 00 00
0x0014  40 01 00 00  13 00 00 00    imm_i32           0x00000013
0x001c  c0 03                       end
0x001e  c0 03                       end"],
            &[TypeEntry::new(vec![], vec![])],
            &[LocalVariableListEntry::new(vec![])],
        );

        // test local variables
        assert_fn(
            r#"
        fn foo(num:i32)
            [sum:i32]
        {
            when
                [a:i32, b:i32]
                eqz_i32(local_load_i32_s(num))
                {
                    local_store_i32(a, imm_i32(0x11))
                    local_store_i32(b, imm_i32(0x13))
                    local_store_i32(sum, local_load_i32_s(num))
                }

        }"#,
            &["\
0x0000  81 01 00 00  00 00 00 00    local_load_i32_s  rev:0   off:0x00  idx:0
0x0008  c0 02                       eqz_i32
0x000a  00 01                       nop
0x000c  c6 03 00 00  01 00 00 00    block_nez         local:1   off:0x3e
        3e 00 00 00
0x0018  40 01 00 00  11 00 00 00    imm_i32           0x00000011
0x0020  8a 01 00 00  00 00 00 00    local_store_i32   rev:0   off:0x00  idx:0
0x0028  40 01 00 00  13 00 00 00    imm_i32           0x00000013
0x0030  8a 01 00 00  00 00 01 00    local_store_i32   rev:0   off:0x00  idx:1
0x0038  81 01 01 00  00 00 00 00    local_load_i32_s  rev:1   off:0x00  idx:0
0x0040  8a 01 01 00  00 00 01 00    local_store_i32   rev:1   off:0x00  idx:1
0x0048  c0 03                       end
0x004a  c0 03                       end"],
            &[
                TypeEntry::new(vec![], vec![]),
                TypeEntry::new(vec![OperandDataType::I32], vec![]),
            ],
            &[
                LocalVariableListEntry::new(vec![]),
                LocalVariableListEntry::new(vec![
                    LocalVariableEntry::from_i32(),
                    LocalVariableEntry::from_i32(),
                ]),
            ],
        );
    }

    #[test]
    fn test_assemble_expression_if() {
        assert_fn(
            r#"
        fn foo()
        {
            if
                imm_i32(0x11)   // testing
                imm_i32(0x13)   // consequence
                imm_i32(0x17)   // alternative

        }"#,
            &["\
0x0000  40 01 00 00  11 00 00 00    imm_i32           0x00000011
0x0008  c4 03 00 00  00 00 00 00    block_alt         type:0   local:0   off:0x20
        00 00 00 00  20 00 00 00
0x0018  40 01 00 00  13 00 00 00    imm_i32           0x00000013
0x0020  c5 03 00 00  12 00 00 00    break_alt         off:0x12
0x0028  40 01 00 00  17 00 00 00    imm_i32           0x00000017
0x0030  c0 03                       end
0x0032  c0 03                       end"],
            &[TypeEntry::new(vec![], vec![])],
            &[LocalVariableListEntry::new(vec![])],
        );

        // test results
        assert_fn(
            r#"
        fn foo(num:i32, inc:i32) -> i32
        {
            imm_i32(0x11)
            if -> i32
                eqz_i32(local_load_i32_s(num))                          // testing
                add_i32(local_load_i32_s(inc), imm_i32(0x13))           // consequence
                mul_i32(local_load_i32_s(inc), local_load_i32_s(num))   // alternative

        }"#,
            &["\
0x0000  40 01 00 00  11 00 00 00    imm_i32           0x00000011
0x0008  81 01 00 00  00 00 00 00    local_load_i32_s  rev:0   off:0x00  idx:0
0x0010  c0 02                       eqz_i32
0x0012  00 01                       nop
0x0014  c4 03 00 00  02 00 00 00    block_alt         type:2   local:0   off:0x2c
        00 00 00 00  2c 00 00 00
0x0024  81 01 01 00  00 00 01 00    local_load_i32_s  rev:1   off:0x00  idx:1
0x002c  40 01 00 00  13 00 00 00    imm_i32           0x00000013
0x0034  00 03                       add_i32
0x0036  00 01                       nop
0x0038  c5 03 00 00  1c 00 00 00    break_alt         off:0x1c
0x0040  81 01 01 00  00 00 01 00    local_load_i32_s  rev:1   off:0x00  idx:1
0x0048  81 01 01 00  00 00 00 00    local_load_i32_s  rev:1   off:0x00  idx:0
0x0050  04 03                       mul_i32
0x0052  c0 03                       end
0x0054  c0 03                       end"],
            &[
                TypeEntry::new(vec![], vec![]),
                TypeEntry::new(
                    vec![OperandDataType::I32, OperandDataType::I32],
                    vec![OperandDataType::I32],
                ),
                TypeEntry::new(vec![], vec![OperandDataType::I32]),
            ],
            &[
                LocalVariableListEntry::new(vec![]),
                LocalVariableListEntry::new(vec![
                    LocalVariableEntry::from_i32(),
                    LocalVariableEntry::from_i32(),
                ]),
            ],
        );
    }

    #[test]
    fn test_assemble_expression_block() {
        assert_fn(
            r#"
        fn foo()
        {
            block
                imm_i32(0x11)   // inside the scope
                imm_i32(0x13)   // outside the scope
        }"#,
            &["\
0x0000  c1 03 00 00  00 00 00 00    block             type:0   local:0
        00 00 00 00
0x000c  40 01 00 00  11 00 00 00    imm_i32           0x00000011
0x0014  c0 03                       end
0x0016  00 01                       nop
0x0018  40 01 00 00  13 00 00 00    imm_i32           0x00000013
0x0020  c0 03                       end"],
            &[TypeEntry::new(vec![], vec![])],
            &[LocalVariableListEntry::new(vec![])],
        );

        // test block with group
        assert_fn(
            r#"
        fn foo()
        {
            block {
                // inside the scope
                imm_i32(0x11)
                imm_i32(0x13)
            }
            // outside the scope
            imm_i32(0x17)
        }"#,
            &["\
0x0000  c1 03 00 00  00 00 00 00    block             type:0   local:0
        00 00 00 00
0x000c  40 01 00 00  11 00 00 00    imm_i32           0x00000011
0x0014  40 01 00 00  13 00 00 00    imm_i32           0x00000013
0x001c  c0 03                       end
0x001e  00 01                       nop
0x0020  40 01 00 00  17 00 00 00    imm_i32           0x00000017
0x0028  c0 03                       end"],
            &[TypeEntry::new(vec![], vec![])],
            &[LocalVariableListEntry::new(vec![])],
        );

        // test params and local variables
        assert_fn(
            r#"
        fn foo(a1:i32)
        {
            block(b1:i32 = imm_i32(0x11)) -> i32
            [b2:i32, b3:i32]
            {
                block(c1:i32 = imm_i32(0x13), c2:i32=imm_i32(0x17)) -> (i32,i32)
                [c3:i32, c4:i32, c5:i32]
                {
                    local_load_i32_s(a1) // rindex=2, index=0
                    local_load_i32_s(b1) // rindex=1, index=0
                    local_load_i32_s(b2) // rindex=1, index=1
                    local_load_i32_s(c1) // rindex=0, index=0
                    local_load_i32_s(c2) // rindex=0, index=1
                    local_load_i32_s(c3) // rindex=0, index=2
                }
            }
        }"#,
            &["\
0x0000  40 01 00 00  11 00 00 00    imm_i32           0x00000011
0x0008  c1 03 00 00  02 00 00 00    block             type:2   local:2
        02 00 00 00
0x0014  40 01 00 00  13 00 00 00    imm_i32           0x00000013
0x001c  40 01 00 00  17 00 00 00    imm_i32           0x00000017
0x0024  c1 03 00 00  03 00 00 00    block             type:3   local:3
        03 00 00 00
0x0030  81 01 02 00  00 00 00 00    local_load_i32_s  rev:2   off:0x00  idx:0
0x0038  81 01 01 00  00 00 00 00    local_load_i32_s  rev:1   off:0x00  idx:0
0x0040  81 01 01 00  00 00 01 00    local_load_i32_s  rev:1   off:0x00  idx:1
0x0048  81 01 00 00  00 00 00 00    local_load_i32_s  rev:0   off:0x00  idx:0
0x0050  81 01 00 00  00 00 01 00    local_load_i32_s  rev:0   off:0x00  idx:1
0x0058  81 01 00 00  00 00 02 00    local_load_i32_s  rev:0   off:0x00  idx:2
0x0060  c0 03                       end
0x0062  c0 03                       end
0x0064  c0 03                       end"],
            &[
                TypeEntry::new(vec![], vec![]),
                TypeEntry::new(vec![OperandDataType::I32], vec![]),
                TypeEntry::new(vec![OperandDataType::I32], vec![OperandDataType::I32]),
                TypeEntry::new(
                    vec![OperandDataType::I32, OperandDataType::I32],
                    vec![OperandDataType::I32, OperandDataType::I32],
                ),
            ],
            &[
                LocalVariableListEntry::new(vec![]),
                LocalVariableListEntry::new(vec![LocalVariableEntry::from_i32()]),
                LocalVariableListEntry::new(vec![
                    LocalVariableEntry::from_i32(),
                    LocalVariableEntry::from_i32(),
                    LocalVariableEntry::from_i32(),
                ]),
                LocalVariableListEntry::new(vec![
                    LocalVariableEntry::from_i32(),
                    LocalVariableEntry::from_i32(),
                    LocalVariableEntry::from_i32(),
                    LocalVariableEntry::from_i32(),
                    LocalVariableEntry::from_i32(),
                ]),
            ],
        );

        // test type index and local list index
        assert_fn(
            r#"
        fn foo(a1:i32, a2:i32) -> i32   // type=1, local=1
        [a3:i32]
        {
            block(b1:i32=imm_i32(0x11)) -> i32  // type=2, local=1
            [b2:i32, b3:i32]
            nop()

            block(c1:i32=imm_i32(0x13), c2:i32=imm_i32(0x17)) -> i32    // type=1, local=1
            [c3:i32]
            nop()

            block(d1:i32=imm_i32(0x19), d2:i32=imm_i32(0x23), d3:i32=imm_i32(0x29)) -> i32  // type=3, local=1
            nop()
        }"#,
            &["\
0x0000  40 01 00 00  11 00 00 00    imm_i32           0x00000011
0x0008  c1 03 00 00  02 00 00 00    block             type:2   local:1
        01 00 00 00
0x0014  00 01                       nop
0x0016  c0 03                       end
0x0018  40 01 00 00  13 00 00 00    imm_i32           0x00000013
0x0020  40 01 00 00  17 00 00 00    imm_i32           0x00000017
0x0028  c1 03 00 00  01 00 00 00    block             type:1   local:1
        01 00 00 00
0x0034  00 01                       nop
0x0036  c0 03                       end
0x0038  40 01 00 00  19 00 00 00    imm_i32           0x00000019
0x0040  40 01 00 00  23 00 00 00    imm_i32           0x00000023
0x0048  40 01 00 00  29 00 00 00    imm_i32           0x00000029
0x0050  c1 03 00 00  03 00 00 00    block             type:3   local:1
        01 00 00 00
0x005c  00 01                       nop
0x005e  c0 03                       end
0x0060  c0 03                       end"],
            &[
                TypeEntry::new(vec![], vec![]),
                TypeEntry::new(
                    vec![OperandDataType::I32, OperandDataType::I32],
                    vec![OperandDataType::I32],
                ),
                TypeEntry::new(vec![OperandDataType::I32], vec![OperandDataType::I32]),
                TypeEntry::new(
                    vec![
                        OperandDataType::I32,
                        OperandDataType::I32,
                        OperandDataType::I32,
                    ],
                    vec![OperandDataType::I32],
                ),
            ],
            &[
                LocalVariableListEntry::new(vec![]),
                LocalVariableListEntry::new(vec![
                    LocalVariableEntry::from_i32(),
                    LocalVariableEntry::from_i32(),
                    LocalVariableEntry::from_i32(),
                ]),
            ],
        );
    }

    #[test]
    fn test_assemble_expression_break() {
        assert_eq!(
            bytecode(
                r#"
        fn foo() {
            block(a:i32=imm_i32(0x42)) {
                break (imm_i32(0x11))
                break_fn (imm_i32(0x19), imm_i32(0x23), imm_i32(0x29))
            }
        }
        "#
            ),
            "\
0x0000  40 01 00 00  42 00 00 00    imm_i32           0x00000042
0x0008  c1 03 00 00  01 00 00 00    block             type:1   local:1
        01 00 00 00
0x0014  40 01 00 00  11 00 00 00    imm_i32           0x00000011
0x001c  c2 03 00 00  2a 00 00 00    break             rev:0   off:0x2a
0x0024  40 01 00 00  19 00 00 00    imm_i32           0x00000019
0x002c  40 01 00 00  23 00 00 00    imm_i32           0x00000023
0x0034  40 01 00 00  29 00 00 00    imm_i32           0x00000029
0x003c  c2 03 01 00  00 00 00 00    break             rev:1   off:0x00
0x0044  c0 03                       end
0x0046  c0 03                       end"
        );
    }

    #[test]
    fn test_assemble_expression_recur_fn() {
        assert_eq!(
            bytecode(
                r#"
        fn foo() {
            block(a:i32=imm_i32(0x42)) {
                imm_i32(0x50)
                recur (imm_i32(0x11))
                recur_fn (imm_i32(0x19), imm_i32(0x23), imm_i32(0x29))
            }
        }
        "#
            ),
            "\
0x0000  40 01 00 00  42 00 00 00    imm_i32           0x00000042
0x0008  c1 03 00 00  01 00 00 00    block             type:1   local:1
        01 00 00 00
0x0014  40 01 00 00  50 00 00 00    imm_i32           0x00000050
0x001c  40 01 00 00  11 00 00 00    imm_i32           0x00000011
0x0024  c3 03 00 00  10 00 00 00    recur             rev:0   off:0x10
0x002c  40 01 00 00  19 00 00 00    imm_i32           0x00000019
0x0034  40 01 00 00  23 00 00 00    imm_i32           0x00000023
0x003c  40 01 00 00  29 00 00 00    imm_i32           0x00000029
0x0044  c3 03 00 00  30 00 00 00    recur             rev:0   off:0x30
0x004c  c0 03                       end
0x004e  c0 03                       end"
        );
    }

    #[test]
    fn test_assemble_instruction_base() {
        assert_eq!(
            bytecode(
                r#"
fn foo() {
    nop()
    imm_i32(0x11)
    imm_i64(0x13)
    imm_f32(3.142)
    imm_f64(2.718)
}
        "#
            ),
            "\
0x0000  00 01                       nop
0x0002  00 01                       nop
0x0004  40 01 00 00  11 00 00 00    imm_i32           0x00000011
0x000c  41 01 00 00  13 00 00 00    imm_i64           low:0x00000013  high:0x00000000
        00 00 00 00
0x0018  42 01 00 00  87 16 49 40    imm_f32           0x40491687
0x0020  43 01 00 00  58 39 b4 c8    imm_f64           low:0xc8b43958  high:0x4005be76
        76 be 05 40
0x002c  c0 03                       end"
        );
    }

    #[test]
    fn test_assemble_instruction_local_load_store() {
        assert_eq!(
            bytecode(
                r#"
fn foo(left:i32, right:i32) {
    local_load_i64(left, offset=4)
    local_load_i32_s(left, offset=2)
    local_load_i32_u(left, offset=0)
    local_load_i16_s(right)
    local_load_i16_u(right)
    local_load_i8_s(right)
    local_load_i8_u(right)
    local_load_f32(left)
    local_load_f64(right)
}
        "#
            ),
            "\
0x0000  80 01 00 00  04 00 00 00    local_load_64     rev:0   off:0x04  idx:0
0x0008  81 01 00 00  02 00 00 00    local_load_i32_s  rev:0   off:0x02  idx:0
0x0010  82 01 00 00  00 00 00 00    local_load_i32_u  rev:0   off:0x00  idx:0
0x0018  83 01 00 00  00 00 01 00    local_load_i16_s  rev:0   off:0x00  idx:1
0x0020  84 01 00 00  00 00 01 00    local_load_i16_u  rev:0   off:0x00  idx:1
0x0028  85 01 00 00  00 00 01 00    local_load_i8_s   rev:0   off:0x00  idx:1
0x0030  86 01 00 00  00 00 01 00    local_load_i8_u   rev:0   off:0x00  idx:1
0x0038  88 01 00 00  00 00 00 00    local_load_f32    rev:0   off:0x00  idx:0
0x0040  87 01 00 00  00 00 01 00    local_load_f64    rev:0   off:0x00  idx:1
0x0048  c0 03                       end"
        );

        assert_eq!(
            bytecode(
                r#"
fn foo(left:i32, right:i32) {
    local_load_extend_i64(left, imm_i32(0x2))
    local_load_extend_i32_s(left, imm_i32(0x4))
    local_load_extend_i32_u(left, imm_i32(0x8))
    local_load_extend_i16_s(right, imm_i32(0x2))
    local_load_extend_i16_u(right, imm_i32(0x4))
    local_load_extend_i8_s(right, imm_i32(0x8))
    local_load_extend_i8_u(right, imm_i32(0xa))
    local_load_extend_f32(left, imm_i32(0x2))
    local_load_extend_f64(right, imm_i32(0x4))
}
        "#
            ),
            "\
0x0000  40 01 00 00  02 00 00 00    imm_i32           0x00000002
0x0008  8f 01 00 00  00 00 00 00    local_load_extend_i64  rev:0   idx:0
0x0010  40 01 00 00  04 00 00 00    imm_i32           0x00000004
0x0018  90 01 00 00  00 00 00 00    local_load_extend_i32_s  rev:0   idx:0
0x0020  40 01 00 00  08 00 00 00    imm_i32           0x00000008
0x0028  91 01 00 00  00 00 00 00    local_load_extend_i32_u  rev:0   idx:0
0x0030  40 01 00 00  02 00 00 00    imm_i32           0x00000002
0x0038  92 01 00 00  01 00 00 00    local_load_extend_i16_s  rev:0   idx:1
0x0040  40 01 00 00  04 00 00 00    imm_i32           0x00000004
0x0048  93 01 00 00  01 00 00 00    local_load_extend_i16_u  rev:0   idx:1
0x0050  40 01 00 00  08 00 00 00    imm_i32           0x00000008
0x0058  94 01 00 00  01 00 00 00    local_load_extend_i8_s  rev:0   idx:1
0x0060  40 01 00 00  0a 00 00 00    imm_i32           0x0000000a
0x0068  95 01 00 00  01 00 00 00    local_load_extend_i8_u  rev:0   idx:1
0x0070  40 01 00 00  02 00 00 00    imm_i32           0x00000002
0x0078  97 01 00 00  00 00 00 00    local_load_extend_f32  rev:0   idx:0
0x0080  40 01 00 00  04 00 00 00    imm_i32           0x00000004
0x0088  96 01 00 00  01 00 00 00    local_load_extend_f64  rev:0   idx:1
0x0090  c0 03                       end"
        );

        assert_eq!(
            bytecode(
                r#"
fn foo()
[left:i32, right:i32]
{
    local_store_i64(left, imm_i64(0x11), offset=4)
    local_store_i32(left, imm_i32(0x13), offset=2)
    local_store_i16(right, imm_i32(0x17))
    local_store_i8(right, imm_i32(0x19))
    local_store_f32(left, imm_f32(3.142))
    local_store_f64(right, imm_f64(2.718))
}
        "#
            ),
            "\
0x0000  41 01 00 00  11 00 00 00    imm_i64           low:0x00000011  high:0x00000000
        00 00 00 00
0x000c  89 01 00 00  04 00 00 00    local_store_i64   rev:0   off:0x04  idx:0
0x0014  40 01 00 00  13 00 00 00    imm_i32           0x00000013
0x001c  8a 01 00 00  02 00 00 00    local_store_i32   rev:0   off:0x02  idx:0
0x0024  40 01 00 00  17 00 00 00    imm_i32           0x00000017
0x002c  8b 01 00 00  00 00 01 00    local_store_i16   rev:0   off:0x00  idx:1
0x0034  40 01 00 00  19 00 00 00    imm_i32           0x00000019
0x003c  8c 01 00 00  00 00 01 00    local_store_i8    rev:0   off:0x00  idx:1
0x0044  42 01 00 00  87 16 49 40    imm_f32           0x40491687
0x004c  8e 01 00 00  00 00 00 00    local_store_f32   rev:0   off:0x00  idx:0
0x0054  43 01 00 00  58 39 b4 c8    imm_f64           low:0xc8b43958  high:0x4005be76
        76 be 05 40
0x0060  8d 01 00 00  00 00 01 00    local_store_f64   rev:0   off:0x00  idx:1
0x0068  c0 03                       end"
        );

        assert_eq!(
            bytecode(
                r#"
fn foo()
[left:i32, right:i32]
{
    local_store_extend_i64(left, imm_i32(0x2), imm_i64(0x11))
    local_store_extend_i32(left, imm_i32(0x4),imm_i32(0x13))
    local_store_extend_i16(right, imm_i32(0x8),imm_i32(0x17))
    local_store_extend_i8(right, imm_i32(0xa),imm_i32(0x19))
    local_store_extend_f32(left, imm_i32(0x2),imm_f32(3.142))
    local_store_extend_f64(right, imm_i32(0x4),imm_f64(2.718))
}
        "#
            ),
            "\
0x0000  40 01 00 00  02 00 00 00    imm_i32           0x00000002
0x0008  41 01 00 00  11 00 00 00    imm_i64           low:0x00000011  high:0x00000000
        00 00 00 00
0x0014  98 01 00 00  00 00 00 00    local_store_extend_i64  rev:0   idx:0
0x001c  40 01 00 00  04 00 00 00    imm_i32           0x00000004
0x0024  40 01 00 00  13 00 00 00    imm_i32           0x00000013
0x002c  99 01 00 00  00 00 00 00    local_store_extend_i32  rev:0   idx:0
0x0034  40 01 00 00  08 00 00 00    imm_i32           0x00000008
0x003c  40 01 00 00  17 00 00 00    imm_i32           0x00000017
0x0044  9a 01 00 00  01 00 00 00    local_store_extend_i16  rev:0   idx:1
0x004c  40 01 00 00  0a 00 00 00    imm_i32           0x0000000a
0x0054  40 01 00 00  19 00 00 00    imm_i32           0x00000019
0x005c  9b 01 00 00  01 00 00 00    local_store_extend_i8  rev:0   idx:1
0x0064  40 01 00 00  02 00 00 00    imm_i32           0x00000002
0x006c  42 01 00 00  87 16 49 40    imm_f32           0x40491687
0x0074  9d 01 00 00  00 00 00 00    local_store_extend_f32  rev:0   idx:0
0x007c  40 01 00 00  04 00 00 00    imm_i32           0x00000004
0x0084  43 01 00 00  58 39 b4 c8    imm_f64           low:0xc8b43958  high:0x4005be76
        76 be 05 40
0x0090  9c 01 00 00  01 00 00 00    local_store_extend_f64  rev:0   idx:1
0x0098  c0 03                       end"
        );
    }

    #[test]
    fn test_assemble_instruction_data_load_store() {
        // todo
    }

    #[test]
    fn test_assemble_instruction_memory_load_store() {
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
        assert_eq!(
            bytecode(
                r#"
fn foo()
{
    call(bar, imm_i32(0x11), imm_i32(0x13))
    envcall(0x100, imm_i32(0x23), imm_i32(0x29))
    syscall(
        0x101 // num
        imm_i32(0x31), imm_i32(0x37) // args
        )
    dyncall(get_function(bar), imm_i32(0x41), imm_i32(0x43))
}

fn bar(left:i32, right:i32) nop()
        "#
            ),
            "\
0x0000  40 01 00 00  11 00 00 00    imm_i32           0x00000011
0x0008  40 01 00 00  13 00 00 00    imm_i32           0x00000013
0x0010  00 04 00 00  01 00 00 00    call              idx:1
0x0018  40 01 00 00  23 00 00 00    imm_i32           0x00000023
0x0020  40 01 00 00  29 00 00 00    imm_i32           0x00000029
0x0028  02 04 00 00  00 01 00 00    envcall           idx:256
0x0030  40 01 00 00  31 00 00 00    imm_i32           0x00000031
0x0038  40 01 00 00  37 00 00 00    imm_i32           0x00000037
0x0040  40 01 00 00  01 01 00 00    imm_i32           0x00000101
0x0048  40 01 00 00  02 00 00 00    imm_i32           0x00000002
0x0050  03 04                       syscall
0x0052  00 01                       nop
0x0054  05 04 00 00  01 00 00 00    get_function      idx:1
0x005c  40 01 00 00  41 00 00 00    imm_i32           0x00000041
0x0064  40 01 00 00  43 00 00 00    imm_i32           0x00000043
0x006c  01 04                       dyncall
0x006e  c0 03                       end"
        );

        // test external
        assert_eq!(
            bytecode_with_import_and_external(
                r#"
external fn libabc::dothis(i32,i32)->i32
external fn libabc::dothat()

fn foo() {
    extcall(dothis, imm_i32(0x17), imm_i32(0x19))   // index 0
    extcall(dothat) // index 1

    // the public index of function "bar" is `1`,
    // because the function public index does not include external functions.
    call(bar)
}

fn bar() {
    nop()
}"#,
                vec![],
                vec![ExternalLibraryEntry::new(
                    "libabc".to_owned(),
                    Box::new(ExternalLibraryDependency::Local(Box::new(
                        DependencyLocal {
                            path: "libabc.so.1".to_owned(),
                            values: None,
                            condition: None
                        }
                    )))
                )]
            ),
            "\
0x0000  40 01 00 00  17 00 00 00    imm_i32           0x00000017
0x0008  40 01 00 00  19 00 00 00    imm_i32           0x00000019
0x0010  04 04 00 00  00 00 00 00    extcall           idx:0
0x0018  04 04 00 00  01 00 00 00    extcall           idx:1
0x0020  00 04 00 00  01 00 00 00    call              idx:1
0x0028  c0 03                       end"
        );
    }

    #[test]
    fn test_assemble_instruction_host() {
        // todo
    }
}
