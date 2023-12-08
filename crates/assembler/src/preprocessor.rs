// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancasm_parser::{
    ast::{
        BranchCase, DataKindNode, DataNode, ExternalFunctionNode, ExternalItem, ExternalNode,
        FunctionNode, ImportDataNode, ImportFunctionNode, ImportItem, ImportNode, Instruction,
        LocalNode, ModuleElementNode, ModuleNode, ParamNode,
    },
    NAME_PATH_SEPARATOR,
};
use ancvm_types::DataType;

use crate::AssembleError;

// merge multiple submodule nodes into one module node.
//
// e.g.
// consider that an applicaton contains the following 3 submodules (source code files):
//
// - `(module $myapp ...)`
// - `(module $myapp::process ...)`
// - `(module $myapp::utils ...)`
//
// they will be merged into one module:
//
// `(module $myapp ...)`
//
// the rules of the merger are:
//
// 1. canonicalize the identifiers of functions and datas
//
// in submodule 'myapp':
// (function $entry ...)
//
// in submodule 'myapp::utils':
// (function $add ...)
//
// the identifiers will be renamed to:
//
// - 'myapp::entry'
// - 'myapp::utils::add'
//
// note that the identifier and name of the function
// are the same (as well as the data).
//
// 2. canonicalize the identifiers of call instruction and
//    data store and load instructions.
//
// e.g.
// the identifier `$add`` in `(call $add ...)` and macro `(macro.get_function_public_index $add ...)`
// will be rewritten to '$myapp::utils::add'.
//
// when the identifier already contains the namespace path separator `::` and
// is an absolute path, it will not be rewritten, however,
// the relative name paths will.
//
// relative paths begin with the keywords 'module' and 'self', e.g.
// `(call $module::utils::add ...)`
// `(call $self::utils::add ...)`
//
// 3. canonicalize the identifiers of external functions.
//
// the identifiers and names of external functions (as well as imported items)
// can be different for simplify the writing of the assembly text, so these identifiers
// need to be canonicalized before assemble.
//
// e.g.
// in submodule 'myapp':
//
// (external (library share "math.so.1")
//         (function $add "add" ...)
// )
// (extcall $add ...)
//
// in submodule 'myapp::utils':
//
// (external (library share "math.so.1")
//         (function $f0 "add" ...)
// )
// (extcall $f0 ...)
//
// the identifiers '$add' and '$f0' will be
// rewritten as 'EXTERNAL_FUNCTION::share::math.so.1::add' in both the node 'external' and 'extcall'
//
// in addition to rewriteing identifiers, duplicate 'external' nodes
// will be removed.
//
// the expect identifier name for external function is:
// EXTERNAL_FUNCTION::EXTERNAL_LIBRARY_TYPE::LIBRARY_SO_NAME::SYMBOL_NAME
//
// 4. canonicalize the identifiers of imported items.
//
// it's similar to the external functions, e.g.
//
// in submodule 'myapp':
//
// (import (module share "math")
//         (function $add "add" ...)
// )
// (call $add ...)
//
// in submodule 'myapp::utils':
//
// (import (module share "math")
//         (function $f0 "add" ...)
// )
// (call $f0 ...)
//
// the identifiers '$add' and '$f0' will be
// rewritten as '$math::add' in both the node 'import' and 'call'
//
// in addition to rewriteing identifiers, duplicate 'import' nodes
// will be removed.
//
// the expect identifier name for imported item is:
// MODULE_NAME::PATH_NAMES::ITEM_NAME
//
// note: at the assembly level, submodules are transparent to each other,
// i.e., all functions and data (including imported functions, imported data,
// and declared external functions) are public and can be accessed in any submodule.

#[derive(Debug, PartialEq)]
pub struct MergedModuleNode {
    // the main module name
    pub name: String,

    pub runtime_version_major: u16,
    pub runtime_version_minor: u16,

    pub constructor_function_name: Option<String>,
    pub destructor_function_name: Option<String>,

    pub function_nodes: Vec<CanonicalFunctionNode>,
    pub read_only_data_nodes: Vec<CanonicalDataNode>,
    pub read_write_data_nodes: Vec<CanonicalDataNode>,
    pub uninit_data_nodes: Vec<CanonicalDataNode>,
    pub external_nodes: Vec<ExternalNode>,
    pub import_nodes: Vec<ImportNode>,
}

#[derive(Debug, PartialEq)]
pub struct CanonicalFunctionNode {
    // the full name path, for function calling instructions
    //
    // e.g.
    // the id of function 'add' in module 'myapp' is 'myapp::add'
    // the id of function 'add' in submodule 'myapp:utils' is 'myapp::utils::add'
    pub id: String,

    // the canonicalize name path, which includes the submodule path, but
    // excludes the module name.
    //
    // e.g.
    // the name path of function 'add' in module 'myapp' is 'add'
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
    // the id of data 'buf' in module 'myapp' is 'myapp::buf'
    // the id of data 'buf' in submodule 'myapp:utils' is 'myapp::utils::buf'
    pub id: String,

    // the canonicalize name path, which includes the submodule path, but
    // excludes the module name.
    //
    // e.g.
    // the name path of data 'buf' in module 'myapp' is 'buf'
    // the name path of data 'buf' in submodule 'myapp:utils' is 'utils::buf'
    pub name_path: String,

    pub export: bool,
    pub data_kind: DataKindNode,
}

struct RenameItemModule {
    module_name_path: String,
    items: Vec<RenameItem>,
}

struct RenameItem {
    // item_name, without name path, e.g.
    // - "add"
    // - "sub_i32"
    from: String,

    // rename to, e.g.
    //
    // - "MODULE_NAME::NAME_PATH::FUNC_NAME"
    // - "MODULE_NAME::NAME_PATH::DATA_NAME"
    // - "EXTERNAL_FUNCTION::EXTERNAL_LIBRARY_TYPE::LIBRARY_SO_NAME::SYMBOL_NAME"
    to: String,

