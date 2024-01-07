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
//
// the module entries must be ordered by 'top most -> deep most', and
// the first entry should be the application (includes test) module.
//
// consider there is an application with the following dependencies tree:
//
//           modules
//
//            [a] app
//    /----<--/|\-->----\
//    |        |        |
//   [b]      [e]<--\  [c]
//    |        |    |   |
//    v       [f]   |  [d]
//    \-------\|    \---/
//            [g]
//
// in the search path of the tree above,
// the depth of node 'e' can be either 1 or 3, we should select the largest one (i.e., 3)
// as its final depth. it is similar to the node 'g', its deapth can be 2, 3 and 5,
// so the 5 should be selectd.
//
// so the module order shoudl be:
//
// depth:   0       1       2      3      4      5
// order:  (a) -> (b,c) -> (d) -> (e) -> (f) -> (g)

pub fn link(module_entries: &[&ModuleEntry]) -> Result<IndexEntry, AssembleError> {
    let function_index_module_entries = link_functions(module_entries)?;
    let data_index_module_entries = link_data(module_entries)?;
    let (
        unified_external_library_entries,
        unified_external_function_entries,
        external_function_index_module_entries,
    ) = link_external_functions(module_entries)?;

    let main_module_entry = module_entries[0];
    let start_function_public_indices = get_constructors_public_indices(main_module_entry);
    let exit_function_public_indices = get_destructors_public_indices(main_module_entry);
    let entry_function_public_index = get_entry_function_public_index(main_module_entry);

    Ok(IndexEntry {
        function_index_module_entries,
        data_index_module_entries,
        unified_external_library_entries,
        unified_external_function_entries,
        external_function_index_module_entries,
        start_function_public_indices,
        exit_function_public_indices,
        entry_function_public_index,
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
            let target_function_internal_index = find_function_internal_index(
                target_module_entry,
                &import_function_entry.name_path,
            )?;

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

    let current_section_import_data_entries = current_module_entry
        .import_data_entries
        .iter()
        .filter(|entry| entry.data_section_type == current_data_section_type)
        .collect::<Vec<_>>();

    for import_data_entry in current_section_import_data_entries {
        // convert the import module index into global module index.
        let target_module_name =
            &current_import_module_entries[import_data_entry.import_module_index].name;
        let target_module_index = find_module_index(module_entries, target_module_name);
        let target_module_entry = module_entries[target_module_index];
        let target_data_internal_index = find_exported_data_internal_index(
            target_module_entry,
            current_data_section_type,
            &import_data_entry.name_path,
        )?;

        // check the memory data type
        let expect_memory_data_type = &import_data_entry.memory_data_type;
        let actual_memory_data_type = match current_data_section_type {
            DataSectionType::ReadOnly => {
                &target_module_entry.read_only_data_entries[target_data_internal_index]
                    .memory_data_type
            }
            DataSectionType::ReadWrite => {
                &target_module_entry.read_write_data_entries[target_data_internal_index]
                    .memory_data_type
            }
            DataSectionType::Uninit => {
                &target_module_entry.uninit_data_entries[target_data_internal_index]
                    .memory_data_type
            }
        };

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

    let current_section_data_entries_count = match current_data_section_type {
        DataSectionType::ReadOnly => current_module_entry.read_only_data_entries.len(),
        DataSectionType::ReadWrite => current_module_entry.read_write_data_entries.len(),
        DataSectionType::Uninit => current_module_entry.uninit_data_entries.len(),
    };

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

fn find_function_internal_index(
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

fn find_exported_data_internal_index(
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

            // the public index of data is mixed up all sections, e.g.
            // the public index of uninitialized data 'foo' =
            //      'the amount of imported read-only data' +
            //      'the amount of read-only data' +
            //      'the amount of imported read-write data' +
            //      'the amount of read-write data' +
            //      'the amount of uninit data'
            //      'the internal index of uninit data'

            let target_data_internal_index = target_data_public_index
                - match target_data_section_type {
                    DataSectionType::ReadOnly => module_entry.import_read_only_data_count,
                    DataSectionType::ReadWrite => {
                        module_entry.import_read_only_data_count
                            + module_entry.read_only_data_entries.len()
                            + module_entry.import_read_write_data_count
                    }
                    DataSectionType::Uninit => {
                        module_entry.import_read_only_data_count
                            + module_entry.read_only_data_entries.len()
                            + module_entry.import_read_write_data_count
                            + module_entry.read_write_data_entries.len()
                            + module_entry.import_uninit_data_count
                    }
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

fn get_constructors_public_indices(main_module_entry: &ModuleEntry) -> Vec<u32> {
    // search functions which 'id' start with '__constructor_',
    // then add the main module constructor.
    // todo
    vec![]
}

fn get_destructors_public_indices(main_module_entry: &ModuleEntry) -> Vec<u32> {
    // search functions which 'id' start with '__destructor_',
    // then add the main module destructor.
    // todo
    vec![]
}

fn get_entry_function_public_index(main_module_entry: &ModuleEntry) -> u32 {
    0
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use ancasm_parser::{
        lexer::{filter, lex},
        parser::parse,
        peekable_iterator::PeekableIterator,
    };
    use ancvm_types::{
        entry::{
            DataIndexEntry, ExternalFunctionIndexEntry, FunctionIndexEntry,
            FunctionIndexModuleEntry, ModuleEntry, UnifiedExternalFunctionEntry,
            UnifiedExternalLibraryEntry,
        },
        DataSectionType, ExternalLibraryType,
    };

    use crate::{
        assembler::assemble_merged_module_node, linker::link,
        preprocessor::merge_and_canonicalize_submodule_nodes,
    };

    fn assemble_from_str(source: &str) -> ModuleEntry {
        let mut chars = source.chars();
        let mut char_iter = PeekableIterator::new(&mut chars, 3);
        let all_tokens = lex(&mut char_iter).unwrap();
        let effective_tokens = filter(&all_tokens);
        let mut token_iter = effective_tokens.into_iter();
        let mut peekable_token_iter = PeekableIterator::new(&mut token_iter, 2);

        let module_node = parse(&mut peekable_token_iter, None).unwrap();
        let merged_module_node =
            merge_and_canonicalize_submodule_nodes(&[module_node], None, None).unwrap();

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
                (compiler_version "1.0")
                (depend
                    (module $std share "std" "1.0")
                    (module $format share "format" "1.0")
                )
                (import $std
                    (function $print "print")
                )
                (import $format
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
                (compiler_version "1.0")
                (depend
                    (module $std share "std" "1.0")
                )
                (import $std
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
                (compiler_version "1.0")
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

        let ref_module_entries = module_entries.iter().collect::<Vec<_>>();
        let index_entry = link(&ref_module_entries).unwrap();

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

        // assert!(index_entry.start_function_index_entries.is_empty());
        // assert!(index_entry.exit_function_index_entries.is_empty());
    }

    #[test]
    fn test_link_data() {
        let module_entries = assemble_from_strs(vec![
            r#"
            (module $myapp
                (compiler_version "1.0")
                (depend
                    (module $std share "std" "1.0")
                    (module $format share "format" "1.0")
                )
                (import $std
                    (data $foo "foo" read_only i32)
                )
                (import $format
                    (data $hello "hello" read_write i32)
                )
                (data $alice (read_only i32 23))
                (data $bob (read_write i64 29))
                (data $carol (read_write i64 31))
                (data $david (uninit i32))
            )
            "#,
            r#"
            (module $format
                (compiler_version "1.0")
                (depend
                    (module $std share "std" "1.0")
                )
                (import $std
                    (data $foo "foo" read_only i32)
                    (data $bar "bar" read_write i32)
                )
                (data export $baz (read_only i32 17))
                (data export $hello (read_write i32 19))
                (data export $world (uninit i32))
            )
            "#,
            r#"
            (module $std
                (compiler_version "1.0")
                (data export $foo (read_only i32 11))
                (data export $bar (read_write i32 13))
            )
            "#,
        ]);

        let ref_module_entries = module_entries.iter().collect::<Vec<_>>();
        let index_entry = link(&ref_module_entries).unwrap();

        let function_index_module_entries = &index_entry.function_index_module_entries;
        assert_eq!(function_index_module_entries.len(), 3);
        assert!(function_index_module_entries[0].index_entries.is_empty());
        assert!(function_index_module_entries[1].index_entries.is_empty());
        assert!(function_index_module_entries[2].index_entries.is_empty());

        let data_index_module_entries = &index_entry.data_index_module_entries;
        assert_eq!(data_index_module_entries.len(), 3);

        assert_eq!(
            data_index_module_entries[0].index_entries,
            vec![
                // std::foo
                DataIndexEntry {
                    data_public_index: 0,
                    target_module_index: 2,
                    data_internal_index: 0,
                    target_data_section_type: DataSectionType::ReadOnly
                },
                // myapp::alice
                DataIndexEntry {
                    data_public_index: 1,
                    target_module_index: 0,
                    data_internal_index: 0,
                    target_data_section_type: DataSectionType::ReadOnly
                },
                // format::hello
                DataIndexEntry {
                    data_public_index: 2,
                    target_module_index: 1,
                    data_internal_index: 0,
                    target_data_section_type: DataSectionType::ReadWrite
                },
                // myapp::bob
                DataIndexEntry {
                    data_public_index: 3,
                    target_module_index: 0,
                    data_internal_index: 0,
                    target_data_section_type: DataSectionType::ReadWrite
                },
                // myapp::carol
                DataIndexEntry {
                    data_public_index: 4,
                    target_module_index: 0,
                    data_internal_index: 1,
                    target_data_section_type: DataSectionType::ReadWrite
                },
                // myapp::david
                DataIndexEntry {
                    data_public_index: 5,
                    target_module_index: 0,
                    data_internal_index: 0,
                    target_data_section_type: DataSectionType::Uninit
                },
            ]
        );

        assert_eq!(
            data_index_module_entries[1].index_entries,
            vec![
                // std::foo
                DataIndexEntry {
                    data_public_index: 0,
                    target_module_index: 2,
                    data_internal_index: 0,
                    target_data_section_type: DataSectionType::ReadOnly
                },
                // format::baz
                DataIndexEntry {
                    data_public_index: 1,
                    target_module_index: 1,
                    data_internal_index: 0,
                    target_data_section_type: DataSectionType::ReadOnly
                },
                // std::bar
                DataIndexEntry {
                    data_public_index: 2,
                    target_module_index: 2,
                    data_internal_index: 0,
                    target_data_section_type: DataSectionType::ReadWrite
                },
                // format::hello
                DataIndexEntry {
                    data_public_index: 3,
                    target_module_index: 1,
                    data_internal_index: 0,
                    target_data_section_type: DataSectionType::ReadWrite
                },
                // format::world
                DataIndexEntry {
                    data_public_index: 4,
                    target_module_index: 1,
                    data_internal_index: 0,
                    target_data_section_type: DataSectionType::Uninit
                },
            ]
        );

        assert_eq!(
            data_index_module_entries[2].index_entries,
            vec![
                // std::foo
                DataIndexEntry {
                    data_public_index: 0,
                    target_module_index: 2,
                    data_internal_index: 0,
                    target_data_section_type: DataSectionType::ReadOnly
                },
                // std::bar
                DataIndexEntry {
                    data_public_index: 1,
                    target_module_index: 2,
                    data_internal_index: 0,
                    target_data_section_type: DataSectionType::ReadWrite
                },
            ]
        );

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

        // assert!(index_entry.start_function_index_entries.is_empty());
        // assert!(index_entry.exit_function_index_entries.is_empty());
    }

    #[test]
    fn test_link_external_functions() {
        let module_entries = assemble_from_strs(vec![
            r#"
            (module $myapp
                (compiler_version "1.0")
                (depend
                    (library $libc system "libc.so.6")
                )
                (external $libc
                    (function $fopen "fopen"
                        (params i64 i64) (result i64)
                    )
                    (function $fwrite "fwrite"
                        (params i64 i64 i64 i64) (result i64)
                    )
                    (function $fclose "fclose"
                        (params i64) (result i32)
                    )
                )
                (function $_entry (result i64)
                    (code)
                )
                (function $_start
                    (code)
                )
                (function $_exit
                    (code)
                )
            )
            "#,
            r#"
            (module $db
                (compiler_version "1.0")
                (depend
                    (library $libsqlite3 user "libsqlite3.so.0")
                    (library $libz share "libz.so.1")
                    (library $libc system "libc.so.6")
                )
                (external $libsqlite3
                    (function $sqlite3_open "sqlite3_open")
                    (function $sqlite3_exec "sqlite3_exec")
                    (function $sqlite3_close "sqlite3_close")
                )
                (external $libz
                    (function $inflateInit "inflateInit")
                    (function $inflate "inflate")
                    (function $inflateEnd "inflateEnd")
                )
                (external $libc
                    (function $fopen "fopen"
                        (params i64 i64) (result i64)
                    )
                    (function $fread "fread"
                        (params i64 i64 i64 i64) (result i64)
                    )
                    (function $fclose "fclose"
                        (params i64) (result i32)
                    )
                    (function $fstat "fstat"
                        (params i32 i64) (result i32)
                    )
                )
            )
            "#,
            r#"
            (module $compress
                (compiler_version "1.0")
                (depend
                    (library $libz share "libz.so.1")
                )
                (external $libz
                    (function $inflateInit "inflateInit")
                    (function $inflate "inflate")
                    (function $inflateEnd "inflateEnd")
                )
            )
            "#,
        ]);

        let ref_module_entries = module_entries.iter().collect::<Vec<_>>();
        let index_entry = link(&ref_module_entries).unwrap();

        let function_index_module_entries = &index_entry.function_index_module_entries;
        assert_eq!(function_index_module_entries.len(), 3);

        assert_eq!(
            function_index_module_entries[0],
            FunctionIndexModuleEntry {
                index_entries: vec![
                    // myapp::_entry
                    FunctionIndexEntry {
                        function_public_index: 0,
                        target_module_index: 0,
                        function_internal_index: 0
                    },
                    // myapp::_start
                    FunctionIndexEntry {
                        function_public_index: 1,
                        target_module_index: 0,
                        function_internal_index: 1
                    },
                    // myapp::_exit
                    FunctionIndexEntry {
                        function_public_index: 2,
                        target_module_index: 0,
                        function_internal_index: 2
                    },
                ]
            }
        );

        assert!(function_index_module_entries[1].index_entries.is_empty());

        assert!(function_index_module_entries[2].index_entries.is_empty());

        let data_index_module_entries = &index_entry.data_index_module_entries;
        assert_eq!(data_index_module_entries.len(), 3);
        assert!(data_index_module_entries[0].index_entries.is_empty());
        assert!(data_index_module_entries[1].index_entries.is_empty());
        assert!(data_index_module_entries[2].index_entries.is_empty());

        assert_eq!(
            index_entry.unified_external_library_entries,
            vec![
                UnifiedExternalLibraryEntry {
                    name: "libc.so.6".to_owned(),
                    external_library_type: ExternalLibraryType::System
                },
                UnifiedExternalLibraryEntry {
                    name: "libsqlite3.so.0".to_owned(),
                    external_library_type: ExternalLibraryType::User
                },
                UnifiedExternalLibraryEntry {
                    name: "libz.so.1".to_owned(),
                    external_library_type: ExternalLibraryType::Share
                },
            ]
        );

        assert_eq!(
            index_entry.unified_external_function_entries,
            vec![
                UnifiedExternalFunctionEntry {
                    name: "fopen".to_owned(),
                    unified_external_library_index: 0
                },
                UnifiedExternalFunctionEntry {
                    name: "fwrite".to_owned(),
                    unified_external_library_index: 0
                },
                UnifiedExternalFunctionEntry {
                    name: "fclose".to_owned(),
                    unified_external_library_index: 0
                },
                UnifiedExternalFunctionEntry {
                    name: "sqlite3_open".to_owned(),
                    unified_external_library_index: 1,
                },
                UnifiedExternalFunctionEntry {
                    name: "sqlite3_exec".to_owned(),
                    unified_external_library_index: 1,
                },
                UnifiedExternalFunctionEntry {
                    name: "sqlite3_close".to_owned(),
                    unified_external_library_index: 1,
                },
                UnifiedExternalFunctionEntry {
                    name: "inflateInit".to_owned(),
                    unified_external_library_index: 2,
                },
                UnifiedExternalFunctionEntry {
                    name: "inflate".to_owned(),
                    unified_external_library_index: 2,
                },
                UnifiedExternalFunctionEntry {
                    name: "inflateEnd".to_owned(),
                    unified_external_library_index: 2,
                },
                UnifiedExternalFunctionEntry {
                    name: "fread".to_owned(),
                    unified_external_library_index: 0,
                },
                UnifiedExternalFunctionEntry {
                    name: "fstat".to_owned(),
                    unified_external_library_index: 0,
                },
            ]
        );

        let external_function_index_module_entries =
            &index_entry.external_function_index_module_entries;
        assert_eq!(external_function_index_module_entries.len(), 3);
        assert_eq!(
            external_function_index_module_entries[0].index_entries,
            vec![
                ExternalFunctionIndexEntry {
                    external_function_index: 0,
                    unified_external_function_index: 0,
                    type_index: 0,
                },
                ExternalFunctionIndexEntry {
                    external_function_index: 1,
                    unified_external_function_index: 1,
                    type_index: 1,
                },
                ExternalFunctionIndexEntry {
                    external_function_index: 2,
                    unified_external_function_index: 2,
                    type_index: 2,
                },
            ]
        );
        assert_eq!(
            external_function_index_module_entries[1].index_entries,
            vec![
                ExternalFunctionIndexEntry {
                    external_function_index: 0,
                    unified_external_function_index: 3,
                    type_index: 0,
                },
                ExternalFunctionIndexEntry {
                    external_function_index: 1,
                    unified_external_function_index: 4,
                    type_index: 0,
                },
                ExternalFunctionIndexEntry {
                    external_function_index: 2,
                    unified_external_function_index: 5,
                    type_index: 0,
                },
                ExternalFunctionIndexEntry {
                    external_function_index: 3,
                    unified_external_function_index: 6,
                    type_index: 0,
                },
                ExternalFunctionIndexEntry {
                    external_function_index: 4,
                    unified_external_function_index: 7,
                    type_index: 0,
                },
                ExternalFunctionIndexEntry {
                    external_function_index: 5,
                    unified_external_function_index: 8,
                    type_index: 0,
                },
                ExternalFunctionIndexEntry {
                    external_function_index: 6,
                    unified_external_function_index: 0,
                    type_index: 1,
                },
                ExternalFunctionIndexEntry {
                    external_function_index: 7,
                    unified_external_function_index: 9,
                    type_index: 2,
                },
                ExternalFunctionIndexEntry {
                    external_function_index: 8,
                    unified_external_function_index: 2,
                    type_index: 3,
                },
                ExternalFunctionIndexEntry {
                    external_function_index: 9,
                    unified_external_function_index: 10,
                    type_index: 4,
                },
            ]
        );
        assert_eq!(
            external_function_index_module_entries[2].index_entries,
            vec![
                ExternalFunctionIndexEntry {
                    external_function_index: 0,
                    unified_external_function_index: 6,
                    type_index: 0,
                },
                ExternalFunctionIndexEntry {
                    external_function_index: 1,
                    unified_external_function_index: 7,
                    type_index: 0,
                },
                ExternalFunctionIndexEntry {
                    external_function_index: 2,
                    unified_external_function_index: 8,
                    type_index: 0,
                },
            ]
        );

        // assert!(index_entry.start_function_index_entries.is_empty());
        // assert!(index_entry.exit_function_index_entries.is_empty());
    }

    #[test]
    fn test_link_constructors_and_destructors() {
        let module_entries = assemble_from_strs(vec![
            r#"
            (module $myapp
                (compiler_version "1.0")
            )
            "#,
            r#"
            (module $mod_a
                (compiler_version "1.0")
                (constructor $start_a)
                (destructor $exit_a)
                (function $start_a (code))  // 1,0  start
                (function $exit_a (code))   // 1,1  exit
            )
            "#,
            r#"
            (module $mod_b
                (compiler_version "1.0")
                (constructor $start_b)
                (function $start_b (code))  // 2,0  start
            )
            "#,
            r#"
            (module $mod_c
                (compiler_version "1.0")
            )
            "#,
            r#"
            (module $mod_d
                (compiler_version "1.0")
                (destructor $exit_d)
                (function $exit_d (code))   // 4,0  exit
            )
            "#,
            r#"
            (module $mod_e
                (compiler_version "1.0")
                (constructor $start_e)
                (destructor $exit_e)
                (function $start_e (code))  // 5,0  start
                (function $exit_e (code))   // 5,1  exit
            )
            "#,
            r#"
            (module $mod_f
                (compiler_version "1.0")
                (destructor $exit_f)
                (function $exit_f (code))   // 6,0  exit
            )
            "#,
        ]);

        let ref_module_entries = module_entries.iter().collect::<Vec<_>>();
        let _index_entry = link(&ref_module_entries).unwrap();

        //         assert_eq!(
        //             index_entry.start_function_index_entries,
        //             vec![
        //                 ModuleFunctionIndexEntry::new(1, 0),
        //                 ModuleFunctionIndexEntry::new(2, 0),
        //                 ModuleFunctionIndexEntry::new(5, 0),
        //             ]
        //         );
        //
        //         assert_eq!(
        //             index_entry.exit_function_index_entries,
        //             vec![
        //                 ModuleFunctionIndexEntry::new(1, 1),
        //                 ModuleFunctionIndexEntry::new(4, 0),
        //                 ModuleFunctionIndexEntry::new(5, 1),
        //                 ModuleFunctionIndexEntry::new(6, 0),
        //             ]
        //         );
    }
}
