// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_parser::ast::{
    BranchCase, DataKindNode, DataNode, ExternNode, ExternalFuncNode, ExternalItem, FuncNode,
    Instruction, ModuleElementNode, ModuleNode,
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
// (fn $entry ...)
//
// in submodule 'myapp::utils':
// (fn $add ...)
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
// the identifier `$add`` in `(call $add ...)` and macro `(macro.get_func_pub_index $add ...)`
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
// 3. canonicalize the identifiers of external functions and imported items.
//
// the identifiers and names of external functions (as well as imported items)
// can be different for simplify the writing of the assembly text, so these identifiers
// need to be canonicalized before assemble.
//
// e.g.
// in submodule 'myapp':
//
// (extern (library shared "math.so.1")
//         (fn $add "add" ...)
// )
// (extcall $add ...)
//
// in submodule 'myapp::utils':
//
// (extern (library shared "math.so.1")
//         (fn $f0 "add" ...)
// )
// (extcall $f0 ...)
//
// the identifiers '$add' and '$f0' will be
// rewritten as 'shared::math.so.1::add' in both the node 'extern' and 'extcall'
//
// the expect identifier name for external function is:
// EXTERNAL_FUNC::EXTERNAL_LIBRARY_TYPE::LIBRARY_SO_NAME::SYMBOL
//
// it's similar to the names of imported items, the expect identifier name for
// imported items is:
// IMPORTED_DATA|IMPORTED_FUNC::PATH_NAMES::ITEM_NAME

#[derive(Debug)]
pub struct MergedModuleNode {
    // the main module name
    pub name: String,

    pub runtime_version_major: u16,
    pub runtime_version_minor: u16,

    pub func_nodes: Vec<FuncNode>,
    pub read_only_data_nodes: Vec<DataNode>,
    pub read_write_data_nodes: Vec<DataNode>,
    pub uninit_data_nodes: Vec<DataNode>,
    pub extern_nodes: Vec<ExternNode>,
}

struct RenameItemModule {
    // module_name_path: String,
    items: Vec<RenameItem>,
}

struct RenameItem {
    from: String,
    to: String,
    kind: RenameKind,
}

#[derive(Debug, PartialEq)]
enum RenameKind {
    // the internal and impored functions
    // FUNC::NAME_PATH::FUNC_NAME
    Func,

    // the internal and imported data
    // DATA::NAME_PATH::DATA_NAME
    Data,

    // the external functions
    // EXTERNAL_FUNC::EXTERNAL_LIBRARY_TYPE::LIBRARY_SO_NAME::SYMBOL
    ExternalFunc,
}

