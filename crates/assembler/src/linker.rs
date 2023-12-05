// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_types::{
    entry::{
        DataIndexEntry, DataIndexModuleEntry, ExternalFunctionIndexEntry,
        ExternalFunctionIndexModuleEntry, FunctionIndexEntry, FunctionIndexModuleEntry, IndexEntry,
        ModuleEntry, UnifiedExternalFunctionEntry, UnifiedExternalLibraryEntry,
    },
    DataSectionType,
};

use crate::AssembleError;

// build:
// - function public index table
// - data public index table
// - external function index table
pub fn link(module_entries: &[&ModuleEntry]) -> Result<IndexEntry, AssembleError> {
    let function_index_module_entries = link_functions(module_entries)?;
    let data_index_module_entries = link_data(module_entries)?;
    let (
        unified_external_library_entries,
        unified_external_function_entries,
        external_function_index_module_entries,
    ) = link_external_functions(module_entries)?;

    // todo::
    // find all constructor functions and the 'entry' function in the first module.

    // todo::
    // find all destructor functions

    Ok(IndexEntry {
        function_index_module_entries,
        data_index_module_entries,
        unified_external_library_entries,
        unified_external_function_entries,
        external_function_index_module_entries,
    })
}

fn link_functions(
    module_entries: &[&ModuleEntry],
) -> Result<Vec<FunctionIndexModuleEntry>, AssembleError> {
    let mut function_index_module_entries: Vec<FunctionIndexModuleEntry> = vec![];

    for (current_module_index, current_module_entry) in module_entries.iter().enumerate() {
        let import_module_entries = &current_module_entry.import_module_entries;

        // build function entries
        let mut function_index_entries: Vec<FunctionIndexEntry> = vec![];

        // add import function entries

        for (function_public_index, import_function_entry) in current_module_entry
            .import_function_entries
            .iter()
            .enumerate()
        {
            // convert the import module index into global module index.
            let target_module_name =
                &import_module_entries[import_function_entry.import_module_index].name;
            let target_module_index = find_module_index(module_entries, target_module_name);
            let target_module_entry = module_entries[target_module_index];
            let target_function_internal_index =
                find_function_index(target_module_entry, &import_function_entry.name_path)?;

            let target_function_type_index =
                target_module_entry.function_entries[target_function_internal_index].type_index;

            // check the function type
            let expect_type = &current_module_entry.type_entries[import_function_entry.type_index];
            let actual_type = &target_module_entry.type_entries[target_function_type_index];

            if expect_type != actual_type {
                return Err(AssembleError {
                    message: format!(
                        "The signature of imported function \"{}\" in module \"{}\" does not match.",
                        import_function_entry.name_path, target_module_entry.name)
                });
            }

            let function_index_entry = FunctionIndexEntry {
                function_public_index,
                target_module_index,
                function_internal_index: target_function_internal_index,
            };

            function_index_entries.push(function_index_entry);
        }

        // add internal function entries

        for function_internal_index in 0..current_module_entry.function_entries.len() {
            let function_index_entry = FunctionIndexEntry {
                function_public_index: function_internal_index
                    + current_module_entry.import_function_count,
                target_module_index: current_module_index,
                function_internal_index,
            };

            function_index_entries.push(function_index_entry);
        }

        // add function entries to module

        let function_index_module_entry = FunctionIndexModuleEntry {
            index_entries: function_index_entries,
        };

        function_index_module_entries.push(function_index_module_entry);
    }

    Ok(function_index_module_entries)
}

fn link_data(module_entries: &[&ModuleEntry]) -> Result<Vec<DataIndexModuleEntry>, AssembleError> {
    // note:
    //
    // 1. the data public index includes (and are sorted by the following order):
    //
    // - imported read-only data items
    // - internal read-only data items
    // - imported read-write data items
    // - internal read-write data items
    // - imported uninitilized data items
    // - internal uninitilized data items
    //
    // 2. the 'data internal index' is section relevant.
    //
    // e.g.
    // there are indices 0,1,2,3... in read-only section, and
    // there are also indices 0,1,2,3... in read-write section, and
    // there are also indices 0,1,2,3... in uninitialized section.

    let mut data_index_module_entries: Vec<DataIndexModuleEntry> = vec![];

    for current_module_index in 0..module_entries.len() {
        // data public index in a module
        let mut data_public_index: usize = 0;

        // build data entries
        let mut data_index_entries: Vec<DataIndexEntry> = vec![];

        // add read-only data entries
        link_specified_section_data_of_a_module(
            module_entries,
            current_module_index,
            DataSectionType::ReadOnly,
            &mut data_public_index,
            &mut data_index_entries,
        )?;

        // add read-write data entries
        link_specified_section_data_of_a_module(
            module_entries,
            current_module_index,
            DataSectionType::ReadWrite,
            &mut data_public_index,
            &mut data_index_entries,
        )?;

        // add uninit data entries
        link_specified_section_data_of_a_module(
            module_entries,
            current_module_index,
            DataSectionType::Uninit,
            &mut data_public_index,
            &mut data_index_entries,
        )?;

        // add function entries to module

        let data_index_module_entry = DataIndexModuleEntry {
            index_entries: data_index_entries,
        };

        data_index_module_entries.push(data_index_module_entry);
    }

    Ok(data_index_module_entries)
}

