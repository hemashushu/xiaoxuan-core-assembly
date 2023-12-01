// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_parser::{
    ast::{
        BranchCase, DataKindNode, DataNode, ExternalFunctionNode, ExternalItem, ExternalNode,
        FunctionNode, Instruction, ModuleElementNode, ModuleNode,
    },
    NAME_PATH_SEPARATOR,
};

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
// rewritten as 'IMPORT_FUNCTION::share::math::add' in both the node 'import' and 'call'
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

    pub function_nodes: Vec<FunctionNode>,
    pub read_only_data_nodes: Vec<DataNode>,
    pub read_write_data_nodes: Vec<DataNode>,
    pub uninit_data_nodes: Vec<DataNode>,
    pub external_nodes: Vec<ExternalNode>,
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

    kind: RenameKind,
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

pub fn canonicalize_submodule_nodes(
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
    let mut canonical_external_nodes: Vec<ExternalNode> = vec![];

    // canonicalize the external nodes
    // remove the duplicate external items and group by external library

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
                    ExternalItem::ExternalFunction(original_external_func) => {
                        // the format of expect identifier name:
                        // EXTERNAL_FUNCTION::EXTERNAL_LIBRARY_TYPE::LIBRARY_SO_NAME::SYMBOL_NAME

                        let name = original_external_func.name.clone();

                        let expect_identifier = format!(
                            "EXTERNAL_FUNCTION::{}::{}::{}",
                            original_external_node
                                .external_library_node
                                .external_library_type,
                            original_external_node.external_library_node.name,
                            name
                        );

                        //                         let actual_identifier = original_external_func.id.clone();
                        //
                        //                         if expect_identifier != actual_identifier {
                        //                             let rename_item = RenameItem {
                        //                                 from: format!("{}::{}",submodule_name_path,  actual_identifier),
                        //                                 to: expect_identifier.clone(),
                        //                                 kind: RenameKind::ExternalFunction,
                        //                             };
                        //                             rename_items.push(rename_item);
                        //                         }

                        let canonical_external_function_node = ExternalFunctionNode {
                            id: expect_identifier,
                            name,
                            params: original_external_func.params.clone(),
                            results: original_external_func.results.clone(),
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
                    && rename_items
                        .iter()
                        .find(|item| item.from == actual_identifier && item.to == expect_identifier)
                        .is_none()
                {
                    let rename_item = RenameItem {
                        from: actual_identifier,
                        to: expect_identifier,
                        kind: RenameKind::ExternalFunction,
                    };
                    rename_items.push(rename_item);
                }
            }
        }

        let rename_item_module = RenameItemModule {
            module_name_path: submodule_name_path.to_owned(),
            items: rename_items,
        };

        rename_item_modules.push(rename_item_module);
    }

    // todo::
    // canonicalize the import nodes

    let mut canonical_function_nodes: Vec<FunctionNode> = vec![];
    let mut canonical_read_only_data_nodes: Vec<DataNode> = vec![];
    let mut canonical_read_write_data_nodes: Vec<DataNode> = vec![];
    let mut canonical_uninit_data_nodes: Vec<DataNode> = vec![];

    for module_idx in 0..submodule_nodes.len() {
        let module_node = &submodule_nodes[module_idx];
        let module_name_path = &module_node.name_path;
        // let rename_items = &rename_item_modules[module_idx].items;

        // canonicalize the func nodes
        let original_function_nodes = module_node
            .element_nodes
            .iter()
            .filter_map(|node| match node {
                ModuleElementNode::FunctionNode(function_node) => Some(function_node),
                _ => None,
            })
            // .map(|function_node| {
            //     canonicalize_function_node(function_node, module_name_path, &rename_item_modules)
            // })
            .collect::<Vec<_>>();

        let mut function_nodes = vec![];
        for original_function_node in original_function_nodes {
            let function_node = canonicalize_function_node(
                original_function_node,
                module_name_path,
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
            .map(|data_node| canonicalize_data_node(data_node, module_name_path))
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
            .map(|data_node| canonicalize_data_node(data_node, module_name_path))
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
            .map(|data_node| canonicalize_data_node(data_node, module_name_path))
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
    };

    Ok(merged_module_node)
}

fn canonicalize_function_node(
    function_node: &FunctionNode,
    module_name_path: &str,
    rename_item_modules: &[RenameItemModule],
) -> Result<FunctionNode, AssembleError> {
    let function_full_name = format!("{}::{}", module_name_path, function_node.name);
    let canonical_code = canonicalize_identifiers_of_instructions(
        &function_node.code,
        module_name_path,
        rename_item_modules,
    )?;

    Ok(FunctionNode {
        name: function_full_name,
        exported: function_node.exported,
        params: function_node.params.clone(),
        results: function_node.results.clone(),
        locals: function_node.locals.clone(),
        code: canonical_code,
    })
}

fn canonicalize_data_node(data_node: &DataNode, module_name_path: &str) -> DataNode {
    let data_full_name = format!("{}::{}", module_name_path, data_node.name);

    DataNode {
        name: data_full_name,
        exported: data_node.exported,
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
            name_path,
            offset,
        } => Instruction::DataLoad {
            opcode: *opcode,
            name_path: canonicalize_name_path_in_instruction(
                RenameKind::Data,
                name_path,
                module_name_path,
                rename_item_modules,
            )?,
            offset: *offset,
        },
        Instruction::DataStore {
            opcode,
            name_path,
            offset,
            value,
        } => Instruction::DataStore {
            opcode: *opcode,
            name_path: canonicalize_name_path_in_instruction(
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
            name_path,
            offset,
        } => Instruction::DataLongLoad {
            opcode: *opcode,
            name_path: canonicalize_name_path_in_instruction(
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
            name_path,
            offset,
            value,
        } => Instruction::DataLongStore {
            opcode: *opcode,
            name_path: canonicalize_name_path_in_instruction(
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
        Instruction::Call { name_path, args } => Instruction::Call {
            name_path: canonicalize_name_path_in_instruction(
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
        Instruction::ExtCall { name_path, args } => Instruction::ExtCall {
            name_path: canonicalize_name_path_in_instruction(
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
        Instruction::HostAddrFunction(name_path) => {
            Instruction::HostAddrFunction(canonicalize_name_path_in_instruction(
                RenameKind::Function,
                name_path,
                module_name_path,
                rename_item_modules,
            )?)
        }
        Instruction::MacroGetFunctionPublicIndex(name_path) => {
            Instruction::MacroGetFunctionPublicIndex(canonicalize_name_path_in_instruction(
                RenameKind::Function,
                name_path,
                module_name_path,
                rename_item_modules,
            )?)
        }
    };

    Ok(canonical_instruction)
}

fn rename(rename_kind: RenameKind, name: &str, rename_items: &[RenameItem]) -> Option<String> {
    let idx_opt = rename_items
        .iter()
        .position(|item| item.kind == rename_kind && item.from == name);

    rename_items
        .iter()
        .find(|item| item.kind == rename_kind && item.from == name)
        .map(|item| item.to.clone())
}

fn get_rename_items_by_target_module_name_path<'a, 'b>(
    rename_item_modules: &'a [RenameItemModule],
    target_module_name_path: &'b str,
) -> &'a [RenameItem] {
    let rename_item_module_opt = rename_item_modules
        .iter()
        .find(|module| module.module_name_path == target_module_name_path);
    match rename_item_module_opt {
        Some(rename_item_module) => &rename_item_module.items,
        None => &[],
    }
}

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

        let actual_name = rename(rename_kind, name_path, rename_items);

        if rename_kind == RenameKind::ExternalFunction {
            actual_name.ok_or(AssembleError {
                message: format!(
                    "Can not find the external function: {}, current module: {}",
                    name_path, current_module_name_path
                ),
            })
        } else {
            Ok(format!(
                "{}::{}",
                current_module_name_path,
                actual_name.unwrap_or(name_path.to_owned())
            ))
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
        let actual_name = rename(rename_kind, last_part, rename_items);

        if rename_kind == RenameKind::ExternalFunction {
            actual_name.ok_or(AssembleError {
                message: format!(
                    "Can not find the external function: {}, current module: {}",
                    name_path, current_module_name_path
                ),
            })
        } else {
            Ok(format!(
                "{}::{}",
                target_module_name_path,
                actual_name.unwrap_or(last_part.to_owned())
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use ancvm_types::{opcode::Opcode, ExternalLibraryType, MemoryDataType};
    use pretty_assertions::assert_eq;

    use ancvm_parser::{
        ast::{
            DataKindNode, DataNode, ExternalFunctionNode, ExternalItem, ExternalLibraryNode,
            ExternalNode, FunctionNode, InitedData, Instruction, UninitData,
        },
        lexer::lex,
        parser::parse,
        peekable_iterator::PeekableIterator,
    };

    use super::{canonicalize_submodule_nodes, MergedModuleNode};

    fn merge_submodules_from_strs(sources: &[&str]) -> MergedModuleNode {
        let submodule_nodes = sources
            .iter()
            .map(|source| {
                let mut chars = source.chars();
                let mut char_iter = PeekableIterator::new(&mut chars, 2);
                let mut tokens = lex(&mut char_iter).unwrap().into_iter();
                let mut token_iter = PeekableIterator::new(&mut tokens, 2);
                parse(&mut token_iter).unwrap()
            })
            .collect::<Vec<_>>();

        canonicalize_submodule_nodes(&submodule_nodes).unwrap()
    }

    #[test]
    fn test_preprocess_merge_functions_and_datas() {
        assert_eq!(
            merge_submodules_from_strs(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (data $code (read_only i32 0x11))
                (function $entry (code))
                (function $main (code))
            )
            "#,
                r#"
            (module $myapp::utils
                (runtime_version "1.0")
                (data $count (read_write i32 0x13))
                (data $sum (uninit i32))
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
                    FunctionNode {
                        name: "myapp::entry".to_owned(),
                        exported: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![]
                    },
                    FunctionNode {
                        name: "myapp::main".to_owned(),
                        exported: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![]
                    },
                    FunctionNode {
                        name: "myapp::utils::add".to_owned(),
                        exported: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![]
                    },
                    FunctionNode {
                        name: "myapp::utils::sub".to_owned(),
                        exported: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![]
                    }
                ],
                read_only_data_nodes: vec![DataNode {
                    name: "myapp::code".to_owned(),
                    exported: false,
                    data_kind: DataKindNode::ReadOnly(InitedData {
                        memory_data_type: MemoryDataType::I32,
                        length: 4,
                        align: 4,
                        value: 0x11u32.to_le_bytes().to_vec()
                    })
                }],
                read_write_data_nodes: vec![DataNode {
                    name: "myapp::utils::count".to_owned(),
                    exported: false,
                    data_kind: DataKindNode::ReadWrite(InitedData {
                        memory_data_type: MemoryDataType::I32,
                        length: 4,
                        align: 4,
                        value: 0x13u32.to_le_bytes().to_vec()
                    })
                }],
                uninit_data_nodes: vec![DataNode {
                    name: "myapp::utils::sum".to_owned(),
                    exported: false,
                    data_kind: DataKindNode::Uninit(UninitData {
                        memory_data_type: MemoryDataType::I32,
                        length: 4,
                        align: 4,
                    })
                }],
                external_nodes: vec![],
            }
        )
    }

    #[test]
    fn test_preprocess_canonicalize_identifiers_of_func_and_data_instructions() {
        assert_eq!(
            merge_submodules_from_strs(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (data $d0 (read_only i32 0x11))
                (function $add (code))
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
                (data $d0 (read_only i64 0x13))
                (function $add (code))
                (function $test (code
                    ;; group 0
                    (data.load32_i32 $myapp::d0)
                    (data.load32_i32 $module::d0)
                    ;; group 1
                    (data.load64_i64 $d0)
                    (data.load64_i64 $self::d0)
                    (data.load64_i64 $myapp::utils::d0)
                    (data.load64_i64 $module::utils::d0)
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
                    FunctionNode {
                        name: "myapp::add".to_owned(),
                        exported: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![]
                    },
                    FunctionNode {
                        name: "myapp::test".to_owned(),
                        exported: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![
                            // group 0
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                name_path: "myapp::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                name_path: "myapp::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                name_path: "myapp::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                name_path: "myapp::d0".to_owned(),
                                offset: 0
                            },
                            // group 1
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                name_path: "myapp::utils::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                name_path: "myapp::utils::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                name_path: "myapp::utils::d0".to_owned(),
                                offset: 0
                            },
                            // group 2
                            Instruction::Call {
                                name_path: "myapp::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                name_path: "myapp::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                name_path: "myapp::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                name_path: "myapp::add".to_owned(),
                                args: vec![]
                            },
                            // group 3
                            Instruction::Call {
                                name_path: "myapp::utils::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                name_path: "myapp::utils::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                name_path: "myapp::utils::add".to_owned(),
                                args: vec![]
                            },
                        ]
                    },
                    FunctionNode {
                        name: "myapp::utils::add".to_owned(),
                        exported: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![]
                    },
                    FunctionNode {
                        name: "myapp::utils::test".to_owned(),
                        exported: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![
                            // group 0
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                name_path: "myapp::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load32_i32,
                                name_path: "myapp::d0".to_owned(),
                                offset: 0
                            },
                            // group 1
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                name_path: "myapp::utils::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                name_path: "myapp::utils::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                name_path: "myapp::utils::d0".to_owned(),
                                offset: 0
                            },
                            Instruction::DataLoad {
                                opcode: Opcode::data_load64_i64,
                                name_path: "myapp::utils::d0".to_owned(),
                                offset: 0
                            },
                            // group 2
                            Instruction::Call {
                                name_path: "myapp::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                name_path: "myapp::add".to_owned(),
                                args: vec![]
                            },
                            // group 3
                            Instruction::Call {
                                name_path: "myapp::utils::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                name_path: "myapp::utils::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                name_path: "myapp::utils::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::Call {
                                name_path: "myapp::utils::add".to_owned(),
                                args: vec![]
                            },
                        ]
                    }
                ],
                read_only_data_nodes: vec![
                    DataNode {
                        name: "myapp::d0".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4,
                            value: 0x11u32.to_le_bytes().to_vec()
                        })
                    },
                    DataNode {
                        name: "myapp::utils::d0".to_owned(),
                        exported: false,
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
            }
        )
    }

    #[test]
    fn test_preprocess_canonicalize_identifiers_of_external_functions() {
        assert_eq!(
            merge_submodules_from_strs(&[
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
                    FunctionNode {
                        name: "myapp::test".to_owned(),
                        exported: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![
                            // group 0
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            // group 1
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::sub_wrap"
                                    .to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::share::std.so.1::print".to_owned(),
                                args: vec![]
                            },
                            // group 2
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            // group 3
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::mul".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::share::std.so.1::print".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::system::libc.so.6::getuid"
                                    .to_owned(),
                                args: vec![]
                            },
                        ]
                    },
                    FunctionNode {
                        name: "myapp::utils::test".to_owned(),
                        exported: false,
                        params: vec![],
                        results: vec![],
                        locals: vec![],
                        code: vec![
                            // group 0
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            // group 1
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::mul".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::share::std.so.1::print".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::system::libc.so.6::getuid"
                                    .to_owned(),
                                args: vec![]
                            },
                            // group 2
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                name_path: "EXTERNAL_FUNCTION::user::math.so.1::add".to_owned(),
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
            }
        )
    }
}