pub fn merge_submodule_nodes(
    submodule_nodes: &[ModuleNode],
) -> Result<MergedModuleNode, AssembleError> {
    // the first submodule is the main submodule of an application or a library.
    // so pick the name and runtime version from the first submodule.
    let name = submodule_nodes[0].name_path.clone();
    let runtime_version_major = submodule_nodes[0].runtime_version_major;
    let runtime_version_minor = submodule_nodes[0].runtime_version_minor;

    // check submodules name path and runtime version
    for module_node in &submodule_nodes[1..] {
        let first_name = module_node.name_path.split("::").next().unwrap();
        if first_name != name {
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
    let mut canonical_extern_nodes: Vec<ExternNode> = vec![];

    // canonicalize the extern nodes
    // remove the duplicate external items and group by external library

    for submodule_node in submodule_nodes {
        let mut rename_items: Vec<RenameItem> = vec![];

        let original_extern_nodes = submodule_node
            .element_nodes
            .iter()
            .filter_map(|node| match node {
                ModuleElementNode::ExternNode(extern_node) => Some(extern_node),
                _ => None,
            })
            .collect::<Vec<_>>();

        // find the existed external node
        // create new canonical external node if it does not exist.

        for original_extern_node in original_extern_nodes {
            let extern_node_idx_opt = canonical_extern_nodes.iter().position(|node| {
                node.external_library_node == original_extern_node.external_library_node
            });

            let extern_node_idx = if let Some(idx) = extern_node_idx_opt {
                idx
            } else {
                let idx = canonical_extern_nodes.len();

                // create new canonical external node
                let canonical_extern_node = ExternNode {
                    external_library_node: original_extern_node.external_library_node.clone(),
                    external_items: vec![],
                };

                canonical_extern_nodes.push(canonical_extern_node);
                idx
            };

            let canonical_extern_node = &mut canonical_extern_nodes[extern_node_idx];

            // create new canonical external item if it does not exist.

            for original_extern_item in &original_extern_node.external_items {
                let extern_item_idx_opt = canonical_extern_node
                    .external_items
                    .iter()
                    .position(|exists_external_item| exists_external_item == original_extern_item);

                if let Some(idx) = extern_item_idx_opt {
                    // already exist.
                    let expect_identifier_name = match &canonical_extern_node.external_items[idx] {
                        ExternalItem::ExternalFunc(external_func) => external_func.name.to_owned(),
                    };

                    let actual_identifier_name = match original_extern_item {
                        ExternalItem::ExternalFunc(external_func) => external_func.name.to_owned(),
                    };

                    // add rename item
                    if expect_identifier_name != actual_identifier_name {
                        let rename_item = RenameItem {
                            from: actual_identifier_name,
                            to: expect_identifier_name,
                            kind: RenameKind::ExternalFunc,
                        };
                        rename_items.push(rename_item);
                    }
                } else {
                    // create new canonical external item
                    match original_extern_item {
                        ExternalItem::ExternalFunc(original_external_func) => {
                            // the format of expect identifier name:
                            // EXTERNAL_FUNC::EXTERNAL_LIBRARY_TYPE::LIBRARY_SO_NAME::SYMBOL

                            let symbol = original_external_func.symbol.clone();

                            let expect_identifier_name = format!(
                                "EXTERNAL_FUNC::{}::{}::{}",
                                original_extern_node
                                    .external_library_node
                                    .external_library_type,
                                original_extern_node.external_library_node.name,
                                symbol
                            );

                            let actual_identifier_name = original_external_func.name.clone();

                            // add rename item
                            if expect_identifier_name != actual_identifier_name {
                                let rename_item = RenameItem {
                                    from: actual_identifier_name,
                                    to: expect_identifier_name.clone(),
                                    kind: RenameKind::ExternalFunc,
                                };
                                rename_items.push(rename_item);
                            }

                            let canonical_external_func_node = ExternalFuncNode {
                                name: expect_identifier_name,
                                symbol,
                                params: original_external_func.params.clone(),
                                results: original_external_func.results.clone(),
                            };

                            canonical_extern_node
                                .external_items
                                .push(ExternalItem::ExternalFunc(canonical_external_func_node));
                        }
                    }
                }
            }
        }

        let rename_item_module = RenameItemModule {
            items: rename_items,
        };

        rename_item_modules.push(rename_item_module);
    }

    // todo::
    // canonicalize the import nodes

    let mut canonical_func_nodes: Vec<FuncNode> = vec![];
    let mut canonical_read_only_data_nodes: Vec<DataNode> = vec![];
    let mut canonical_read_write_data_nodes: Vec<DataNode> = vec![];
    let mut canonical_uninit_data_nodes: Vec<DataNode> = vec![];

    for module_idx in 0..submodule_nodes.len() {
        let module_node = &submodule_nodes[module_idx];
        let module_name_path = &module_node.name_path;
        let rename_items = &rename_item_modules[module_idx].items;

        // canonicalize the func nodes
        let mut func_nodes = module_node
            .element_nodes
            .iter()
            .filter_map(|node| match node {
                ModuleElementNode::FuncNode(func_node) => Some(func_node),
                _ => None,
            })
            .map(|func_node| canonicalize_func_node(func_node, module_name_path, rename_items))
            .collect::<Vec<_>>();

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

        canonical_func_nodes.append(&mut func_nodes);
        canonical_read_only_data_nodes.append(&mut read_only_data_nodes);
        canonical_read_write_data_nodes.append(&mut read_write_data_nodes);
        canonical_uninit_data_nodes.append(&mut uninit_data_nodes);
    }

    let merged_module_node = MergedModuleNode {
        name,
        runtime_version_major,
        runtime_version_minor,
        func_nodes: canonical_func_nodes,
        read_only_data_nodes: canonical_read_only_data_nodes,
        read_write_data_nodes: canonical_read_write_data_nodes,
        uninit_data_nodes: canonical_uninit_data_nodes,
        extern_nodes: canonical_extern_nodes,
    };

    Ok(merged_module_node)
}

fn canonicalize_func_node(
    func_node: &FuncNode,
    module_name_path: &str,
    rename_items: &[RenameItem],
) -> FuncNode {
    let func_full_name = format!("{}::{}", module_name_path, func_node.name);
    let canonical_code =
        canonicalize_identifiers_of_instructions(&func_node.code, module_name_path, rename_items);

    FuncNode {
        name: func_full_name,
        exported: func_node.exported,
        params: func_node.params.clone(),
        results: func_node.results.clone(),
        locals: func_node.locals.clone(),
        code: canonical_code,
    }
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
    rename_items: &[RenameItem],
) -> Vec<Instruction> {
    instructions
        .iter()
        .map(|instruction| {
            canonicalize_identifiers_of_instruction(instruction, module_name_path, rename_items)
        })
        .collect::<Vec<_>>()
}

fn canonicalize_identifiers_of_instruction(
    instruction: &Instruction,
    module_name_path: &str,
    rename_items: &[RenameItem],
) -> Instruction {
    match instruction {
        Instruction::NoParams { opcode, operands } => Instruction::NoParams {
            opcode: *opcode,
            operands: canonicalize_identifiers_of_instructions(
                operands,
                module_name_path,
                rename_items,
            ),
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
                rename_items,
            )),
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
                rename_items,
            )),
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
                rename_items,
            )),
            value: Box::new(canonicalize_identifiers_of_instruction(
                value,
                module_name_path,
                rename_items,
            )),
        },
        Instruction::DataLoad {
            opcode,
            name_path,
            offset,
        } => Instruction::DataLoad {
            opcode: *opcode,
            name_path: canonicalize_func_and_data_name_path(
                RenameKind::Data,
                name_path,
                module_name_path,
                rename_items,
            ),
            offset: *offset,
        },
        Instruction::DataStore {
            opcode,
            name_path,
            offset,
            value,
        } => Instruction::DataStore {
            opcode: *opcode,
            name_path: canonicalize_func_and_data_name_path(
                RenameKind::Data,
                name_path,
                module_name_path,
                rename_items,
            ),
            offset: *offset,
            value: Box::new(canonicalize_identifiers_of_instruction(
                value,
                module_name_path,
                rename_items,
            )),
        },
        Instruction::DataLongLoad {
            opcode,
            name_path,
            offset,
        } => Instruction::DataLongLoad {
            opcode: *opcode,
            name_path: canonicalize_func_and_data_name_path(
                RenameKind::Data,
                name_path,
                module_name_path,
                rename_items,
            ),
            offset: Box::new(canonicalize_identifiers_of_instruction(
                offset,
                module_name_path,
                rename_items,
            )),
        },
        Instruction::DataLongStore {
            opcode,
            name_path,
            offset,
            value,
        } => Instruction::DataLongStore {
            opcode: *opcode,
            name_path: canonicalize_func_and_data_name_path(
                RenameKind::Data,
                name_path,
                module_name_path,
                rename_items,
            ),
            offset: Box::new(canonicalize_identifiers_of_instruction(
                offset,
                module_name_path,
                rename_items,
            )),
            value: Box::new(canonicalize_identifiers_of_instruction(
                value,
                module_name_path,
                rename_items,
            )),
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
                rename_items,
            )),
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
                rename_items,
            )),
            value: Box::new(canonicalize_identifiers_of_instruction(
                value,
                module_name_path,
                rename_items,
            )),
        },
        Instruction::UnaryOp { opcode, number } => Instruction::UnaryOp {
            opcode: *opcode,
            number: Box::new(canonicalize_identifiers_of_instruction(
                number,
                module_name_path,
                rename_items,
            )),
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
                rename_items,
            )),
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
                rename_items,
            )),
            right: Box::new(canonicalize_identifiers_of_instruction(
                right,
                module_name_path,
                rename_items,
            )),
        },
        Instruction::When { test, consequent } => Instruction::When {
            test: Box::new(canonicalize_identifiers_of_instruction(
                test,
                module_name_path,
                rename_items,
            )),
            consequent: Box::new(canonicalize_identifiers_of_instruction(
                consequent,
                module_name_path,
                rename_items,
            )),
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
                rename_items,
            )),
            consequent: Box::new(canonicalize_identifiers_of_instruction(
                consequent,
                module_name_path,
                rename_items,
            )),
            alternate: Box::new(canonicalize_identifiers_of_instruction(
                alternate,
                module_name_path,
                rename_items,
            )),
        },
        Instruction::Branch {
            results,
            cases,
            default,
        } => Instruction::Branch {
            results: results.clone(),
            cases: cases
                .iter()
                .map(|case| BranchCase {
                    test: Box::new(canonicalize_identifiers_of_instruction(
                        &case.test,
                        module_name_path,
                        rename_items,
                    )),
                    consequent: Box::new(canonicalize_identifiers_of_instruction(
                        &case.consequent,
                        module_name_path,
                        rename_items,
                    )),
                })
                .collect::<Vec<_>>(),
            default: default.as_ref().map(|instruction| {
                Box::new(canonicalize_identifiers_of_instruction(
                    instruction,
                    module_name_path,
                    rename_items,
                ))
            }),
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
                rename_items,
            )),
        },
        Instruction::Do(instructions) => Instruction::Do(canonicalize_identifiers_of_instructions(
            instructions,
            module_name_path,
            rename_items,
        )),
        Instruction::Break(instructions) => Instruction::Break(
            canonicalize_identifiers_of_instructions(instructions, module_name_path, rename_items),
        ),
        Instruction::Recur(instructions) => Instruction::Recur(
            canonicalize_identifiers_of_instructions(instructions, module_name_path, rename_items),
        ),
        Instruction::Return(instructions) => Instruction::Return(
            canonicalize_identifiers_of_instructions(instructions, module_name_path, rename_items),
        ),
        Instruction::Rerun(instructions) => Instruction::Rerun(
            canonicalize_identifiers_of_instructions(instructions, module_name_path, rename_items),
        ),
        Instruction::Call { name_path, args } => Instruction::Call {
            name_path: canonicalize_func_and_data_name_path(
                RenameKind::Func,
                name_path,
                module_name_path,
                rename_items,
            ),
            args: canonicalize_identifiers_of_instructions(args, module_name_path, rename_items),
        },
        Instruction::DynCall { num, args } => Instruction::DynCall {
            num: Box::new(canonicalize_identifiers_of_instruction(
                num,
                module_name_path,
                rename_items,
            )),
            args: canonicalize_identifiers_of_instructions(args, module_name_path, rename_items),
        },
        Instruction::EnvCall { num, args } => Instruction::EnvCall {
            num: *num,
            args: canonicalize_identifiers_of_instructions(args, module_name_path, rename_items),
        },
        Instruction::SysCall { num, args } => Instruction::SysCall {
            num: *num,
            args: canonicalize_identifiers_of_instructions(args, module_name_path, rename_items),
        },
        Instruction::ExtCall { name, args } => Instruction::ExtCall {
            name: canonicalize_external_func_call_name(name, rename_items),
            args: canonicalize_identifiers_of_instructions(args, module_name_path, rename_items),
        },
        Instruction::Debug(_) => instruction.clone(),
        Instruction::Unreachable(_) => instruction.clone(),
        Instruction::HostAddrFunc(name_path) => {
            Instruction::HostAddrFunc(canonicalize_func_and_data_name_path(
                RenameKind::Func,
                name_path,
                module_name_path,
                rename_items,
            ))
        }
        Instruction::MacroGetFuncPubIndex(name_path) => {
            Instruction::MacroGetFuncPubIndex(canonicalize_func_and_data_name_path(
                RenameKind::Func,
                name_path,
                module_name_path,
                rename_items,
            ))
        }
    }
}

