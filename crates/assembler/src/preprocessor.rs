// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancasm_parser::{
    ast::{
        BranchCase, DataDetailNode, DataNode, DependItem, DependentModuleNode,
        ExternalFunctionNode, ExternalItem, ExternalNode, FunctionNode, ImportDataNode,
        ImportFunctionNode, ImportItem, ImportNode, Instruction, LocalNode, ModuleElementNode,
        ModuleNode, ParamNode,
    },
    lexer::{filter, lex},
    parser::parse,
    peekable_iterator::PeekableIterator,
    NAME_PATH_SEPARATOR,
};
use ancvm_types::{
    DataType, EffectiveVersion, ModuleShareType, RUNTIME_MAJOR_VERSION, RUNTIME_MINOR_VERSION,
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
// 1. canonicalize the identifiers and names of functions and datas, e.g.
//
// in module 'myapp':
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
// note that the name of a function will be renamed also,
// the name is the same as its identifier except it does not include
// the module name, so the function names are:
//
// - 'entry'
// - 'utils::add'
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
// the relative name paths will be canonicalized.
//
// relative paths begin with the keywords 'module' and 'self', e.g.
//
// - `(call $package::utils::add ...)`
// - `(call $self::utils::add ...)`
//
// 3. canonicalize the identifiers of external functions.
//
// the identifiers and names of external functions
// can be different for simplify the writing of the assembly text, so these identifiers
// need to be canonicalized before assemble.
//
// e.g.
// in module 'myapp':
//
// (depend
//     (library $math "math.so.1" share)
// )
// (external $math
//     (function $add "add" ...)
// )
// (extcall $add ...)
//
// in submodule 'myapp::utils':
//
// (external $math
//     (function $f0 "add" ...)
// )
// (extcall $f0 ...)
//
// the identifiers '$add' and '$f0' will be rewritten as 'math::add'.
//
// in addition to rewriteing identifiers, duplicate 'external' nodes
// will be removed.
//
// the expect identifier for external function is:
// LIBRARY_ID::SYMBOL_NAME
//
// 4. canonicalize the identifiers of 'extcall' instruction
//
// in the example above, the instruction '(extcall $add ...)' and
// '(extcall $f0 ...)' will be rewritten as '(extcall $math::add)'.
//
// 5. canonicalize the identifiers of imported items.
//
// it's similar to the external functions, e.g.
//
// in module 'myapp':
//
// (depend
//     (module $math "math" share)
// )
//
// (import $math
//     (function $add "add" ...)
// )
// (call $add ...)
//
// in submodule 'myapp::utils':
//
// (import $math
//     (function $f0 "add" ...)
// )
// (call $f0 ...)
//
// the identifiers '$add' and '$f0' will be
// rewritten as '$math::add' in both the node 'import' and 'call'
//
// in addition to rewriteing identifiers, duplicate 'import' nodes
// will be removed.
//
// the expect identifier for imported item is:
// PACKAGE_ID::MODULE_NAME_PATH::FUNC_OR_DATA_NAME

// note:
//
// 1. at the assembly level, submodules are transparent to each other,
// i.e., all functions and data (including imported functions, imported data,
// and declared external functions) are public and can be accessed in any submodule.
//
// 2. the identifier of call/extcall/data loading/data_storing can be
// absolute path, relative path or just an identifier.

#[derive(Debug, PartialEq)]
pub struct MergedModuleNode {
    // the main module name
    pub name: String,

    pub runtime_version: EffectiveVersion,
    pub constructor_function_name_path: Option<String>,
    pub destructor_function_name_path: Option<String>,

    pub function_nodes: Vec<CanonicalFunctionNode>,
    pub read_only_data_nodes: Vec<CanonicalDataNode>,
    pub read_write_data_nodes: Vec<CanonicalDataNode>,
    pub uninit_data_nodes: Vec<CanonicalDataNode>,

    pub depend_items: Vec<DependItem>,
    pub import_nodes: Vec<ImportNode>,
    pub external_nodes: Vec<ExternalNode>,
}

#[derive(Debug, PartialEq)]
pub struct CanonicalFunctionNode {
    // the full name path, for function calling instructions
    //
    // e.g.
    // the id of function 'add' in main module 'myapp' is 'myapp::add'
    // the id of function 'add' in submodule 'myapp:utils' is 'myapp::utils::add'
    pub id: String,

    // the canonicalize name path, which includes the submodule path, but
    // excludes the module name.
    //
    // e.g.
    // the name path of function 'add' in main module 'myapp' is 'add'
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
    // the id of data 'buf' in main module 'myapp' is 'myapp::buf'
    // the id of data 'buf' in submodule 'myapp:utils' is 'myapp::utils::buf'
    pub id: String,

    // the canonicalize name path, which includes the submodule path, but
    // excludes the module name.
    //
    // e.g.
    // the name path of data 'buf' in main module 'myapp' is 'buf'
    // the name path of data 'buf' in submodule 'myapp:utils' is 'utils::buf'
    pub name_path: String,

    pub export: bool,
    pub data_kind: DataDetailNode,
}

/**
 * The module object which is used for containing RenameItem
 */
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
    // - "PACKAGE_ID::MODULE_NAME_PATH::FUNC_NAME"
    // - "PACKAGE_ID::MODULE_NAME_PATH::DATA_NAME"
    // - "LIBRARY_ID::SYMBOL_NAME"
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

pub struct InitialFunctionItem {
    pub module_name: String,
    pub module_share_type: ModuleShareType,
    pub module_version: EffectiveVersion,
    pub function_name_path: String, // the name of the constructor or destructor
}

pub struct Initialization {
    pub constructors: Vec<InitialFunctionItem>, // the main module name of a package
    pub destructors: Vec<InitialFunctionItem>,  // the main module name of a package
}

pub const FUNCTION_ENTRY_NAME: &str = "__entry";
pub const FUNCTION_INIT_NAME: &str = "__init";
pub const FUNCTION_FINI_NAME: &str = "__fini";

pub fn merge_and_canonicalize_submodule_nodes(
    submodule_nodes: &[ModuleNode],
    config_depend_items: Option<&[DependItem]>, // config-dependencies come from 'module.anon'
    initialization: Option<&Initialization>, // for generating entry/init/fini functions, for application only
) -> Result<MergedModuleNode, AssembleError> {
    // the first submodule is the main submodule of an application or a library.
    // so pick the name and runtime version from the first submodule.

    let main_module_name = submodule_nodes[0].name_path.clone();
    let runtime_version = submodule_nodes[0].runtime_version.unwrap();
    let constructor_function_name = submodule_nodes[0].constructor_function_name_path.clone();
    let destructor_function_name = submodule_nodes[0].destructor_function_name_path.clone();

    // check submodules name path
    for module_node in &submodule_nodes[1..] {
        let first_part_of_name_path = module_node
            .name_path
            .split(NAME_PATH_SEPARATOR)
            .next()
            .unwrap();

        if first_part_of_name_path != main_module_name {
            return Err(AssembleError {
                message: format!(
                    "The name path of submodule: \"{}\" does not starts with: \"{}\"",
                    module_node.name_path, main_module_name
                ),
            });
        }
    }

    // the dependencies shoud be defined in:
    // - 'package.anon' in multiple source file package
    // - '?.anc' in single file application
    let mut depend_items = if let Some(items) = config_depend_items {
        if submodule_nodes[0].depend_node.is_some() {
            return Err(AssembleError {
                message: "Depend node is not allowed in applicaiton with multiple source file."
                    .to_owned(),
            });
        }

        items.to_owned()
    } else if let Some(node) = &submodule_nodes[0].depend_node {
        node.depend_items.clone()
    } else {
        vec![]
    };

    let mut rename_item_modules: Vec<RenameItemModule> = vec![];

    //
    // canonicalize the import nodes and remove the duplicate items
    //

    let mut canonical_import_nodes: Vec<ImportNode> = vec![];

    for submodule_node in submodule_nodes {
        let submodule_name_path = &submodule_node.name_path;
        let mut rename_items: Vec<RenameItem> = vec![];

        // extra the original imported nodes

        let original_imported_nodes = submodule_node
            .element_nodes
            .iter()
            .filter_map(|node| match node {
                ModuleElementNode::ImportNode(import_node) => Some(import_node),
                _ => None,
            })
            .collect::<Vec<_>>();

        for original_import_node in original_imported_nodes {
            // check the module id

            if !depend_items.iter().any(|item| match item {
                DependItem::DependentModule(module) => module.id == original_import_node.module_id,
                DependItem::DependentLibrary(_) => false,
            }) {
                return Err(AssembleError {
                    message: format!(
                        "Can not find the specified module id: {}.",
                        original_import_node.module_id
                    ),
                });
            }

            // create new canonical import node if it does not exist.

            let idx_opt_of_canonical_import_node = canonical_import_nodes
                .iter()
                .position(|node| node.module_id == original_import_node.module_id);

            let idx_of_canonical_import_node = if let Some(idx) = idx_opt_of_canonical_import_node {
                idx
            } else {
                let idx = canonical_import_nodes.len();

                // create new canonical import node
                let canonical_import_node = ImportNode {
                    module_id: original_import_node.module_id.clone(),
                    import_items: vec![],
                };

                canonical_import_nodes.push(canonical_import_node);
                idx
            };

            let canonical_import_node = &mut canonical_import_nodes[idx_of_canonical_import_node];

            for original_import_item in &original_import_node.import_items {
                // build the canonical import item

                let canonical_import_item = match original_import_item {
                    ImportItem::ImportFunction(original_import_function) => {
                        // the scheme of identifier:
                        // PACKAGE_ID::MODULE_NAME_PATH::FUNCTION_NAME

                        let name_path = original_import_function.name_path.clone();

                        let expect_identifier =
                            format!("{}::{}", original_import_node.module_id, name_path);

                        let canonical_import_function_node = ImportFunctionNode {
                            id: expect_identifier,
                            name_path,
                            params: original_import_function.params.clone(),
                            results: original_import_function.results.clone(),
                        };

                        ImportItem::ImportFunction(canonical_import_function_node)
                    }
                    ImportItem::ImportData(original_import_data) => {
                        // the scheme of identifier:
                        // PACKAGE_ID::MODULE_NAME_PATH::FUNCTION_NAME

                        let name_path = original_import_data.name_path.clone();

                        let expect_identifier =
                            format!("{}::{}", original_import_node.module_id, name_path);

                        let canonical_import_data_node = ImportDataNode {
                            id: expect_identifier,
                            name_path,
                            memory_data_type: original_import_data.memory_data_type,
                            data_section_type: original_import_data.data_section_type, // data_kind_node: original_import_data.data_kind_node.clone(),
                        };

                        ImportItem::ImportData(canonical_import_data_node)
                    }
                };

                // create new canonical import item if it does not exist.

                let idx_opt_of_canonical_import_item = canonical_import_node
                    .import_items
                    .iter()
                    .position(|exists_import_item| exists_import_item == &canonical_import_item);

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

                // add rename item if it does not exist
                //
                // sometimes there maybe multiple identifiers for one imported item, e.g.
                //
                // ```clojure
                // (import $module_id
                //     (function $add "add" ...)
                //     (function $add_i32 "add" ...)
                // )
                // ```
                //
                // so it is necessary to add rename item for each imported item.

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

        // add rename items to target module

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

    //
    // canonicalize the external nodes and remove the duplicate items
    //

    let mut canonical_external_nodes: Vec<ExternalNode> = vec![];

    for submodule_node in submodule_nodes {
        let submodule_name_path = &submodule_node.name_path;
        let mut rename_items: Vec<RenameItem> = vec![];

        // extra the original external nodes

        let original_external_nodes = submodule_node
            .element_nodes
            .iter()
            .filter_map(|node| match node {
                ModuleElementNode::ExternalNode(external_node) => Some(external_node),
                _ => None,
            })
            .collect::<Vec<_>>();

        for original_external_node in original_external_nodes {
            // check the library id

            if !depend_items.iter().any(|item| match item {
                DependItem::DependentModule(_) => false,
                DependItem::DependentLibrary(library) => {
                    library.id == original_external_node.library_id
                }
            }) {
                return Err(AssembleError {
                    message: format!(
                        "Can not find the specified library id: {}.",
                        original_external_node.library_id
                    ),
                });
            }

            // create new canonical external node if it does not exist.

            let idx_opt_of_canonical_external_node = canonical_external_nodes
                .iter()
                .position(|node| node.library_id == original_external_node.library_id);

            let idx_of_canonical_external_node =
                if let Some(idx) = idx_opt_of_canonical_external_node {
                    idx
                } else {
                    let idx = canonical_external_nodes.len();

                    // create new canonical external node
                    let canonical_external_node = ExternalNode {
                        library_id: original_external_node.library_id.clone(),
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
                        // the scheme of identifier:
                        // LIBRARY_ID::SYMBOL_NAME

                        let name = original_external_function.name.clone();

                        let expect_identifier =
                            format!("{}::{}", original_external_node.library_id, name);

                        let canonical_external_function_node = ExternalFunctionNode {
                            id: expect_identifier,
                            name,
                            params: original_external_function.params.clone(),
                            results: original_external_function.results.clone(),
                        };

                        ExternalItem::ExternalFunction(canonical_external_function_node)
                    }
                };

                // create new canonical external item if it does not exist.

                let idx_opt_of_canonical_external_item =
                    canonical_external_node.external_items.iter().position(
                        |exists_external_item| exists_external_item == &canonical_external_item,
                    );

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

                // add rename item if it does not exist
                //
                // sometimes there maybe multiple identifiers for one external item, e.g.
                //
                // ```clojure
                // (external $library_id
                //     (function $add "add" ...)
                //     (function $add_i32 "add" ...)
                // )
                // ```
                //
                // so it is necessary to add rename item for each external item.

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

        // add rename items to target module

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
        // the relative name path of 'myapp::utils::add' is 'utils::add',
        // the relative name path of 'myapp' is `None``.
        let relative_name_path = {
            if let Some((_head, tail)) = module_node.name_path.split_once("::") {
                Some(tail)
            } else {
                None
            }
        };

        // canonicalize the function nodes
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

        // canonicalize the read_only data nodes
        let mut read_only_data_nodes = module_node
            .element_nodes
            .iter()
            .filter_map(|node| match node {
                ModuleElementNode::DataNode(data_node)
                    if matches!(data_node.data_detail, DataDetailNode::ReadOnly(_)) =>
                {
                    Some(data_node)
                }
                _ => None,
            })
            .map(|data_node| {
                canonicalize_data_node(data_node, module_name_path, relative_name_path)
            })
            .collect::<Vec<_>>();

        // canonicalize the read_write data nodes
        let mut read_write_data_nodes = module_node
            .element_nodes
            .iter()
            .filter_map(|node| match node {
                ModuleElementNode::DataNode(data_node)
                    if matches!(data_node.data_detail, DataDetailNode::ReadWrite(_)) =>
                {
                    Some(data_node)
                }
                _ => None,
            })
            .map(|data_node| {
                canonicalize_data_node(data_node, module_name_path, relative_name_path)
            })
            .collect::<Vec<_>>();

        // canonicalize the uninit data nodes
        let mut uninit_data_nodes = module_node
            .element_nodes
            .iter()
            .filter_map(|node| match node {
                ModuleElementNode::DataNode(data_node)
                    if matches!(data_node.data_detail, DataDetailNode::Uninit(_)) =>
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

    // generates:
    // - the '__entry' function (for application only)
    // - the '__init' function
    // - the '__fini' function
    // - the dependent modules and import functions, which are required by constructor and destructor
    if let Some(init) = initialization {
        generate_entry_and_init_and_finit(
            &main_module_name,
            &mut depend_items,
            &mut canonical_import_nodes,
            &mut canonical_function_nodes,
            init,
            &constructor_function_name,
            &destructor_function_name,
        );
    }

    let merged_module_node = MergedModuleNode {
        name: main_module_name,
        runtime_version,
        depend_items,
        constructor_function_name_path: constructor_function_name,
        destructor_function_name_path: destructor_function_name,
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
        data_kind: data_node.data_detail.clone(),
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
        Instruction::UnaryOp {
            opcode,
            source: number,
        } => Instruction::UnaryOp {
            opcode: *opcode,
            source: Box::new(canonicalize_identifiers_of_instruction(
                number,
                module_name_path,
                rename_item_modules,
            )?),
        },
        Instruction::UnaryOpWithImmI16 {
            opcode,
            imm: amount,
            source: number,
        } => Instruction::UnaryOpWithImmI16 {
            opcode: *opcode,
            imm: *amount,
            source: Box::new(canonicalize_identifiers_of_instruction(
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
        Instruction::FnRecur(instructions) => {
            Instruction::FnRecur(canonicalize_identifiers_of_instructions(
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
        Instruction::DynCall {
            public_index: num,
            args,
        } => Instruction::DynCall {
            public_index: Box::new(canonicalize_identifiers_of_instruction(
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
        Instruction::Debug { .. } => instruction.clone(),
        Instruction::Unreachable { .. } => instruction.clone(),
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
            "package" => current_module_name_path
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

fn generate_entry_and_init_and_finit(
    main_module_name: &str,
    depend_items: &mut Vec<DependItem>,
    import_nodes: &mut Vec<ImportNode>,
    canonical_function_nodes: &mut Vec<CanonicalFunctionNode>,
    initialization: &Initialization,
    constructor_function_name_path: &Option<String>,
    destructor_function_name_path: &Option<String>,
) {
    // generates:
    //
    // - the '__entry' function (for application only)
    // - the '__init' function
    // - the '__fini' function

    // the content of function __entry:
    //
    // ```js
    // function __entry() {
    //      __init()                // call __init
    //      let exit_code = main()  // call main and retain the exit code
    //      __fini()                // call __finit
    //      return exit_code        // return the exit code
    // }
    // ```

    let mut add_function_item = |function_item: &InitialFunctionItem, ids: &mut Vec<String>| {
        // create new depend item if it does not exist

        let module_name = &function_item.module_name;

        let module_id_opt = depend_items.iter().find_map(|item| match item {
            DependItem::DependentModule(m) if &m.name == module_name => Some(&m.id),
            _ => None,
        });

        let module_id = if let Some(id) = module_id_opt {
            id
        } else {
            // create new depend item
            let item = DependItem::DependentModule(DependentModuleNode {
                id: module_name.clone(),
                module_share_type: function_item.module_share_type,
                name: module_name.clone(),
                module_version: function_item.module_version,
            });

            depend_items.push(item);
            module_name
        };

        // create new import node if it does not exist.

        let import_node_idx_opt = import_nodes
            .iter()
            .position(|node| &node.module_id == module_id);

        let import_node_idx = if let Some(idx) = import_node_idx_opt {
            idx
        } else {
            let idx = import_nodes.len();

            // create new import node
            let import_node = ImportNode {
                module_id: module_id.clone(),
                import_items: vec![],
            };

            import_nodes.push(import_node);
            idx
        };

        let import_node = &mut import_nodes[import_node_idx];

        // add import function

        // PACKAGE_ID::MODULE_NAME_PATH::FUNCTION_NAME
        let function_id = format!("{}::{}", module_id, function_item.function_name_path);

        import_node
            .import_items
            .push(ImportItem::ImportFunction(ImportFunctionNode {
                id: function_id.clone(),
                name_path: function_item.function_name_path.clone(),
                params: vec![],
                results: vec![],
            }));

        ids.push(function_id);
    };

    let mut constructor_ids: Vec<String> = vec![];
    let mut destructor_ids: Vec<String> = vec![];

    // generating the constructor and destructor function calling ids,
    // as well as generating the depend items and import items
    // for initialization in initializations {
        for constructor_function_item in &initialization.constructors {
            add_function_item(constructor_function_item, &mut constructor_ids);
        }

        for destructor_function_item in &initialization.destructors {
            add_function_item(destructor_function_item, &mut destructor_ids);
        }
    // }

    let mut calling_constructor_strings = constructor_ids
        .iter()
        .map(|id| format!("(call ${})", id))
        .collect::<Vec<String>>();

    let mut calling_destructor_strings = destructor_ids
        .iter()
        .map(|id| format!("(call ${})", id))
        .collect::<Vec<String>>();

    if let Some(name_path) = constructor_function_name_path {
        let id = format!("{}::{}", main_module_name, name_path);
        calling_constructor_strings.push(format!("(call ${})", id));
    }

    if let Some(name_path) = destructor_function_name_path {
        let id = format!("{}::{}", main_module_name, name_path);
        calling_destructor_strings.push(format!("(call ${})", id));
    }

    let function_entry_code = format!(
        r#"
        (function ${FUNCTION_ENTRY_NAME} (result i64)
            (local $exit_code i64)
            (code
                (call ${MAIN_MODULE_NAME}::{FUNCTION_INIT_NAME})
                (local.store64 $exit_code (call ${MAIN_MODULE_NAME}::main))
                (call ${MAIN_MODULE_NAME}::{FUNCTION_FINI_NAME})
                (local.load64_i64 $exit_code)
            )
        )
        "#,
        FUNCTION_ENTRY_NAME = FUNCTION_ENTRY_NAME,
        FUNCTION_INIT_NAME = FUNCTION_INIT_NAME,
        MAIN_MODULE_NAME = main_module_name,
        FUNCTION_FINI_NAME = FUNCTION_FINI_NAME
    );

    // let function_init_code = format!(
    //     r#"
    //     (function export $__init
    //         (code
    //             (envcall {ENV_CALL_CODE_COUNT_START_FUNCTION})
    //             (for (param $remain i32)
    //                 (do
    //                     (when
    //                         (i32.eqz (local.load32_i32 $remain))
    //                         (break)
    //                     )
    //                     (dyncall
    //                         (envcall
    //                             {ENV_CALL_CODE_GET_START_FUNCTION_ITEM}
    //                             (i32.dec (local.load32_i32 $remain) 1)
    //                         )
    //                     )
    //                     (recur
    //                         (i32.dec (local.load32_i32 $remain) 1)
    //                     )
    //                 )
    //             )
    //         )
    //     )
    //     "#,
    //     ENV_CALL_CODE_COUNT_START_FUNCTION = (EnvCallCode::count_start_function as u32),
    //     ENV_CALL_CODE_GET_START_FUNCTION_ITEM = (EnvCallCode::get_start_function_item as u32),
    // );

    // the 'initialization' function for the package

    let function_init_code = format!(
        r#"
        (function ${FUNCTION_INIT_NAME}
            (code
                {CALL_CONSTRUCTOR_STRINGS}
            )
        )
        "#,
        FUNCTION_INIT_NAME = FUNCTION_INIT_NAME,
        CALL_CONSTRUCTOR_STRINGS = (calling_constructor_strings.join(" ")),
    );

    // let function_fini_code = format!(
    //     r#"
    //     (function export $__fini
    //         (code
    //             (envcall {ENV_CALL_CODE_COUNT_EXIT_FUNCTION})
    //             (for (param $remain i32)
    //                 (do
    //                     (when
    //                         (i32.eqz (local.load32_i32 $remain))
    //                         (break)
    //                     )
    //                     (dyncall
    //                         (envcall
    //                             {ENV_CALL_CODE_GET_EXIT_FUNCTION_ITEM}
    //                             (i32.dec (local.load32_i32 $remain) 1)
    //                         )
    //                     )
    //                     (recur
    //                         (i32.dec (local.load32_i32 $remain) 1)
    //                     )
    //                 )
    //             )
    //         )
    //     )
    //     "#,
    //     ENV_CALL_CODE_COUNT_EXIT_FUNCTION = (EnvCallCode::count_exit_function as u32),
    //     ENV_CALL_CODE_GET_EXIT_FUNCTION_ITEM = (EnvCallCode::get_exit_function_item as u32),
    // );

    // the 'finish' function for the package

    let function_fini_code = format!(
        r#"
        (function ${FUNCTION_FINI_NAME}
            (code
                {CALL_DESTRUCTOR_STRINGS}
            )
        )
        "#,
        FUNCTION_FINI_NAME = FUNCTION_FINI_NAME,
        CALL_DESTRUCTOR_STRINGS = (calling_destructor_strings.join(" ")),
    );

    let module_code_start = r#"
    (module $__auto_generated
        (runtime_version "1.0")
        "#
    .to_string();

    let module_code_end = r#"
    )
        "#
    .to_string();

    let auto_generated_codes = vec![
        module_code_start,
        function_init_code,
        function_fini_code,
        function_entry_code,
        module_code_end,
    ];

    let auto_generated_code_string = auto_generated_codes.join("");

    // lex and parse the code snippet
    let mut chars = auto_generated_code_string.chars();
    let mut char_iter = PeekableIterator::new(&mut chars, 3);
    let all_tokens = lex(&mut char_iter).unwrap();
    let effective_tokens = filter(&all_tokens);
    let mut token_iter = effective_tokens.into_iter();
    let mut peekable_token_iter = PeekableIterator::new(&mut token_iter, 2);
    let auto_generated_module_node = parse(
        &mut peekable_token_iter,
        Some(EffectiveVersion::new(
            RUNTIME_MAJOR_VERSION,
            RUNTIME_MINOR_VERSION,
        )),
    )
    .unwrap();

    // insert the auto-generated node to main module node
    auto_generated_module_node
        .element_nodes
        .iter()
        .for_each(|node| {
            if let ModuleElementNode::FunctionNode(function_node) = node {
                let id = format!("{}::{}", main_module_name, function_node.name);

                let canonical_function_node = CanonicalFunctionNode {
                    id,
                    name_path: function_node.name.clone(),
                    export: function_node.export,
                    params: function_node.params.clone(),
                    results: function_node.results.clone(),
                    locals: function_node.locals.clone(),
                    code: function_node.code.clone(),
                };

                canonical_function_nodes.push(canonical_function_node);
            }
        });
}

#[cfg(test)]
mod tests {
    use ancvm_types::{
        opcode::Opcode, DataSectionType, EffectiveVersion, ExternalLibraryType, MemoryDataType,
        ModuleShareType,
    };
    use pretty_assertions::assert_eq;

    use ancasm_parser::{
        ast::{
            DataDetailNode, DependItem, DependentLibraryNode, DependentModuleNode,
            ExternalFunctionNode, ExternalItem, ExternalNode, ImportDataNode, ImportFunctionNode,
            ImportItem, ImportNode, InitedData, Instruction, UninitData,
        },
        lexer::{filter, lex},
        parser::parse,
        peekable_iterator::PeekableIterator,
    };

    use crate::{
        preprocessor::{CanonicalDataNode, CanonicalFunctionNode, InitialFunctionItem},
        AssembleError,
    };

    use super::{merge_and_canonicalize_submodule_nodes, Initialization, MergedModuleNode};

    fn preprocess_and_get_merged_module_node(submodule_sources: &[&str]) -> MergedModuleNode {
        preprocess_from_strs(submodule_sources, None).unwrap()
    }

    fn preprocess_from_strs(
        submodule_sources: &[&str],
        initialization: Option<&Initialization>,
    ) -> Result<MergedModuleNode, AssembleError> {
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

        merge_and_canonicalize_submodule_nodes(&submodule_nodes, None, initialization)
    }

    #[test]
    fn test_preprocess_merge_functions() {
        assert_eq!(
            preprocess_and_get_merged_module_node(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (function $entry (code))
                (function $main (code))
            )
            "#,
                r#"
            (module $myapp::utils
                (function $add (code))
                (function $sub (code))
            )
            "#
            ]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version: EffectiveVersion::new(1, 0),
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                depend_items: vec![],
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
            preprocess_and_get_merged_module_node(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (data $code (read_only i32 0x11))
            )
            "#,
                r#"
            (module $myapp::utils
                (data $count (read_write i32 0x13))
                (data $sum (uninit i32))
            )
            "#
            ]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version: EffectiveVersion::new(1, 0),
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                depend_items: vec![],
                function_nodes: vec![],
                read_only_data_nodes: vec![CanonicalDataNode {
                    id: "myapp::code".to_owned(),
                    name_path: "code".to_owned(),
                    export: false,
                    data_kind: DataDetailNode::ReadOnly(InitedData {
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
                    data_kind: DataDetailNode::ReadWrite(InitedData {
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
                    data_kind: DataDetailNode::Uninit(UninitData {
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
    fn test_preprocess_canonicalize_identifiers_of_function_related_instructions() {
        assert_eq!(
            preprocess_and_get_merged_module_node(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (function $add (code))
                (function $test (code
                    // group 0
                    (call $add)
                    (call $myapp::add)
                    (call $package::add)
                    (call $self::add)
                    // group 1
                    (call $myapp::utils::add)
                    (call $package::utils::add)
                    (call $self::utils::add)
                ))
            )
            "#,
                r#"
            (module $myapp::utils
                (function $add (code))
                (function $test (code
                    // group 2
                    (call $myapp::add)
                    (call $package::add)
                    // group 3
                    (call $add)
                    (call $self::add)
                    (call $myapp::utils::add)
                    (call $package::utils::add)
                ))
            )
            "#
            ]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version: EffectiveVersion::new(1, 0),
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                depend_items: vec![],
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
                            // group 0
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
                            // group 1
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
    fn test_preprocess_canonicalize_identifiers_of_data_related_instructions() {
        assert_eq!(
            preprocess_and_get_merged_module_node(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (data $d0 (read_only i32 0x11))
                (function $test (code
                    // group 0
                    (data.load32_i32 $d0)
                    (data.load32_i32 $myapp::d0)
                    (data.load32_i32 $package::d0)
                    (data.load32_i32 $self::d0)
                    // group 1
                    (data.load64_i64 $myapp::utils::d0)
                    (data.load64_i64 $package::utils::d0)
                    (data.load64_i64 $self::utils::d0)
                ))
            )
            "#,
                r#"
            (module $myapp::utils
                (data $d0 (read_only i64 0x13))
                (function $test (code
                    // group 2
                    (data.load32_i32 $myapp::d0)
                    (data.load32_i32 $package::d0)
                    // group 3
                    (data.load64_i64 $d0)
                    (data.load64_i64 $self::d0)
                    (data.load64_i64 $myapp::utils::d0)
                    (data.load64_i64 $package::utils::d0)
                ))
            )
            "#
            ]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version: EffectiveVersion::new(1, 0),
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                depend_items: vec![],
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
                            // group 2
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
                            // group 3
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
                        data_kind: DataDetailNode::ReadOnly(InitedData {
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
                        data_kind: DataDetailNode::ReadOnly(InitedData {
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
    fn test_preprocess_depend_item_id() {
        assert_eq!(
            preprocess_and_get_merged_module_node(&[r#"
            (module $myapp
                (runtime_version "1.0")
                (depend
                    (module $math share "math" "1.0")
                    (library $libc share "libc.so.6")
                )
                (import $math)
                (external $libc)
            )
            "#,]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version: EffectiveVersion::new(1, 0),
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                depend_items: vec![
                    DependItem::DependentModule(DependentModuleNode {
                        id: "math".to_owned(),
                        module_share_type: ModuleShareType::Share,
                        name: "math".to_owned(),
                        module_version: EffectiveVersion::new(1, 0)
                    }),
                    DependItem::DependentLibrary(DependentLibraryNode {
                        id: "libc".to_owned(),
                        external_library_type: ExternalLibraryType::Share,
                        name: "libc.so.6".to_owned()
                    })
                ],
                function_nodes: vec![],
                read_only_data_nodes: vec![],
                read_write_data_nodes: vec![],
                uninit_data_nodes: vec![],
                import_nodes: vec![ImportNode {
                    module_id: "math".to_owned(),
                    import_items: vec![]
                }],
                external_nodes: vec![ExternalNode {
                    library_id: "libc".to_owned(),
                    external_items: vec![]
                }],
            }
        );

        // module id not found
        assert!(matches!(
            preprocess_from_strs(
                &[r#"
            (module $myapp
                (runtime_version "1.0")
                (depend
                    (module $math share "math" "1.0")
                    (library $libc share "libc.so.6")
                )
                (import $mymath)
            )
            "#],
                None
            ),
            Err(AssembleError { message: _ })
        ));

        // library id not found
        assert!(matches!(
            preprocess_from_strs(
                &[r#"
            (module $myapp
                (runtime_version "1.0")
                (depend
                    (module $math share "math" "1.0")
                    (library $libc share "libc.so.6")
                )
                (external $mylibc)
            )
            "#],
                None
            ),
            Err(AssembleError { message: _ })
        ));
    }

    #[test]
    fn test_preprocess_canonicalize_identifiers_of_import_functions() {
        assert_eq!(
            preprocess_and_get_merged_module_node(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (depend
                    (module $math share "math" "1.0")
                    (module $format user "format" "1.2")
                    (module $random share "random" "2.3")
                )
                (import $math
                    (function $add "add")
                    (function $sub "wrap::sub")
                )
                (import $format
                    (function $print "print")
                )
                (function $test (code
                    // group 0
                    (call $add)                 // math::add
                    (call $myapp::add)          // math::add
                    (call $package::add)         // math::add
                    (call $self::add)           // math::add

                    // group 1
                    (call $sub)                 // math::wrap::sub
                    (call $print)               // format::print

                    // group 2
                    (call $myapp::utils::f0)    // math::add
                    (call $package::utils::f0)   // math::add
                    (call $self::utils::f0)     // math::add

                    // group 3
                    (call $myapp::utils::f1)    // math::mul
                    (call $myapp::utils::f2)    // format::print
                    (call $myapp::utils::f3)    // random::rand
                ))
            )
            "#,
                r#"
            (module $myapp::utils
                (import $math
                    (function $f0 "add")        // duplicate
                    (function $f1 "mul")        // new
                )
                (import $format
                    (function $f2 "print")      // duplicate
                )
                (import $random
                    (function $f3 "rand")       // new
                )
                (function $test (code
                    // group 4
                    (call $f0)                  // math::add
                    (call $myapp::utils::f0)    // math::add
                    (call $package::utils::f0)   // math::add
                    (call $self::f0)            // math::add

                    // group 5
                    (call $f1)                  // math::mul
                    (call $f2)                  // format::print
                    (call $f3)                  // random::rand

                    // group 6
                    (call $myapp::add)          // math::add
                    (call $package::add)         // math::add
                ))
            )
            "#
            ]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version: EffectiveVersion::new(1, 0),
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                depend_items: vec![
                    DependItem::DependentModule(DependentModuleNode {
                        id: "math".to_owned(),
                        module_share_type: ModuleShareType::Share,
                        name: "math".to_owned(),
                        module_version: EffectiveVersion::new(1, 0)
                    }),
                    DependItem::DependentModule(DependentModuleNode {
                        id: "format".to_owned(),
                        module_share_type: ModuleShareType::User,
                        name: "format".to_owned(),
                        module_version: EffectiveVersion::new(1, 2)
                    }),
                    DependItem::DependentModule(DependentModuleNode {
                        id: "random".to_owned(),
                        module_share_type: ModuleShareType::Share,
                        name: "random".to_owned(),
                        module_version: EffectiveVersion::new(2, 3)
                    }),
                ],
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
                            // group 4
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
                            // group 5
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
                            // group 6
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
                        module_id: "math".to_owned(),
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
                        module_id: "format".to_owned(),
                        import_items: vec![ImportItem::ImportFunction(ImportFunctionNode {
                            id: "format::print".to_owned(),
                            name_path: "print".to_owned(),
                            params: vec![],
                            results: vec![]
                        }),]
                    },
                    ImportNode {
                        module_id: "random".to_owned(),
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
            preprocess_and_get_merged_module_node(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (depend
                    (module $math share "math" "1.0")
                    (module $match user "match" "1.2")
                    (module $random share "random" "2.3")
                )
                (import $math
                    (data $count "count" read_only i32)
                    (data $sum "wrap::sum" read_write i64)
                )
                (import $match
                    (data $score "score" read_write f32)
                )
                (function $test (code
                    // group 0
                    (data.load32_i32 $count)                // math::count
                    (data.load32_i32 $myapp::count)         // math::count
                    (data.load32_i32 $package::count)       // math::count
                    (data.load32_i32 $self::count)          // math::count

                    // group 1
                    (data.load64_i64 $sum)                  // math::wrap::sum
                    (data.load32_f32 $score)                // match::score

                    // group 2
                    (data.load32_i32 $myapp::utils::d0)     // math::count
                    (data.load32_i32 $package::utils::d0)   // math::count
                    (data.load32_i32 $self::utils::d0)      // math::count

                    // group 3
                    (data.load32_i32 $myapp::utils::d1)     // math::increment
                    (data.load32_f32 $myapp::utils::d2)     // match::score
                    (data.load64_i64 $myapp::utils::d3)     // random::seed
                ))
            )
            "#,
                r#"
            (module $myapp::utils
                (import $math
                    (data $d0 "count" read_only i32)        // duplicate
                    (data $d1 "increment" uninit i32)       // new
                )
                (import $match
                    (data $d2 "score" read_write f32)       // duplicate
                )
                (import $random
                    (data $d3 "seed" read_write i64)        // new
                )
                (function $test (code
                    // group 4
                    (data.load32_i32 $d0)                   // math::count
                    (data.load32_i32 $myapp::utils::d0)     // math::count
                    (data.load32_i32 $package::utils::d0)   // math::count
                    (data.load32_i32 $self::d0)             // math::count

                    // group 5
                    (data.load32_i32 $d1)                   // math::increment
                    (data.load32_f32 $d2)                   // match::score
                    (data.load64_i64 $d3)                   // random::seed

                    // group 6
                    (data.load32_i32 $myapp::count)         // math::count
                    (data.load32_i32 $package::count)       // math::count
                ))
            )
            "#
            ]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version: EffectiveVersion::new(1, 0),
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                depend_items: vec![
                    DependItem::DependentModule(DependentModuleNode {
                        id: "math".to_owned(),
                        module_share_type: ModuleShareType::Share,
                        name: "math".to_owned(),
                        module_version: EffectiveVersion::new(1, 0),
                    }),
                    DependItem::DependentModule(DependentModuleNode {
                        id: "match".to_owned(),
                        module_share_type: ModuleShareType::User,
                        name: "match".to_owned(),
                        module_version: EffectiveVersion::new(1, 2),
                    }),
                    DependItem::DependentModule(DependentModuleNode {
                        id: "random".to_owned(),
                        module_share_type: ModuleShareType::Share,
                        name: "random".to_owned(),
                        module_version: EffectiveVersion::new(2, 3),
                    }),
                ],
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
                            // group 4
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
                            // group 5
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
                            // group 6
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
                        module_id: "math".to_owned(),
                        import_items: vec![
                            ImportItem::ImportData(ImportDataNode {
                                id: "math::count".to_owned(),
                                name_path: "count".to_owned(),
                                memory_data_type: MemoryDataType::I32,
                                data_section_type: DataSectionType::ReadOnly
                            }),
                            ImportItem::ImportData(ImportDataNode {
                                id: "math::wrap::sum".to_owned(),
                                name_path: "wrap::sum".to_owned(),
                                memory_data_type: MemoryDataType::I64,
                                data_section_type: DataSectionType::ReadWrite
                            }),
                            ImportItem::ImportData(ImportDataNode {
                                id: "math::increment".to_owned(),
                                name_path: "increment".to_owned(),
                                memory_data_type: MemoryDataType::I32,
                                data_section_type: DataSectionType::Uninit
                            }),
                        ]
                    },
                    ImportNode {
                        module_id: "match".to_owned(),
                        import_items: vec![ImportItem::ImportData(ImportDataNode {
                            id: "match::score".to_owned(),
                            name_path: "score".to_owned(),
                            memory_data_type: MemoryDataType::F32,
                            data_section_type: DataSectionType::ReadWrite
                        }),]
                    },
                    ImportNode {
                        module_id: "random".to_owned(),
                        import_items: vec![ImportItem::ImportData(ImportDataNode {
                            id: "random::seed".to_owned(),
                            name_path: "seed".to_owned(),
                            memory_data_type: MemoryDataType::I64,
                            data_section_type: DataSectionType::ReadWrite
                        }),]
                    }
                ]
            }
        )
    }

    #[test]
    fn test_preprocess_canonicalize_identifiers_of_external_functions() {
        assert_eq!(
            preprocess_and_get_merged_module_node(&[
                r#"
            (module $myapp
                (runtime_version "1.0")
                (depend
                    (library $math user "math.so.1")
                    (library $std share "std.so.1")
                    (library $libc system "libc.so.6")
                )
                (external $math
                    (function $add "add")
                    (function $sub "sub_wrap")
                )
                (external $std
                    (function $print "print")
                )
                (function $test (code
                    // group 0
                    (extcall $add)
                    (extcall $myapp::add)
                    (extcall $package::add)
                    (extcall $self::add)

                    // group 1
                    (extcall $sub)
                    (extcall $print)

                    // group 2
                    (extcall $myapp::utils::f0)     // add
                    (extcall $package::utils::f0)   // add
                    (extcall $self::utils::f0)      // add

                    // group 3
                    (extcall $myapp::utils::f1)     // mul
                    (extcall $myapp::utils::f2)     // print
                    (extcall $myapp::utils::f3)     // getuid
                ))
            )
            "#,
                r#"
            (module $myapp::utils
                (external $math
                    (function $f0 "add")            // duplicate
                    (function $f1 "mul")            // new
                )
                (external $std
                    (function $f2 "print")          // duplicate
                )
                (external $libc
                    (function $f3 "getuid")         // new
                )
                (function $test (code
                    // group 0
                    (extcall $f0)
                    (extcall $myapp::utils::f0)
                    (extcall $package::utils::f0)
                    (extcall $self::f0)

                    // group 1
                    (extcall $f1)
                    (extcall $f2)
                    (extcall $f3)

                    // group 2
                    (extcall $myapp::add)
                    (extcall $package::add)
                ))
            )
            "#
            ]),
            MergedModuleNode {
                name: "myapp".to_owned(),
                runtime_version: EffectiveVersion::new(1, 0),
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                depend_items: vec![
                    DependItem::DependentLibrary(DependentLibraryNode {
                        id: "math".to_owned(),
                        external_library_type: ExternalLibraryType::User,
                        name: "math.so.1".to_owned()
                    }),
                    DependItem::DependentLibrary(DependentLibraryNode {
                        id: "std".to_owned(),
                        external_library_type: ExternalLibraryType::Share,
                        name: "std.so.1".to_owned()
                    }),
                    DependItem::DependentLibrary(DependentLibraryNode {
                        id: "libc".to_owned(),
                        external_library_type: ExternalLibraryType::System,
                        name: "libc.so.6".to_owned()
                    }),
                ],
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
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            // group 1
                            Instruction::ExtCall {
                                id: "math::sub_wrap".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "std::print".to_owned(),
                                args: vec![]
                            },
                            // group 2
                            Instruction::ExtCall {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            // group 3
                            Instruction::ExtCall {
                                id: "math::mul".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "std::print".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "libc::getuid".to_owned(),
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
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            // group 1
                            Instruction::ExtCall {
                                id: "math::mul".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "std::print".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "libc::getuid".to_owned(),
                                args: vec![]
                            },
                            // group 2
                            Instruction::ExtCall {
                                id: "math::add".to_owned(),
                                args: vec![]
                            },
                            Instruction::ExtCall {
                                id: "math::add".to_owned(),
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
                        library_id: "math".to_owned(),
                        external_items: vec![
                            ExternalItem::ExternalFunction(ExternalFunctionNode {
                                id: "math::add".to_owned(),
                                name: "add".to_owned(),
                                params: vec![],
                                results: vec![]
                            }),
                            ExternalItem::ExternalFunction(ExternalFunctionNode {
                                id: "math::sub_wrap".to_owned(),
                                name: "sub_wrap".to_owned(),
                                params: vec![],
                                results: vec![]
                            }),
                            ExternalItem::ExternalFunction(ExternalFunctionNode {
                                id: "math::mul".to_owned(),
                                name: "mul".to_owned(),
                                params: vec![],
                                results: vec![]
                            }),
                        ]
                    },
                    ExternalNode {
                        library_id: "std".to_owned(),
                        external_items: vec![ExternalItem::ExternalFunction(
                            ExternalFunctionNode {
                                id: "std::print".to_owned(),
                                name: "print".to_owned(),
                                params: vec![],
                                results: vec![]
                            }
                        ),]
                    },
                    ExternalNode {
                        library_id: "libc".to_owned(),
                        external_items: vec![ExternalItem::ExternalFunction(
                            ExternalFunctionNode {
                                id: "libc::getuid".to_owned(),
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
    fn test_preprocess_auto_generate_functions() {
        let merged_module_node = preprocess_from_strs(
            &[r#"
            (module $myapp
                (runtime_version "1.0")
                (constructor $main_start)
                (destructor $main_end)
                (function $main (result i64)
                    (code
                        (i64.imm 11)
                    )
                )
                (function $main_start
                    (code nop)
                )
                (function $main_end
                    (code nop)
                )
            )"#],
            Some(&Initialization {
                constructors: vec![
                    InitialFunctionItem {
                        module_name: "storage".to_string(),
                        module_share_type: ModuleShareType::User,
                        module_version: EffectiveVersion { major: 1, minor: 0 },
                        function_name_path: "create_storage".to_string(),
                    },
                    InitialFunctionItem {
                        module_name: "backend".to_string(),
                        module_share_type: ModuleShareType::Share,
                        module_version: EffectiveVersion { major: 1, minor: 0 },
                        function_name_path: "connect".to_string(),
                    },
                ],
                destructors: vec![
                    InitialFunctionItem {
                        module_name: "storage".to_string(),
                        module_share_type: ModuleShareType::User,
                        module_version: EffectiveVersion { major: 1, minor: 0 },
                        function_name_path: "remove_storage".to_string(),
                    },
                    InitialFunctionItem {
                        module_name: "frontend".to_string(),
                        module_share_type: ModuleShareType::Share,
                        module_version: EffectiveVersion { major: 1, minor: 0 },
                        function_name_path: "disconnect".to_string(),
                    },
                ],
            }),
        )
        .unwrap();

        let function_nodes = &merged_module_node.function_nodes;

        // println!("{:#?}", function_nodes);

        assert_eq!(function_nodes.len(), 6);

        let function_node0 = &function_nodes[0];
        assert_eq!(function_node0.id, "myapp::main");
        assert_eq!(function_node0.name_path, "main");
        assert_eq!(function_node0.export, false);

        let function_node1 = &function_nodes[1];
        assert_eq!(function_node1.id, "myapp::main_start");
        assert_eq!(function_node1.name_path, "main_start");
        assert_eq!(function_node1.export, false);

        let function_node2 = &function_nodes[2];
        assert_eq!(function_node2.id, "myapp::main_end");
        assert_eq!(function_node2.name_path, "main_end");
        assert_eq!(function_node2.export, false);

        let function_node3 = &function_nodes[3];
        assert_eq!(function_node3.id, "myapp::__init");
        assert_eq!(function_node3.name_path, "__init");
        assert_eq!(function_node3.export, false);
        assert_eq!(
            function_node3.code,
            vec![
                Instruction::Call {
                    id: "storage::create_storage".to_string(),
                    args: vec![]
                },
                Instruction::Call {
                    id: "backend::connect".to_string(),
                    args: vec![]
                },
                Instruction::Call {
                    id: "myapp::main_start".to_string(),
                    args: vec![]
                }
            ]
        );

        let function_node4 = &function_nodes[4];
        assert_eq!(function_node4.id, "myapp::__fini");
        assert_eq!(function_node4.name_path, "__fini");
        assert_eq!(function_node4.export, false);
        assert_eq!(
            function_node4.code,
            vec![
                Instruction::Call {
                    id: "storage::remove_storage".to_string(),
                    args: vec![]
                },
                Instruction::Call {
                    id: "frontend::disconnect".to_string(),
                    args: vec![]
                },
                Instruction::Call {
                    id: "myapp::main_end".to_string(),
                    args: vec![]
                }
            ]
        );

        let function_node5 = &function_nodes[5];
        assert_eq!(function_node5.id, "myapp::__entry");
        assert_eq!(function_node5.name_path, "__entry");
        assert_eq!(function_node5.export, false);
    }
}