fn link_specified_section_data_of_a_module(
    module_entries: &[&ModuleEntry],
    current_module_index: usize,
    current_data_section_type: DataSectionType,
    // outputs
    data_public_index: &mut usize,
    data_index_entries: &mut Vec<DataIndexEntry>,
) -> Result<(), AssembleError> {
    let current_module_entry = &module_entries[current_module_index];
    let current_import_module_entries = &current_module_entry.import_module_entries;
    let current_section_data_entries_count = match current_data_section_type {
        DataSectionType::ReadOnly => current_module_entry.read_only_data_entries.len(),
        DataSectionType::ReadWrite => current_module_entry.read_write_data_entries.len(),
        DataSectionType::Uninit => current_module_entry.uninit_data_entries.len(),
    };

    let import_data_entries = current_module_entry
        .import_data_entries
        .iter()
        .filter(|entry| entry.data_section_type == current_data_section_type)
        .collect::<Vec<_>>();

    for import_data_entry in import_data_entries {
        // convert the import module index into global module index.
        let target_module_name =
            &current_import_module_entries[import_data_entry.import_module_index].name;
        let target_module_index = find_module_index(module_entries, target_module_name);
        let target_module_entry = module_entries[target_module_index];
        let target_data_internal_index = find_exported_data_index(
            target_module_entry,
            current_data_section_type,
            &import_data_entry.name_path,
        )?;

        // check the memory data type
        let expect_memory_data_type = &import_data_entry.memory_data_type;
        let actual_memory_data_type = &target_module_entry.read_only_data_entries
            [target_data_internal_index]
            .memory_data_type;

        if expect_memory_data_type != actual_memory_data_type {
            return Err(AssembleError {
                message: format!(
                    "The data type of imported data \"{}\" in module \"{}\" does not match.",
                    import_data_entry.name_path, target_module_entry.name
                ),
            });
        }

        let data_index_entry = DataIndexEntry {
            data_public_index: *data_public_index,
            target_module_index,
            data_internal_index: target_data_internal_index,
            target_data_section_type: current_data_section_type,
        };

        data_index_entries.push(data_index_entry);

        // increase public index
        *data_public_index += 1;
    }

    // add internal data entries

    for data_internal_index in 0..current_section_data_entries_count {
        let function_index_entry = DataIndexEntry {
            data_public_index: *data_public_index,
            target_module_index: current_module_index,
            data_internal_index,
            target_data_section_type: current_data_section_type,
        };

        data_index_entries.push(function_index_entry);

        // increase public index
        *data_public_index += 1;
    }

    Ok(())
}

type LinkResultForExternalFunctions = (
    Vec<UnifiedExternalLibraryEntry>,
    Vec<UnifiedExternalFunctionEntry>,
    Vec<ExternalFunctionIndexModuleEntry>,
);