    rename_kind: RenameKind,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum RenameKind {
    // canonicalize the internal functions or rename the imported functions
    Function,

    // canonicalize the internal data or rename the imported data
    Data,

    // rename the external functions
    ExternalFunction,
}

pub fn merge_and_canonicalize_submodule_nodes(
    submodule_nodes: &[ModuleNode],
) -> Result<MergedModuleNode, AssembleError> {
    // the first submodule is the main submodule of an application or a library.
    // so pick the name and runtime version from the first submodule.

    let name = submodule_nodes[0].name_path.clone();
    let runtime_version_major = submodule_nodes[0].runtime_version_major;
    let runtime_version_minor = submodule_nodes[0].runtime_version_minor;
    let constructor_function_name = submodule_nodes[0].constructor_function_name.clone();
    let destructor_function_name = submodule_nodes[0].destructor_function_name.clone();

    // check submodules name path and runtime version

    for module_node in &submodule_nodes[1..] {
        let first_part_of_name_path = module_node
            .name_path
            .split(NAME_PATH_SEPARATOR)
            .next()
            .unwrap();

        if first_part_of_name_path != name {
            return Err(AssembleError {
                message: format!(
                    "The name path of submodule: \"{}\" does not starts with: \"{}\"",
                    module_node.name_path, name
                ),
            });
        }

        if module_node.runtime_version_major != runtime_version_major
            || module_node.runtime_version_minor != runtime_version_minor
        {
            return Err(AssembleError {
                message: format!(
                    "The runtime version of submodule: \"{}.{}\" does not match the main module: \"{}.{}\"",
                    module_node.runtime_version_major, module_node.runtime_version_minor ,
                    runtime_version_major, runtime_version_minor
                ),
            });
        }
    }

    let mut rename_item_modules: Vec<RenameItemModule> = vec![];

    // canonicalize the external nodes and remove the duplicate items
    let mut canonical_external_nodes: Vec<ExternalNode> = vec![];

    for submodule_node in submodule_nodes {
        let submodule_name_path = &submodule_node.name_path;
        let mut rename_items: Vec<RenameItem> = vec![];

        let original_external_nodes = submodule_node
            .element_nodes
            .iter()
            .filter_map(|node| match node {
                ModuleElementNode::ExternalNode(external_node) => Some(external_node),
                _ => None,
            })
            .collect::<Vec<_>>();

        // create new canonical external node if it does not exist.

        for original_external_node in original_external_nodes {
            let idx_opt_of_canonical_external_node =
                canonical_external_nodes.iter().position(|node| {
                    node.external_library_node == original_external_node.external_library_node
                });

            let idx_of_canonical_external_node =
                if let Some(idx) = idx_opt_of_canonical_external_node {
                    idx
                } else {
                    let idx = canonical_external_nodes.len();

                    // create new canonical external node
                    let canonical_external_node = ExternalNode {
                        external_library_node: original_external_node.external_library_node.clone(),
                        external_items: vec![],
                    };

                    canonical_external_nodes.push(canonical_external_node);
                    idx
                };

            let canonical_external_node =
                &mut canonical_external_nodes[idx_of_canonical_external_node];

            for original_external_item in &original_external_node.external_items {
                // build the canonical external item

                let canonical_external_item = match original_external_item {
                    ExternalItem::ExternalFunction(original_external_function) => {
                        // the format of expect identifier name:
                        // EXTERNAL_FUNCTION::EXTERNAL_LIBRARY_TYPE::LIBRARY_SO_NAME::SYMBOL_NAME

                        let name = original_external_function.name.clone();

                        let expect_identifier = format!(
                            "EXTERNAL_FUNCTION::{}::{}::{}",
                            original_external_node
                                .external_library_node
                                .external_library_type,
                            original_external_node.external_library_node.name,
                            name
                        );

                        let canonical_external_function_node = ExternalFunctionNode {
                            id: expect_identifier,
                            name,
                            params: original_external_function.params.clone(),
                            results: original_external_function.results.clone(),
                        };

                        ExternalItem::ExternalFunction(canonical_external_function_node)
                    }
                };

                let idx_opt_of_canonical_external_item =
                    canonical_external_node.external_items.iter().position(
                        |exists_external_item| exists_external_item == &canonical_external_item,
                    );

                // create new canonical external item if it does not exist.

                let idx_of_canonical_external_item =
                    if let Some(idx) = idx_opt_of_canonical_external_item {
                        idx
                    } else {
                        let idx = canonical_external_node.external_items.len();

                        // add new canonical external item
                        canonical_external_node
                            .external_items
                            .push(canonical_external_item);

                        idx
                    };

                let expect_identifier =
                    match &canonical_external_node.external_items[idx_of_canonical_external_item] {
                        ExternalItem::ExternalFunction(external_function) => {
                            external_function.id.to_owned()
                        }
                    };

                let actual_identifier = match original_external_item {
                    ExternalItem::ExternalFunction(external_function) => {
                        external_function.id.to_owned()
                    }
                };

                // add rename item if it does not exist
                if expect_identifier != actual_identifier
                    && !rename_items
                        .iter()
                        .any(|item| item.from == actual_identifier && item.to == expect_identifier)
                {
                    let rename_item = RenameItem {
                        from: actual_identifier,
                        to: expect_identifier,
                        rename_kind: RenameKind::ExternalFunction,
                    };
                    rename_items.push(rename_item);
                }
            }
        }

        let idx_of_rename_item_module_opt = rename_item_modules
            .iter()
            .position(|module| &module.module_name_path == submodule_name_path);

        if let Some(idx_of_rename_item_module) = idx_of_rename_item_module_opt {
            rename_item_modules[idx_of_rename_item_module]
                .items
                .extend(rename_items.into_iter())
        } else {
            let rename_item_module = RenameItemModule {
                module_name_path: submodule_name_path.to_owned(),
                items: rename_items,
            };

            rename_item_modules.push(rename_item_module);
        }
    }

    // canonicalize the import nodes and remove the duplicate items
    let mut canonical_import_nodes: Vec<ImportNode> = vec![];

    for submodule_node in submodule_nodes {
        let submodule_name_path = &submodule_node.name_path;
        let mut rename_items: Vec<RenameItem> = vec![];

        let original_imported_nodes = submodule_node
            .element_nodes
            .iter()
            .filter_map(|node| match node {
                ModuleElementNode::ImportNode(import_node) => Some(import_node),
                _ => None,
            })
            .collect::<Vec<_>>();

        // create new canonical import node if it does not exist.

        for original_import_node in original_imported_nodes {
            let idx_opt_of_canonical_import_node = canonical_import_nodes.iter().position(|node| {
                node.import_module_node == original_import_node.import_module_node
            });

            let idx_of_canonical_import_node = if let Some(idx) = idx_opt_of_canonical_import_node {
                idx
            } else {
                let idx = canonical_import_nodes.len();

                // create new canonical import node
                let canonical_import_node = ImportNode {
                    import_module_node: original_import_node.import_module_node.clone(),
                    import_items: vec![],
                };

                canonical_import_nodes.push(canonical_import_node);
                idx
            };

            let canonical_import_node = &mut canonical_import_nodes[idx_of_canonical_import_node];

            for original_import_item in &original_import_node.import_items {
                // build the canonical external item

                let canonical_import_item = match original_import_item {
                    ImportItem::ImportFunction(original_import_function) => {
                        // the format of expect identifier name:
                        // TARGET_MODULE_NAME::NAME_PATH::FUNCTION_NAME

                        let name_path = original_import_function.name_path.clone();

                        let expect_identifier = format!(
                            "{}::{}",
                            original_import_node.import_module_node.name, name_path
                        );

                        let canonical_import_function_node = ImportFunctionNode {
                            id: expect_identifier,
                            name_path,
                            params: original_import_function.params.clone(),
                            results: original_import_function.results.clone(),
                        };

                        ImportItem::ImportFunction(canonical_import_function_node)
                    }
                    ImportItem::ImportData(original_import_data) => {
                        // the format of expect identifier name:
                        // MODULE_NAME::NAME_PATH::FUNCTION_NAME

                        let name_path = original_import_data.name_path.clone();

                        let expect_identifier = format!(
                            "{}::{}",
                            original_import_node.import_module_node.name, name_path
                        );

                        let canonical_import_data_node = ImportDataNode {
                            id: expect_identifier,
                            name_path,
                            data_kind_node: original_import_data.data_kind_node.clone(),
                        };

                        ImportItem::ImportData(canonical_import_data_node)
                    }
                };

                let idx_opt_of_canonical_import_item = canonical_import_node
                    .import_items
                    .iter()
                    .position(|exists_import_item| exists_import_item == &canonical_import_item);

                // create new canonical import item if it does not exist.

                let idx_of_canonical_import_item =
                    if let Some(idx) = idx_opt_of_canonical_import_item {
                        idx
                    } else {
                        let idx = canonical_import_node.import_items.len();

                        // add new canonical import item
                        canonical_import_node
                            .import_items
                            .push(canonical_import_item);

                        idx
                    };

                let expect_identifier = match &canonical_import_node.import_items
                    [idx_of_canonical_import_item]
                {
                    ImportItem::ImportFunction(import_function) => import_function.id.to_owned(),
                    ImportItem::ImportData(import_data) => import_data.id.to_owned(),
                };

                let actual_identifier = match original_import_item {
                    ImportItem::ImportFunction(import_function) => import_function.id.to_owned(),
                    ImportItem::ImportData(import_data) => import_data.id.to_owned(),
                };

                // add rename item if it does not exist
                if expect_identifier != actual_identifier
                    && !rename_items
                        .iter()
                        .any(|item| item.from == actual_identifier && item.to == expect_identifier)
                {
                    let rename_kind = match original_import_item {
                        ImportItem::ImportFunction(_) => RenameKind::Function,
                        ImportItem::ImportData(_) => RenameKind::Data,
                    };

                    let rename_item = RenameItem {
                        from: actual_identifier,
                        to: expect_identifier,
                        rename_kind,
                    };
                    rename_items.push(rename_item);
                }
            }
        }

        let idx_of_rename_item_module_opt = rename_item_modules
            .iter()
            .position(|module| &module.module_name_path == submodule_name_path);

        if let Some(idx_of_rename_item_module) = idx_of_rename_item_module_opt {
            rename_item_modules[idx_of_rename_item_module]
                .items
                .extend(rename_items.into_iter())
        } else {
            let rename_item_module = RenameItemModule {
                module_name_path: submodule_name_path.to_owned(),
                items: rename_items,
            };

            rename_item_modules.push(rename_item_module);
        }
    }

    let mut canonical_function_nodes: Vec<CanonicalFunctionNode> = vec![];
    let mut canonical_read_only_data_nodes: Vec<CanonicalDataNode> = vec![];
    let mut canonical_read_write_data_nodes: Vec<CanonicalDataNode> = vec![];
    let mut canonical_uninit_data_nodes: Vec<CanonicalDataNode> = vec![];

    for module_node in submodule_nodes {
        let module_name_path = &module_node.name_path;

        // the name path that excludes the main module name
        // e.g.
        // the relative name path of 'myapp::utils::add' is 'utils::add'
        let relative_name_path = {
            if let Some((_head, tail)) = module_node.name_path.split_once("::") {
                Some(tail)
            } else {
                None
            }
        };

        // canonicalize the func nodes
        let original_function_nodes = module_node
            .element_nodes
            .iter()
            .filter_map(|node| match node {
                ModuleElementNode::FunctionNode(function_node) => Some(function_node),
                _ => None,
            })
            .collect::<Vec<_>>();

        let mut function_nodes = vec![];
        for original_function_node in original_function_nodes {
            let function_node = canonicalize_function_node(
                original_function_node,
                module_name_path,
                relative_name_path,
                &rename_item_modules,
            )?;
            function_nodes.push(function_node);
        }

        let mut read_only_data_nodes = module_node
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
            .map(|data_node| {
                canonicalize_data_node(data_node, module_name_path, relative_name_path)
            })
            .collect::<Vec<_>>();

        let mut read_write_data_nodes = module_node
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
            .map(|data_node| {
                canonicalize_data_node(data_node, module_name_path, relative_name_path)
            })
            .collect::<Vec<_>>();

        let mut uninit_data_nodes = module_node
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
            .map(|data_node| {
                canonicalize_data_node(data_node, module_name_path, relative_name_path)
            })
            .collect::<Vec<_>>();

        canonical_function_nodes.append(&mut function_nodes);
        canonical_read_only_data_nodes.append(&mut read_only_data_nodes);
        canonical_read_write_data_nodes.append(&mut read_write_data_nodes);
        canonical_uninit_data_nodes.append(&mut uninit_data_nodes);
    }

    let merged_module_node = MergedModuleNode {
        name,
        runtime_version_major,
        runtime_version_minor,
        constructor_function_name,
        destructor_function_name,
        function_nodes: canonical_function_nodes,
        read_only_data_nodes: canonical_read_only_data_nodes,
        read_write_data_nodes: canonical_read_write_data_nodes,
        uninit_data_nodes: canonical_uninit_data_nodes,
        external_nodes: canonical_external_nodes,
        import_nodes: canonical_import_nodes,
    };

    Ok(merged_module_node)
}

fn canonicalize_function_node(
    function_node: &FunctionNode,
    module_name_path: &str,
    relative_name_path: Option<&str>,
    rename_item_modules: &[RenameItemModule],
) -> Result<CanonicalFunctionNode, AssembleError> {
    let function_full_name = format!("{}::{}", module_name_path, function_node.name);
    let function_name_path = if let Some(path) = relative_name_path {
        format!("{}::{}", path, function_node.name)
    } else {
        function_node.name.to_owned()
    };

    let canonical_code = canonicalize_identifiers_of_instructions(
        &function_node.code,
        module_name_path,
        rename_item_modules,
    )?;

    Ok(CanonicalFunctionNode {
        id: function_full_name,
        name_path: function_name_path,
        export: function_node.export,
        params: function_node.params.clone(),
        results: function_node.results.clone(),
        locals: function_node.locals.clone(),
        code: canonical_code,
    })
}

fn canonicalize_data_node(
    data_node: &DataNode,
    module_name_path: &str,
    relative_name_path: Option<&str>,
) -> CanonicalDataNode {
    let data_full_name = format!("{}::{}", module_name_path, data_node.name);
    let data_name_path = if let Some(path) = relative_name_path {
        format!("{}::{}", path, data_node.name)
    } else {
        data_node.name.to_owned()
    };

    CanonicalDataNode {
        id: data_full_name,
        name_path: data_name_path,
        export: data_node.export,
        data_kind: data_node.data_kind.clone(),
    }
}

fn canonicalize_identifiers_of_instructions(
    instructions: &[Instruction],
    module_name_path: &str,
    rename_item_modules: &[RenameItemModule],
) -> Result<Vec<Instruction>, AssembleError> {
    let mut canonical_instructions = vec![];

    for instruction in instructions {
        let canonical_instruction = canonicalize_identifiers_of_instruction(
            instruction,
            module_name_path,
            rename_item_modules,
        )?;

        canonical_instructions.push(canonical_instruction);
    }
    Ok(canonical_instructions)
}

fn canonicalize_identifiers_of_instruction(
    instruction: &Instruction,
    module_name_path: &str,
    rename_item_modules: &[RenameItemModule],
) -> Result<Instruction, AssembleError> {
    let canonical_instruction = match instruction {
        Instruction::NoParams { opcode, operands } => Instruction::NoParams {
            opcode: *opcode,
            operands: canonicalize_identifiers_of_instructions(
                operands,
                module_name_path,
                rename_item_modules,
            )?,
        },
        Instruction::ImmI32(_) => {
            // instruction without operands, just clone
            instruction.clone()
        }
        Instruction::ImmI64(_) => instruction.clone(),
        Instruction::ImmF32(_) => instruction.clone(),
        Instruction::ImmF64(_) => instruction.clone(),
        Instruction::LocalLoad {
            opcode: _,
            name: _,
            offset: _,
        } => instruction.clone(),
        Instruction::LocalStore {
            opcode,
            name,
            offset,
            value,
        } => Instruction::LocalStore {
            opcode: *opcode,
            name: name.clone(),
            offset: *offset,
            value: Box::new(canonicalize_identifiers_of_instruction(
                value,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::LocalLongLoad {
            opcode,
            name,
            offset,
        } => Instruction::LocalLongLoad {
            opcode: *opcode,
            name: name.clone(),
            offset: Box::new(canonicalize_identifiers_of_instruction(
                offset,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::LocalLongStore {
            opcode,
            name,
            offset,
            value,
        } => Instruction::LocalLongStore {
            opcode: *opcode,
            name: name.clone(),
            offset: Box::new(canonicalize_identifiers_of_instruction(
                offset,
                module_name_path,
                rename_item_modules,
            )?),
            value: Box::new(canonicalize_identifiers_of_instruction(
                value,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::DataLoad {
            opcode,
            id: name_path,
            offset,
        } => Instruction::DataLoad {
            opcode: *opcode,
            id: canonicalize_name_path_in_instruction(
                RenameKind::Data,
                name_path,
                module_name_path,
                rename_item_modules,
            )?,
            offset: *offset,
        },
        Instruction::DataStore {
            opcode,
            id: name_path,
            offset,
            value,
        } => Instruction::DataStore {
            opcode: *opcode,
            id: canonicalize_name_path_in_instruction(
                RenameKind::Data,
                name_path,
                module_name_path,
                rename_item_modules,
            )?,
            offset: *offset,
            value: Box::new(canonicalize_identifiers_of_instruction(
                value,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::DataLongLoad {
            opcode,
            id: name_path,
            offset,
        } => Instruction::DataLongLoad {
            opcode: *opcode,
            id: canonicalize_name_path_in_instruction(
                RenameKind::Data,
                name_path,
                module_name_path,
                rename_item_modules,
            )?,
            offset: Box::new(canonicalize_identifiers_of_instruction(
                offset,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::DataLongStore {
            opcode,
            id: name_path,
            offset,
            value,
        } => Instruction::DataLongStore {
            opcode: *opcode,
            id: canonicalize_name_path_in_instruction(
                RenameKind::Data,
                name_path,
                module_name_path,
                rename_item_modules,
            )?,
            offset: Box::new(canonicalize_identifiers_of_instruction(
                offset,
                module_name_path,
                rename_item_modules,
            )?),
            value: Box::new(canonicalize_identifiers_of_instruction(
                value,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::HeapLoad {
            opcode,
            offset,
            addr,
        } => Instruction::HeapLoad {
            opcode: *opcode,
            offset: *offset,
            addr: Box::new(canonicalize_identifiers_of_instruction(
                addr,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::HeapStore {
            opcode,
            offset,
            addr,
            value,
        } => Instruction::HeapStore {
            opcode: *opcode,
            offset: *offset,
            addr: Box::new(canonicalize_identifiers_of_instruction(
                addr,
                module_name_path,
                rename_item_modules,
            )?),
            value: Box::new(canonicalize_identifiers_of_instruction(
                value,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::UnaryOp { opcode, number } => Instruction::UnaryOp {
            opcode: *opcode,
            number: Box::new(canonicalize_identifiers_of_instruction(
                number,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::UnaryOpParamI16 {
            opcode,
            amount,
            number,
        } => Instruction::UnaryOpParamI16 {
            opcode: *opcode,
            amount: *amount,
            number: Box::new(canonicalize_identifiers_of_instruction(
                number,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::BinaryOp {
            opcode,
            left,
            right,
        } => Instruction::BinaryOp {
            opcode: *opcode,
            left: Box::new(canonicalize_identifiers_of_instruction(
                left,
                module_name_path,
                rename_item_modules,
            )?),
            right: Box::new(canonicalize_identifiers_of_instruction(
                right,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::When { test, consequent } => Instruction::When {
            test: Box::new(canonicalize_identifiers_of_instruction(
                test,
                module_name_path,
                rename_item_modules,
            )?),
            consequent: Box::new(canonicalize_identifiers_of_instruction(
                consequent,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::If {
            results,
            test,
            consequent,
            alternate,
        } => Instruction::If {
            results: results.clone(),
            test: Box::new(canonicalize_identifiers_of_instruction(
                test,
                module_name_path,
                rename_item_modules,
            )?),
            consequent: Box::new(canonicalize_identifiers_of_instruction(
                consequent,
                module_name_path,
                rename_item_modules,
            )?),
            alternate: Box::new(canonicalize_identifiers_of_instruction(
                alternate,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::Branch {
            results,
            cases,
            default,
        } => Instruction::Branch {
            results: results.clone(),
            cases: {
                let mut canonical_cases = vec![];
                for original_case in cases {
                    let canonical_case = BranchCase {
                        test: Box::new(canonicalize_identifiers_of_instruction(
                            &original_case.test,
                            module_name_path,
                            rename_item_modules,
                        )?),
                        consequent: Box::new(canonicalize_identifiers_of_instruction(
                            &original_case.consequent,
                            module_name_path,
                            rename_item_modules,
                        )?),
                    };
                    canonical_cases.push(canonical_case);
                }
                canonical_cases
            },
            default: if let Some(default_instruction) = default {
                let canonical_instruction = canonicalize_identifiers_of_instruction(
                    default_instruction,
                    module_name_path,
                    rename_item_modules,
                )?;

                Some(Box::new(canonical_instruction))
            } else {
                None
            },
        },
        Instruction::For {
            params,
            results,
            locals,
            code,
        } => Instruction::For {
            params: params.clone(),
            results: results.clone(),
            locals: locals.clone(),
            code: Box::new(canonicalize_identifiers_of_instruction(
                code,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::Do(instructions) => Instruction::Do(canonicalize_identifiers_of_instructions(
            instructions,
            module_name_path,
            rename_item_modules,
        )?),
        Instruction::Break(instructions) => {
            Instruction::Break(canonicalize_identifiers_of_instructions(
                instructions,
                module_name_path,
                rename_item_modules,
            )?)
        }
        Instruction::Recur(instructions) => {
            Instruction::Recur(canonicalize_identifiers_of_instructions(
                instructions,
                module_name_path,
                rename_item_modules,
            )?)
        }
        Instruction::Return(instructions) => {
            Instruction::Return(canonicalize_identifiers_of_instructions(
                instructions,
                module_name_path,
                rename_item_modules,
            )?)
        }
        Instruction::Rerun(instructions) => {
            Instruction::Rerun(canonicalize_identifiers_of_instructions(
                instructions,
                module_name_path,
                rename_item_modules,
            )?)
        }
        Instruction::Call {
            id: name_path,
            args,
        } => Instruction::Call {
            id: canonicalize_name_path_in_instruction(
                RenameKind::Function,
                name_path,
                module_name_path,
                rename_item_modules,
            )?,
            args: canonicalize_identifiers_of_instructions(
                args,
                module_name_path,
                rename_item_modules,
            )?,
        },
        Instruction::DynCall { num, args } => Instruction::DynCall {
            num: Box::new(canonicalize_identifiers_of_instruction(
                num,
                module_name_path,
                rename_item_modules,
            )?),
            args: canonicalize_identifiers_of_instructions(
                args,
                module_name_path,
                rename_item_modules,
            )?,
        },
        Instruction::EnvCall { num, args } => Instruction::EnvCall {
            num: *num,
            args: canonicalize_identifiers_of_instructions(
                args,
                module_name_path,
                rename_item_modules,
            )?,
        },
        Instruction::SysCall { num, args } => Instruction::SysCall {
            num: *num,
            args: canonicalize_identifiers_of_instructions(
                args,
                module_name_path,
                rename_item_modules,
            )?,
        },
        Instruction::ExtCall {
            id: name_path,
            args,
        } => Instruction::ExtCall {
            id: canonicalize_name_path_in_instruction(
                RenameKind::ExternalFunction,
                name_path,
                module_name_path,
                rename_item_modules,
            )?,
            args: canonicalize_identifiers_of_instructions(
                args,
                module_name_path,
                rename_item_modules,
            )?,
        },
        Instruction::Debug(_) => instruction.clone(),
        Instruction::Unreachable(_) => instruction.clone(),
        Instruction::HostAddrFunction { id } => Instruction::HostAddrFunction {
            id: canonicalize_name_path_in_instruction(
                RenameKind::Function,
                id,
                module_name_path,
                rename_item_modules,
            )?,
        },
        Instruction::MacroGetFunctionPublicIndex { id } => {
            Instruction::MacroGetFunctionPublicIndex {
                id: canonicalize_name_path_in_instruction(
                    RenameKind::Function,
                    id,
                    module_name_path,
                    rename_item_modules,
                )?,
            }
        }
    };

    Ok(canonical_instruction)
}

// find the canonical name path of the import functions, import data and external functions
fn rename(rename_kind: RenameKind, name: &str, rename_items: &[RenameItem]) -> Option<String> {
    rename_items
        .iter()
        .find(|item| item.rename_kind == rename_kind && item.from == name)
        .map(|item| item.to.clone())
}

fn get_rename_items_by_target_module_name_path<'a>(
    rename_item_modules: &'a [RenameItemModule],
    target_module_name_path: &str,
) -> &'a [RenameItem] {
    let rename_item_module_opt = rename_item_modules
        .iter()
        .find(|module| module.module_name_path == target_module_name_path);
    match rename_item_module_opt {
        Some(rename_item_module) => &rename_item_module.items,
        None => &[],
    }
}

// - canonicalize the function call and data load/store name path
// - rename the import function and import data identifier
// - rename the external function identifier.
fn canonicalize_name_path_in_instruction(
    rename_kind: RenameKind,
    name_path: &str,
    current_module_name_path: &str,
    rename_item_modules: &[RenameItemModule],
) -> Result<String, AssembleError> {
    let name_parts = name_path.split(NAME_PATH_SEPARATOR).collect::<Vec<&str>>();

    if name_parts.len() == 1 {
        let rename_items = get_rename_items_by_target_module_name_path(
            rename_item_modules,
            current_module_name_path,
        );

        let actual_name_path_opt = rename(rename_kind, name_path, rename_items);

        match actual_name_path_opt {
            Some(actual_name_path) => Ok(actual_name_path),
            None => {
                if rename_kind == RenameKind::ExternalFunction {
                    Err(AssembleError {
                        message: format!(
                            "Can not find the external function: {}, current module: {}",
                            name_path, current_module_name_path
                        ),
                    })
                } else {
                    Ok(format!("{}::{}", current_module_name_path, name_path))
                }
            }
        }
    } else {
        let mut canonical_parts = vec![];

        let first_part = name_parts[0];
        canonical_parts.push(match first_part {
            "module" => current_module_name_path
                .split(NAME_PATH_SEPARATOR)
                .next()
                .unwrap()
                .to_owned(),
            "self" => current_module_name_path.to_owned(),
            _ => first_part.to_owned(),
        });

        if name_parts.len() > 2 {
            name_parts[1..(name_parts.len() - 1)]
                .iter()
                .for_each(|s| canonical_parts.push(s.to_string()));
        }

        let target_module_name_path = canonical_parts.join(NAME_PATH_SEPARATOR);

        let rename_items = get_rename_items_by_target_module_name_path(
            rename_item_modules,
            &target_module_name_path,
        );

        let last_part = name_parts[name_parts.len() - 1];
        let actual_name_path_opt = rename(rename_kind, last_part, rename_items);

        match actual_name_path_opt {
            Some(actual_name_path) => Ok(actual_name_path),
            None => {
                if rename_kind == RenameKind::ExternalFunction {
                    Err(AssembleError {
                        message: format!(
                            "Can not find the external function: {}, current module: {}",
                            name_path, current_module_name_path
                        ),
                    })
                } else {
                    Ok(format!("{}::{}", target_module_name_path, last_part))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ancvm_types::{opcode::Opcode, ExternalLibraryType, MemoryDataType, ModuleShareType};
    use pretty_assertions::assert_eq;

    use ancasm_parser::{
        ast::{
            DataKindNode, ExternalFunctionNode, ExternalItem, ExternalLibraryNode, ExternalNode,
            ImportDataNode, ImportFunctionNode, ImportItem, ImportModuleNode, ImportNode,
            InitedData, Instruction, SimplifiedDataKindNode, UninitData,
        },
        lexer::lex,
        parser::parse,
        peekable_iterator::PeekableIterator,
    };

    use crate::preprocessor::{CanonicalDataNode, CanonicalFunctionNode};

    use super::{merge_and_canonicalize_submodule_nodes, MergedModuleNode};

    fn preprocess_from_strs(submodule_sources: &[&str]) -> MergedModuleNode {
        let submodule_nodes = submodule_sources
            .iter()
            .map(|source| {
                let mut chars = source.chars();
                let mut char_iter = PeekableIterator::new(&mut chars, 2);
                let mut tokens = lex(&mut char_iter).unwrap().into_iter();
                let mut token_iter = PeekableIterator::new(&mut tokens, 2);
                parse(&mut token_iter).unwrap()
            })
            .collect::<Vec<_>>();

        merge_and_canonicalize_submodule_nodes(&submodule_nodes).unwrap()
    }

    #[test]
    fn test_preprocess_merge_functions() {
        assert_eq!(
            preprocess_from_strs(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (function $entry (code))
                (function $main (code))
            )
            "#,
                r#"
            (module $myapp::utils
                (runtime_version "1.0")
                (function $add (code))
                (function $sub (code))
            )
            "#
            ]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name: None,
                destructor_function_name: None,
                function_nodes: vec![
                    CanonicalFunctionNode {
                        id: "myapp::entry".to_owned(),
                        name_path: "entry".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![]
                    },
                    CanonicalFunctionNode {
                        id: "myapp::main".to_owned(),
                        name_path: "main".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![]
                    },
                    CanonicalFunctionNode {
                        id: "myapp::utils::add".to_owned(),
                        name_path: "utils::add".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![]
                    },
                    CanonicalFunctionNode {
                        id: "myapp::utils::sub".to_owned(),
                        name_path: "utils::sub".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![]
                    }
                ],
                read_only_data_nodes: vec![],
                read_write_data_nodes: vec![],
                uninit_data_nodes: vec![],
                external_nodes: vec![],
                import_nodes: vec![]
            }
        )
    }

    #[test]
    fn test_preprocess_merge_datas() {
        assert_eq!(
            preprocess_from_strs(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (data $code (read_only i32 0x11))
            )
            "#,
                r#"
            (module $myapp::utils
                (runtime_version "1.0")
                (data $count (read_write i32 0x13))
                (data $sum (uninit i32))
            )
            "#
            ]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name: None,
                destructor_function_name: None,
                function_nodes: vec![],
                read_only_data_nodes: vec![CanonicalDataNode {
                    id: "myapp::code".to_owned(),
                    name_path: "code".to_owned(),
                    export: false,
                    data_kind: DataKindNode::ReadOnly(InitedData {
                        memory_data_type: MemoryDataType::I32,
                        length: 4,
                        align: 4,
                        value: 0x11u32.to_le_bytes().to_vec()
                    })
                }],
                read_write_data_nodes: vec![CanonicalDataNode {
                    id: "myapp::utils::count".to_owned(),
                    name_path: "utils::count".to_owned(),
                    export: false,
                    data_kind: DataKindNode::ReadWrite(InitedData {
                        memory_data_type: MemoryDataType::I32,
                        length: 4,
                        align: 4,
                        value: 0x13u32.to_le_bytes().to_vec()
                    })
                }],
                uninit_data_nodes: vec![CanonicalDataNode {
                    id: "myapp::utils::sum".to_owned(),
                    name_path: "utils::sum".to_owned(),
                    export: false,
                    data_kind: DataKindNode::Uninit(UninitData {
                        memory_data_type: MemoryDataType::I32,
                        length: 4,
                        align: 4,
                    })
                }],
                external_nodes: vec![],
                import_nodes: vec![]
            }
        )
    }

    #[test]
    fn test_preprocess_canonicalize_identifiers_of_function_instructions() {
        assert_eq!(
            preprocess_from_strs(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (function $add (code))
                (function $test (code
                    ;; group 2
                    (call $add)
                    (call $myapp::add)
                    (call $module::add)
                    (call $self::add)
                    ;; group 3
                    (call $myapp::utils::add)
                    (call $module::utils::add)
                    (call $self::utils::add)
                ))
            )
            "#,
                r#"
            (module $myapp::utils
                (runtime_version "1.0")
                (function $add (code))
                (function $test (code
                    ;; group 2
                    (call $myapp::add)
                    (call $module::add)
                    ;; group 3
                    (call $add)
                    (call $self::add)
                    (call $myapp::utils::add)
                    (call $module::utils::add)
                ))
            )
            "#
            ]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name: None,
                destructor_function_name: None,
                function_nodes: vec![
                    CanonicalFunctionNode {
                        id: "myapp::add".to_owned(),
                        name_path: "add".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![]
                    },
                    CanonicalFunctionNode {
                        id: "myapp::test".to_owned(),
                        name_path: "test".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![
                            // group 2
                            Instruction::Call {
                                id: "myapp::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "myapp::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "myapp::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "myapp::add".to_owned(),
                                args: vec![]
                            },
                            // group 3
                            Instruction::Call {
                                id: "myapp::utils::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "myapp::utils::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "myapp::utils::add".to_owned(),
                                args: vec![]
                            },
                        ]
                    },
                    CanonicalFunctionNode {
                        id: "myapp::utils::add".to_owned(),
                        name_path: "utils::add".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![]
                    },
                    CanonicalFunctionNode {
                        id: "myapp::utils::test".to_owned(),
                        name_path: "utils::test".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![
                            // group 2
                            Instruction::Call {
                                id: "myapp::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "myapp::add".to_owned(),
                                args: vec![]
                            },
                            // group 3
                            Instruction::Call {
                                id: "myapp::utils::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "myapp::utils::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "myapp::utils::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "myapp::utils::add".to_owned(),
                                args: vec![]
                            },
                        ]
                    }
                ],
                read_only_data_nodes: vec![],
                read_write_data_nodes: vec![],
                uninit_data_nodes: vec![],
                external_nodes: vec![],
                import_nodes: vec![]
            }
        )
    }

    #[test]
    fn test_preprocess_canonicalize_identifiers_of_data_instructions() {
        assert_eq!(
            preprocess_from_strs(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (data $d0 (read_only i32 0x11))
                (function $test (code
                    ;; group 0
                    (data.load32_i32 $d0)
                    (data.load32_i32 $myapp::d0)
                    (data.load32_i32 $module::d0)
                    (data.load32_i32 $self::d0)
                    ;; group 1
                    (data.load64_i64 $myapp::utils::d0)
                    (data.load64_i64 $module::utils::d0)
                    (data.load64_i64 $self::utils::d0)
                ))
            )
            "#,
                r#"
            (module $myapp::utils
                (runtime_version "1.0")
                (data $d0 (read_only i64 0x13))
                (function $test (code
                    ;; group 0
                    (data.load32_i32 $myapp::d0)
                    (data.load32_i32 $module::d0)
                    ;; group 1
                    (data.load64_i64 $d0)
                    (data.load64_i64 $self::d0)
                    (data.load64_i64 $myapp::utils::d0)
                    (data.load64_i64 $module::utils::d0)
                ))
            )
            "#
            ]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name: None,
                destructor_function_name: None,
                function_nodes: vec![
                    CanonicalFunctionNode {
                        id: "myapp::test".to_owned(),
                        name_path: "test".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![
                            // group 0
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "myapp::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "myapp::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "myapp::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "myapp::d0".to_owned(),
                                offset: 0
                            },
                            // group 1
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                id: "myapp::utils::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                id: "myapp::utils::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                id: "myapp::utils::d0".to_owned(),
                                offset: 0
                            },
                        ]
                    },
                    CanonicalFunctionNode {
                        id: "myapp::utils::test".to_owned(),
                        name_path: "utils::test".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![
                            // group 0
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "myapp::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "myapp::d0".to_owned(),
                                offset: 0
                            },
                            // group 1
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                id: "myapp::utils::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                id: "myapp::utils::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                id: "myapp::utils::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                id: "myapp::utils::d0".to_owned(),
                                offset: 0
                            },
                        ]
                    }
                ],
                read_only_data_nodes: vec![
                    CanonicalDataNode {
                        id: "myapp::d0".to_owned(),
                        name_path: "d0".to_owned(),
                        export: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4,
                            value: 0x11u32.to_le_bytes().to_vec()
                        })
                    },
                    CanonicalDataNode {
                        id: "myapp::utils::d0".to_owned(),
                        name_path: "utils::d0".to_owned(),
                        export: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::I64,
                            length: 8,
                            align: 8,
                            value: 0x13u64.to_le_bytes().to_vec()
                        })
                    }
                ],
                read_write_data_nodes: vec![],
                uninit_data_nodes: vec![],
                external_nodes: vec![],
                import_nodes: vec![]
            }
        )
    }

    #[test]
    fn test_preprocess_canonicalize_identifiers_of_external_functions() {
        assert_eq!(
            preprocess_from_strs(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (external (library user "math.so.1")
                    (function $add "add")
                    (function $sub "sub_wrap")
                )
                (external (library share "std.so.1")
                    (function $print "print")
                )
                (function $test (code
                    ;; group 0
                    (extcall $add)
                    (extcall $myapp::add)
                    (extcall $module::add)
                    (extcall $self::add)

                    ;; group 1
                    (extcall $sub)
                    (extcall $print)

                    ;; group 2
                    (extcall $myapp::utils::f0)     ;; add
                    (extcall $module::utils::f0)    ;; add
                    (extcall $self::utils::f0)      ;; add

                    ;; group 3
                    (extcall $myapp::utils::f1)     ;; mul
                    (extcall $myapp::utils::f2)     ;; print
                    (extcall $myapp::utils::f3)     ;; getuid
                ))
            )
            "#,
                r#"
            (module $myapp::utils
                (runtime_version "1.0")
                (external (library user "math.so.1")
                    (function $f0 "add")      ;; duplicate
                    (function $f1 "mul")      ;; new
                )
                (external (library share "std.so.1")
                    (function $f2 "print")    ;; duplicate
                )
                (external (library system "libc.so.6")  ;; new
                    (function $f3 "getuid")             ;; new
                )
                (function $test (code
                    ;; group 0
                    (extcall $f0)
                    (extcall $myapp::utils::f0)
                    (extcall $module::utils::f0)
                    (extcall $self::f0)

                    ;; group 1
                    (extcall $f1)
                    (extcall $f2)
                    (extcall $f3)

                    ;; group 2
                    (extcall $myapp::add)
                    (extcall $module::add)
                ))
            )
            "#
            ]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name: None,
                destructor_function_name: None,
                function_nodes: vec![
                    CanonicalFunctionNode {
                        id: "myapp::test".to_owned(),
                        name_path: "test".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![
                            // group 0
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            // group 1
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::sub_wrap".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::share::std.so.1::print".to_owned(),
                                args: vec![]
                            },
                            // group 2
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            // group 3
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::mul".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::share::std.so.1::print".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::system::libc.so.6::getuid".to_owned(),
                                args: vec![]
                            },
                        ]
                    },
                    CanonicalFunctionNode {
                        id: "myapp::utils::test".to_owned(),
                        name_path: "utils::test".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![
                            // group 0
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            // group 1
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::mul".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::share::std.so.1::print".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::system::libc.so.6::getuid".to_owned(),
                                args: vec![]
                            },
                            // group 2
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                        ]
                    },
                ],
                read_only_data_nodes: vec![],
                read_write_data_nodes: vec![],
                uninit_data_nodes: vec![],
                external_nodes: vec![
                    ExternalNode {
                        external_library_node: ExternalLibraryNode {
                            external_library_type: ExternalLibraryType::User,
                            name: "math.so.1".to_owned()
                        },
                        external_items: vec![
                            ExternalItem::ExternalFunction(ExternalFunctionNode {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                name: "add".to_owned(),
                                params: vec![],
                                results: vec![]
                            }),
                            ExternalItem::ExternalFunction(ExternalFunctionNode {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::sub_wrap".to_owned(),
                                name: "sub_wrap".to_owned(),
                                params: vec![],
                                results: vec![]
                            }),
                            ExternalItem::ExternalFunction(ExternalFunctionNode {
                                id: "EXTERNAL_FUNCTION::user::math.so.1::mul".to_owned(),
                                name: "mul".to_owned(),
                                params: vec![],
                                results: vec![]
                            }),
                        ]
                    },
                    ExternalNode {
                        external_library_node: ExternalLibraryNode {
                            external_library_type: ExternalLibraryType::Share,
                            name: "std.so.1".to_owned()
                        },
                        external_items: vec![ExternalItem::ExternalFunction(
                            ExternalFunctionNode {
                                id: "EXTERNAL_FUNCTION::share::std.so.1::print".to_owned(),
                                name: "print".to_owned(),
                                params: vec![],
                                results: vec![]
                            }
                        ),]
                    },
                    ExternalNode {
                        external_library_node: ExternalLibraryNode {
                            external_library_type: ExternalLibraryType::System,
                            name: "libc.so.6".to_owned()
                        },
                        external_items: vec![ExternalItem::ExternalFunction(
                            ExternalFunctionNode {
                                id: "EXTERNAL_FUNCTION::system::libc.so.6::getuid".to_owned(),
                                name: "getuid".to_owned(),
                                params: vec![],
                                results: vec![]
                            }
                        ),]
                    }
                ],
                import_nodes: vec![]
            }
        )
    }

    #[test]
    fn test_preprocess_canonicalize_identifiers_of_import_functions() {
        assert_eq!(
            preprocess_from_strs(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (import (module share "math" "1.0")
                    (function $add "add")
                    (function $sub "wrap::sub")
                )
                (import (module user "format" "1.2")
                    (function $print "print")
                )
                (function $test (code
                    ;; group 0
                    (call $add)         ;; math::add
                    (call $myapp::add)  ;; math::add
                    (call $module::add) ;; math::add
                    (call $self::add)   ;; math::add

                    ;; group 1
                    (call $sub)         ;; math::wrap::sub
                    (call $print)       ;; format::print

                    ;; group 2
                    (call $myapp::utils::f0)     ;; math::add
                    (call $module::utils::f0)    ;; math::add
                    (call $self::utils::f0)      ;; math::add

                    ;; group 3
                    (call $myapp::utils::f1)     ;; math::mul
                    (call $myapp::utils::f2)     ;; format::print
                    (call $myapp::utils::f3)     ;; random::rand
                ))
            )
            "#,
                r#"
            (module $myapp::utils
                (runtime_version "1.0")
                (import (module share "math" "1.0")
                    (function $f0 "add")      ;; duplicate
                    (function $f1 "mul")      ;; new
                )
                (import (module user "format" "1.2")
                    (function $f2 "print")    ;; duplicate
                )
                (import (module share "random" "2.3")   ;; new
                    (function $f3 "rand")               ;; new
                )
                (function $test (code
                    ;; group 0
                    (call $f0)                  ;; math::add
                    (call $myapp::utils::f0)    ;; math::add
                    (call $module::utils::f0)   ;; math::add
                    (call $self::f0)            ;; math::add

                    ;; group 1
                    (call $f1)  ;; math::mul
                    (call $f2)  ;; format::print
                    (call $f3)  ;; random::rand

                    ;; group 2
                    (call $myapp::add)  ;; math::add
                    (call $module::add) ;; math::add
                ))
            )
            "#
            ]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name: None,
                destructor_function_name: None,
                function_nodes: vec![
                    CanonicalFunctionNode {
                        id: "myapp::test".to_owned(),
                        name_path: "test".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![
                            // group 0
                            Instruction::Call {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            // group 1
                            Instruction::Call {
                                id: "math::wrap::sub".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "format::print".to_owned(),
                                args: vec![]
                            },
                            // group 2
                            Instruction::Call {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            // group 3
                            Instruction::Call {
                                id: "math::mul".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "format::print".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "random::rand".to_owned(),
                                args: vec![]
                            },
                        ]
                    },
                    CanonicalFunctionNode {
                        id: "myapp::utils::test".to_owned(),
                        name_path: "utils::test".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![
                            // group 0
                            Instruction::Call {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            // group 1
                            Instruction::Call {
                                id: "math::mul".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "format::print".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "random::rand".to_owned(),
                                args: vec![]
                            },
                            // group 2
                            Instruction::Call {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                        ]
                    },
                ],
                read_only_data_nodes: vec![],
                read_write_data_nodes: vec![],
                uninit_data_nodes: vec![],
                external_nodes: vec![],
                import_nodes: vec![
                    ImportNode {
                        import_module_node: ImportModuleNode {
                            module_share_type: ModuleShareType::Share,
                            name: "math".to_owned(),
                            version_major: 1,
                            version_minor: 0
                        },
                        import_items: vec![
                            ImportItem::ImportFunction(ImportFunctionNode {
                                id: "math::add".to_owned(),
                                name_path: "add".to_owned(),
                                params: vec![],
                                results: vec![]
                            }),
                            ImportItem::ImportFunction(ImportFunctionNode {
                                id: "math::wrap::sub".to_owned(),
                                name_path: "wrap::sub".to_owned(),
                                params: vec![],
                                results: vec![]
                            }),
                            ImportItem::ImportFunction(ImportFunctionNode {
                                id: "math::mul".to_owned(),
                                name_path: "mul".to_owned(),
                                params: vec![],
                                results: vec![]
                            }),
                        ]
                    },
                    ImportNode {
                        import_module_node: ImportModuleNode {
                            module_share_type: ModuleShareType::User,
                            name: "format".to_owned(),
                            version_major: 1,
                            version_minor: 2
                        },
                        import_items: vec![ImportItem::ImportFunction(ImportFunctionNode {
                            id: "format::print".to_owned(),
                            name_path: "print".to_owned(),
                            params: vec![],
                            results: vec![]
                        }),]
                    },
                    ImportNode {
                        import_module_node: ImportModuleNode {
                            module_share_type: ModuleShareType::Share,
                            name: "random".to_owned(),
                            version_major: 2,
                            version_minor: 3
                        },
                        import_items: vec![ImportItem::ImportFunction(ImportFunctionNode {
                            id: "random::rand".to_owned(),
                            name_path: "rand".to_owned(),
                            params: vec![],
                            results: vec![]
                        }),]
                    }
                ]
            }
        )
    }

    #[test]
    fn test_preprocess_canonicalize_identifiers_of_import_data() {
        assert_eq!(
            preprocess_from_strs(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (import (module share "math" "1.0")
                    (data $count "count" (read_only i32))
                    (data $sum "wrap::sum" (read_write i64))
                )
                (import (module user "match" "1.2")
                    (data $score "score" (read_write f32))
                )
                (function $test (code
                    ;; group 0
                    (data.load32_i32 $count)           ;; math::count
                    (data.load32_i32 $myapp::count)    ;; math::count
                    (data.load32_i32 $module::count)   ;; math::count
                    (data.load32_i32 $self::count)     ;; math::count

                    ;; group 1
                    (data.load64_i64 $sum)         ;; math::wrap::sum
                    (data.load32_f32 $score)       ;; match::score

                    ;; group 2
                    (data.load32_i32 $myapp::utils::d0)     ;; math::count
                    (data.load32_i32 $module::utils::d0)    ;; math::count
                    (data.load32_i32 $self::utils::d0)      ;; math::count

                    ;; group 3
                    (data.load32_i32 $myapp::utils::d1)     ;; math::increment
                    (data.load32_f32 $myapp::utils::d2)     ;; match::score
                    (data.load64_i64 $myapp::utils::d3)     ;; random::seed
                ))
            )
            "#,
                r#"
            (module $myapp::utils
                (runtime_version "1.0")
                (import (module share "math" "1.0")
                    (data $d0 "count" (read_only i32))  ;; duplicate
                    (data $d1 "increment" (uninit i32)) ;; new
                )
                (import (module user "match" "1.2")
                    (data $d2 "score" (read_write f32)) ;; duplicate
                )
                (import (module share "random" "2.3")   ;; new
                    (data $d3 "seed" (read_write i64))  ;; new
                )
                (function $test (code
                    ;; group 0
                    (data.load32_i32 $d0)                  ;; math::count
                    (data.load32_i32 $myapp::utils::d0)    ;; math::count
                    (data.load32_i32 $module::utils::d0)   ;; math::count
                    (data.load32_i32 $self::d0)            ;; math::count

                    ;; group 1
                    (data.load32_i32 $d1)  ;; math::increment
                    (data.load32_f32 $d2)  ;; match::score
                    (data.load64_i64 $d3)  ;; random::seed

                    ;; group 2
                    (data.load32_i32 $myapp::count)    ;; math::count
                    (data.load32_i32 $module::count)   ;; math::count
                ))
            )
            "#
            ]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name: None,
                destructor_function_name: None,
                function_nodes: vec![
                    CanonicalFunctionNode {
                        id: "myapp::test".to_owned(),
                        name_path: "test".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![
                            // group 0
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::count".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::count".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::count".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::count".to_owned(),
                                offset: 0
                            },
                            // group 1
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                id: "math::wrap::sum".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_f32,
                                id: "match::score".to_owned(),
                                offset: 0
                            },
                            // group 2
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::count".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::count".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::count".to_owned(),
                                offset: 0
                            },
                            // group 3
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::increment".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_f32,
                                id: "match::score".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                id: "random::seed".to_owned(),
                                offset: 0
                            },
                        ]
                    },
                    CanonicalFunctionNode {
                        id: "myapp::utils::test".to_owned(),
                        name_path: "utils::test".to_owned(),
                        export: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![
                            // group 0
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::count".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::count".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::count".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::count".to_owned(),
                                offset: 0
                            },
                            // group 1
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::increment".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_f32,
                                id: "match::score".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                id: "random::seed".to_owned(),
                                offset: 0
                            },
                            // group 2
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::count".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                id: "math::count".to_owned(),
                                offset: 0
                            },
                        ]
                    },
                ],
                read_only_data_nodes: vec![],
                read_write_data_nodes: vec![],
                uninit_data_nodes: vec![],
                external_nodes: vec![],
                import_nodes: vec![
                    ImportNode {
                        import_module_node: ImportModuleNode {
                            module_share_type: ModuleShareType::Share,
                            name: "math".to_owned(),
                            version_major: 1,
                            version_minor: 0
                        },
                        import_items: vec![
                            ImportItem::ImportData(ImportDataNode {
                                id: "math::count".to_owned(),
                                name_path: "count".to_owned(),
                                data_kind_node: SimplifiedDataKindNode::ReadOnly(
                                    MemoryDataType::I32
                                )
                            }),
                            ImportItem::ImportData(ImportDataNode {
                                id: "math::wrap::sum".to_owned(),
                                name_path: "wrap::sum".to_owned(),
                                data_kind_node: SimplifiedDataKindNode::ReadWrite(
                                    MemoryDataType::I64
                                )
                            }),
                            ImportItem::ImportData(ImportDataNode {
                                id: "math::increment".to_owned(),
                                name_path: "increment".to_owned(),
                                data_kind_node: SimplifiedDataKindNode::Uninit(MemoryDataType::I32)
                            }),
                        ]
                    },
                    ImportNode {
                        import_module_node: ImportModuleNode {
                            module_share_type: ModuleShareType::User,
                            name: "match".to_owned(),
                            version_major: 1,
                            version_minor: 2
                        },
                        import_items: vec![ImportItem::ImportData(ImportDataNode {
                            id: "match::score".to_owned(),
                            name_path: "score".to_owned(),
                            data_kind_node: SimplifiedDataKindNode::ReadWrite(MemoryDataType::F32)
                        }),]
                    },
                    ImportNode {
                        import_module_node: ImportModuleNode {
                            module_share_type: ModuleShareType::Share,
                            name: "random".to_owned(),
                            version_major: 2,
                            version_minor: 3
                        },
                        import_items: vec![ImportItem::ImportData(ImportDataNode {
                            id: "random::seed".to_owned(),
                            name_path: "seed".to_owned(),
                            data_kind_node: SimplifiedDataKindNode::ReadWrite(MemoryDataType::I64)
                        }),]
                    }
                ]
            }
        )
    }
}