fn rename(rename_kind: RenameKind, name: &str, rename_items: &[RenameItem]) -> String {
    let idx_opt = rename_items
        .iter()
        .position(|item| item.kind == rename_kind && item.from == name);

    match idx_opt {
        Some(idx) => rename_items[idx].to.clone(),
        None => name.to_owned(),
    }
}

fn canonicalize_func_and_data_name_path(
    rename_kind: RenameKind,
    name_path: &str,
    module_name_path: &str,
    rename_items: &[RenameItem],
) -> String {
    let name_parts = name_path.split("::").collect::<Vec<&str>>();

    if name_parts.len() == 1 {
        let actual_name = rename(rename_kind, name_path, rename_items);
        format!("{}::{}", module_name_path, actual_name)
    } else {
        let mut canonical_parts = vec![];

        let first_part = name_parts[0];
        canonical_parts.push(match first_part {
            "module" => module_name_path.split("::").next().unwrap().to_owned(),
            "self" => module_name_path.to_owned(),
            _ => first_part.to_owned(),
        });

        let last_part = name_parts[name_parts.len() - 1];
        let actual_name = rename(rename_kind, last_part, rename_items);

        if name_parts.len() > 2 {
            name_parts[1..(name_parts.len() - 2)]
                .iter()
                .for_each(|s| canonical_parts.push(s.to_string()));
        }

        canonical_parts.push(actual_name);
        canonical_parts.join("::")
    }
}

fn canonicalize_external_func_call_name(name: &str, rename_items: &[RenameItem]) -> String {
    rename(RenameKind::ExternalFunc, name, rename_items)
}

#[cfg(test)]
mod tests {

    #[test]
    fn test() {
        // todo
    }
}