// merge all external libraries and functions and
// remove duplicate items.
fn link_external_functions(
    module_entries: &[&ModuleEntry],
) -> Result<LinkResultForExternalFunctions, AssembleError> {
    let mut unified_external_library_entries: Vec<UnifiedExternalLibraryEntry> = vec![];
    let mut unified_external_function_entries: Vec<UnifiedExternalFunctionEntry> = vec![];
    let mut external_function_index_module_entries: Vec<ExternalFunctionIndexModuleEntry> = vec![];

    for module_entry in module_entries {
        let mut external_function_index_entries: Vec<ExternalFunctionIndexEntry> = vec![];

        for (original_external_library_index, original_external_library_entry) in
            module_entry.external_library_entries.iter().enumerate()
        {
            let unified_library_index_opt =
                unified_external_library_entries.iter().position(|entry| {
                    entry.external_library_type
                        == original_external_library_entry.external_library_type
                        && entry.name == original_external_library_entry.name
                });

            // create new unified library entry when it does not exist
            let unified_library_index = if let Some(idx) = unified_library_index_opt {
                idx
            } else {
                let idx = unified_external_library_entries.len();
                let unified_external_library_entry = UnifiedExternalLibraryEntry {
                    name: original_external_library_entry.name.clone(),
                    external_library_type: original_external_library_entry.external_library_type,
                };
                unified_external_library_entries.push(unified_external_library_entry);
                idx
            };

            // filter external functions by the specified external libray
            let external_function_entries_with_indices = module_entry
                .external_function_entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    entry.external_library_index == original_external_library_index
                })
                .collect::<Vec<_>>();

            for (original_external_function_index, original_external_function_entry) in
                external_function_entries_with_indices
            {
                let unified_function_index_opt =
                    unified_external_function_entries.iter().position(|entry| {
                        entry.unified_external_library_index == unified_library_index
                            && entry.name == original_external_function_entry.name
                    });

                // create new unified function entry when it does not exist
                let unified_function_index = if let Some(idx) = unified_function_index_opt {
                    idx
                } else {
                    // add unified function entry
                    let idx = unified_external_function_entries.len();
                    let unified_external_function_entry = UnifiedExternalFunctionEntry {
                        name: original_external_function_entry.name.clone(),
                        unified_external_library_index: unified_library_index,
                    };
                    unified_external_function_entries.push(unified_external_function_entry);
                    idx
                };

                // add external function index entry
                let external_function_index_entry = ExternalFunctionIndexEntry {
                    external_function_index: original_external_function_index,
                    unified_external_function_index: unified_function_index,
                    type_index: original_external_function_entry.type_index,
                };
                external_function_index_entries.push(external_function_index_entry);
            }
        }

        let external_function_index_module_entry = ExternalFunctionIndexModuleEntry {
            index_entries: external_function_index_entries,
        };
        external_function_index_module_entries.push(external_function_index_module_entry);
    }

    Ok((
        unified_external_library_entries,
        unified_external_function_entries,
        external_function_index_module_entries,
    ))
}

fn find_module_index(module_entries: &[&ModuleEntry], target_module_name: &str) -> usize {
    module_entries
        .iter()
        .position(|entry| entry.name == target_module_name)
        .unwrap()
}

fn find_function_index(
    target_module_entry: &ModuleEntry,
    target_function_name_path: &str,
) -> Result<usize, AssembleError> {
    let item = target_module_entry
        .function_name_entries
        .iter()
        .find(|entry| entry.name_path == target_function_name_path);

    match item {
        Some(entry) => {
            if !entry.export {
                return Err(AssembleError {
                    message: format!(
                        "The function \"{}\" in module \"{}\" is private.",
                        target_function_name_path, target_module_entry.name
                    ),
                });
            }

            let target_function_public_index = entry.function_public_index;
            let target_function_internal_index =
                target_function_public_index - target_module_entry.import_function_count;
            Ok(target_function_internal_index)
        }
        None => Err(AssembleError {
            message: format!(
                "Can not find the exported function \"{}\" in module \"{}\".",
                target_function_name_path, target_module_entry.name
            ),
        }),
    }
}

fn find_exported_data_index(
    module_entry: &ModuleEntry,
    target_data_section_type: DataSectionType,
    target_data_name_path: &str,
) -> Result<usize, AssembleError> {
    let item = module_entry
        .data_name_entries
        .iter()
        .find(|entry| entry.name_path == target_data_name_path);

    match item {
        Some(entry) => {
            if !entry.export {
                return Err(AssembleError {
                    message: format!(
                        "The exported data \"{}\" in module \"{}\" is private.",
                        target_data_name_path, module_entry.name
                    ),
                });
            }

            let target_data_public_index = entry.data_public_index;
            let target_data_internal_index = target_data_public_index
                - match target_data_section_type {
                    DataSectionType::ReadOnly => module_entry.import_read_only_data_count,
                    DataSectionType::ReadWrite => module_entry.import_read_write_data_count,
                    DataSectionType::Uninit => module_entry.import_uninit_data_count,
                };

            Ok(target_data_internal_index)
        }
        None => Err(AssembleError {
            message: format!(
                "Can not find the exported data \"{}\" in module \"{}\".",
                target_data_name_path, module_entry.name
            ),
        }),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use ancvm_parser::{lexer::lex, parser::parse, peekable_iterator::PeekableIterator};
    use ancvm_types::entry::{FunctionIndexEntry, FunctionIndexModuleEntry, ModuleEntry};

    use crate::{
        assembler::assemble_merged_module_node, linker::link,
        preprocessor::merge_and_canonicalize_submodule_nodes,
    };

    fn assemble_from_str(source: &str) -> ModuleEntry {
        let mut chars = source.chars();
        let mut char_iter = PeekableIterator::new(&mut chars, 2);
        let mut tokens = lex(&mut char_iter).unwrap().into_iter();
        let mut token_iter = PeekableIterator::new(&mut tokens, 2);

        let module_node = parse(&mut token_iter).unwrap();
        let merged_module_node = merge_and_canonicalize_submodule_nodes(&[module_node]).unwrap();

        assemble_merged_module_node(&merged_module_node).unwrap()
    }

    fn assemble_from_strs(sources: Vec<&str>) -> Vec<ModuleEntry> {
        sources
            .iter()
            .map(|source| assemble_from_str(source))
            .collect::<Vec<_>>()
    }

    #[test]
    fn test_link_functions() {
        let module_entries = assemble_from_strs(vec![
            r#"
            (module $myapp
                (runtime_version "1.0")
                (import (module share "std" "1.0")
                    (function $print "print")
                )
                (import (module share "format" "1.0")
                    (function $print_fmt "print_fmt")
                )
                (function $entry
                    (code
                        (call $print)
                        (call $print_fmt)
                    )
                )
            )
            "#,
            r#"
            (module $format
                (runtime_version "1.0")
                (import (module share "std" "1.0")
                    (function $print "print")
                    (function $fprint "fprint")
                )
                (function export $print_fmt
                    (code
                        (call $print)
                    )
                )
                (function export $fprint_fmt
                    (code
                        (call $fprint)
                    )
                )
            )
            "#,
            r#"
            (module $std
                (runtime_version "1.0")
                (function export $print
                    (code
                        (call $fprint)
                    )
                )
                (function export $fprint
                    (code)
                )
            )
            "#,
        ]);

        // println!("{:#?}", module_entries);

        let ref_module_entries = module_entries.iter().collect::<Vec<_>>();
        let index_entry = link(&ref_module_entries).unwrap();
        // println!("{:#?}", index_entry);

        let function_index_module_entries = &index_entry.function_index_module_entries;
        assert_eq!(function_index_module_entries.len(), 3);

        assert_eq!(
            function_index_module_entries[0],
            FunctionIndexModuleEntry {
                index_entries: vec![
                    // std::print / 2::0
                    FunctionIndexEntry {
                        function_public_index: 0,
                        target_module_index: 2,
                        function_internal_index: 0
                    },
                    // format::print_fmt / 1::0
                    FunctionIndexEntry {
                        function_public_index: 1,
                        target_module_index: 1,
                        function_internal_index: 0
                    },
                    // myapp::entry / 0::0
                    FunctionIndexEntry {
                        function_public_index: 2,
                        target_module_index: 0,
                        function_internal_index: 0
                    },
                ]
            }
        );

        assert_eq!(
            function_index_module_entries[1],
            FunctionIndexModuleEntry {
                index_entries: vec![
                    // std::print / 2::0
                    FunctionIndexEntry {
                        function_public_index: 0,
                        target_module_index: 2,
                        function_internal_index: 0
                    },
                    // std::fprint / 2::1
                    FunctionIndexEntry {
                        function_public_index: 1,
                        target_module_index: 2,
                        function_internal_index: 1
                    },
                    // format::print_fmt / 1::0
                    FunctionIndexEntry {
                        function_public_index: 2,
                        target_module_index: 1,
                        function_internal_index: 0
                    },
                    // myapp::fprint_fmt / 1::1
                    FunctionIndexEntry {
                        function_public_index: 3,
                        target_module_index: 1,
                        function_internal_index: 1
                    },
                ]
            }
        );

        assert_eq!(
            function_index_module_entries[2],
            FunctionIndexModuleEntry {
                index_entries: vec![
                    // std::print / 2::0
                    FunctionIndexEntry {
                        function_public_index: 0,
                        target_module_index: 2,
                        function_internal_index: 0
                    },
                    // std::fprint / 2::1
                    FunctionIndexEntry {
                        function_public_index: 1,
                        target_module_index: 2,
                        function_internal_index: 1
                    },
                ]
            }
        );

        let data_index_module_entries = &index_entry.data_index_module_entries;
        assert_eq!(data_index_module_entries.len(), 3);
        assert!(data_index_module_entries[0].index_entries.is_empty());
        assert!(data_index_module_entries[1].index_entries.is_empty());
        assert!(data_index_module_entries[2].index_entries.is_empty());

        let external_function_index_module_entries =
            &index_entry.external_function_index_module_entries;
        assert_eq!(external_function_index_module_entries.len(), 3);
        assert!(external_function_index_module_entries[0]
            .index_entries
            .is_empty());
        assert!(external_function_index_module_entries[1]
            .index_entries
            .is_empty());
        assert!(external_function_index_module_entries[2]
            .index_entries
            .is_empty());

        assert!(index_entry.unified_external_library_entries.is_empty());
        assert!(index_entry.unified_external_function_entries.is_empty());
    }
}
