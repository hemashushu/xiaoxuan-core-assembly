// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_binary::module_image::{
    data_index_section::{DataIndexEntry, DataIndexModuleEntry, DataIndexSection},
    data_name_section::DataNameSection,
    data_section::{ReadOnlyDataSection, ReadWriteDataSection, UninitDataSection},
    external_func_index_section::{
        ExternalFuncIndexEntry, ExternalFuncIndexModuleEntry, ExternalFuncIndexSection,
    },
    external_func_name_section::ExternalFuncNameSection,
    external_func_section::ExternalFuncSection,
    external_library_section::ExternalLibrarySection,
    func_index_section::{FuncIndexEntry, FuncIndexModuleEntry, FuncIndexSection},
    func_name_section::FuncNameSection,
    func_section::FuncSection,
    local_variable_section::LocalVariableSection,
    type_section::TypeSection,
    unified_external_func_section::{UnifiedExternalFuncEntry, UnifiedExternalFuncSection},
    unified_external_library_section::{
        UnifiedExternalLibraryEntry, UnifiedExternalLibrarySection,
    },
    ModuleImage, SectionEntry,
};
use ancvm_program::program_settings::ProgramSettings;
use ancvm_types::DataSectionType;

use crate::{AssembleError, IndexEntry, ModuleEntry};

pub fn generate_image_binaries(
    module_entries: &[ModuleEntry],
    program_settings: &ProgramSettings,
) -> Result<Vec<Vec<u8>>, AssembleError> {
    let mut image_binaries = vec![];

    for module_entry in module_entries {
        // type section
        let (type_items, type_data) = TypeSection::convert_from_entries(&module_entry.type_entries);
        let type_section = TypeSection {
            items: &type_items,
            types_data: &type_data,
        };

        // local variable section
        let (local_list_items, local_list_data) =
            LocalVariableSection::convert_from_entries(&module_entry.local_list_entries);
        let local_variable_section = LocalVariableSection {
            lists: &local_list_items,
            list_data: &local_list_data,
        };

        // function section
        let (func_items, func_data) = FuncSection::convert_from_entries(&module_entry.func_entries);
        let func_section = FuncSection {
            items: &func_items,
            codes_data: &func_data,
        };

        // ro data section
        let (read_only_data_items, read_only_data) =
            ReadOnlyDataSection::convert_from_entries(&module_entry.read_only_data_entries);
        let read_only_data_section = ReadOnlyDataSection {
            items: &read_only_data_items,
            datas_data: &read_only_data,
        };

        // rw data section
        let (read_write_data_items, read_write_data) =
            ReadWriteDataSection::convert_from_entries(&module_entry.read_write_data_entries);
        let read_write_data_section = ReadWriteDataSection {
            items: &read_write_data_items,
            datas_data: &read_write_data,
        };

        // uninitialized data section
        let uninit_data_items =
            UninitDataSection::convert_from_entries(&module_entry.uninit_data_entries);
        let uninit_data_section = UninitDataSection {
            items: &uninit_data_items,
        };

        // external library section
        let (external_library_items, external_library_names_data) =
            ExternalLibrarySection::convert_from_entries(&module_entry.external_library_entries);
        let external_library_section = ExternalLibrarySection {
            items: &external_library_items,
            names_data: &external_library_names_data,
        };

        // external function section
        let (external_func_items, external_func_names_data) =
            ExternalFuncSection::convert_from_entries(&module_entry.external_func_entries);
        let external_func_section = ExternalFuncSection {
            items: &external_func_items,
            names_data: &external_func_names_data,
        };

        let (func_name_items, func_name_data) =
            FuncNameSection::convert_from_entries(&module_entry.func_name_entries);
        let func_name_section = FuncNameSection {
            items: &func_name_items,
            names_data: &func_name_data,
        };

        let (data_name_items, data_name_data) =
            DataNameSection::convert_from_entries(&module_entry.data_name_entries);
        let data_name_section = DataNameSection {
            items: &data_name_items,
            names_data: &data_name_data,
        };

        // external function name section
        let (external_func_name_items, external_func_name_data) =
            ExternalFuncNameSection::convert_from_entries(&module_entry.external_func_name_entries);
        let external_func_name_section = ExternalFuncNameSection {
            items: &external_func_name_items,
            names_data: &external_func_name_data,
        };

        // link functions, datas, external functions
        let module_index_entry = link(module_entries, program_settings)?;

        // func index
        let (func_index_range_items, func_index_items) =
            FuncIndexSection::convert_from_entries(&module_index_entry.func_index_module_entries);
        let func_index_section = FuncIndexSection {
            ranges: &func_index_range_items,
            items: &func_index_items,
        };

        // data index
        let (data_index_range_items, data_index_items) =
            DataIndexSection::convert_from_entries(&module_index_entry.data_index_module_entries);
        let data_index_section = DataIndexSection {
            ranges: &data_index_range_items,
            items: &data_index_items,
        };

        // unified external library
        let (unified_external_library_items, unified_external_library_names_data) =
            UnifiedExternalLibrarySection::convert_from_entries(
                &module_index_entry.unified_external_library_entries,
            );
        let unified_external_library_section = UnifiedExternalLibrarySection {
            items: &unified_external_library_items,
            names_data: &unified_external_library_names_data,
        };

        // unified external function
        let (unified_external_func_items, unified_external_func_names_data) =
            UnifiedExternalFuncSection::convert_from_entries(
                &module_index_entry.unified_external_func_entries,
            );
        let unified_external_func_section = UnifiedExternalFuncSection {
            items: &unified_external_func_items,
            names_data: &unified_external_func_names_data,
        };

        // external function index
        let (external_func_ranges, external_func_items) =
            ExternalFuncIndexSection::convert_from_entries(
                &module_index_entry.external_func_index_module_entries,
            );
        let external_func_index_section = ExternalFuncIndexSection {
            ranges: &external_func_ranges,
            items: &external_func_items,
        };

        // build ModuleImage instance
        let section_entries: Vec<&dyn SectionEntry> = vec![
            &type_section,
            &local_variable_section,
            &func_section,
            &read_only_data_section,
            &read_write_data_section,
            &uninit_data_section,
            &external_library_section,
            &external_func_section,
            &func_name_section,
            &data_name_section,
            &external_func_name_section,
            //
            &func_index_section,
            &data_index_section,
            &unified_external_library_section,
            &unified_external_func_section,
            &external_func_index_section,
        ];

        let (section_items, sections_data) = ModuleImage::convert_from_entries(&section_entries);
        let module_image = ModuleImage {
            name: &module_entry.name,
            items: &section_items,
            sections_data: &sections_data,
        };

        // save
        let mut image_binary: Vec<u8> = Vec::new();
        module_image.save(&mut image_binary).unwrap();

        image_binaries.push(image_binary);
    }

    Ok(image_binaries)
}

pub fn link(
    module_entries: &[ModuleEntry],
    _program_settings: &ProgramSettings,
) -> Result<IndexEntry, AssembleError> {
    // todo
    // load shared modules

    // TEMPORARY, NO LINKING
    let func_index_module_entries = module_entries
        .iter()
        .enumerate()
        .map(|(module_index, module_entry)| {
            let entries = module_entry
                .func_entries
                .iter()
                .enumerate()
                .map(|(func_pub_index, _func_entry)| {
                    FuncIndexEntry::new(func_pub_index, module_index, func_pub_index)
                })
                .collect::<Vec<_>>();
            FuncIndexModuleEntry::new(entries)
        })
        .collect::<Vec<_>>();

    // TEMPORARY, NO LINKING
    // note that data internal index is section relevant.
    // e.g. there are indices 0,1,2,3... in read-only section, and
    // there are also indices 0,1,2,3... in read-write section.
    let data_index_module_entries = module_entries
        .iter()
        .enumerate()
        .map(|(module_index, module_entry)| {
            let mut data_pub_index = 0;
            let mut entries = vec![];

            for (data_internal_idx, _read_only_data_entry) in
                module_entry.read_only_data_entries.iter().enumerate()
            {
                entries.push(DataIndexEntry::new(
                    data_pub_index,
                    module_index,
                    data_internal_idx,
                    DataSectionType::ReadOnly,
                ));

                data_pub_index += 1;
            }

            for (data_internal_idx, _read_write_data_entry) in
                module_entry.read_write_data_entries.iter().enumerate()
            {
                entries.push(DataIndexEntry::new(
                    data_pub_index,
                    module_index,
                    data_internal_idx,
                    DataSectionType::ReadWrite,
                ));

                data_pub_index += 1;
            }

            for (data_internal_idx, _uninit_data_entry) in
                module_entry.uninit_data_entries.iter().enumerate()
            {
                entries.push(DataIndexEntry::new(
                    data_pub_index,
                    module_index,
                    data_internal_idx,
                    DataSectionType::Uninit,
                ));

                data_pub_index += 1;
            }

            DataIndexModuleEntry::new(entries)
        })
        .collect::<Vec<_>>();

    // linking external functions

    let mut unified_external_library_entries: Vec<UnifiedExternalLibraryEntry> = vec![];
    let mut unified_external_func_entries: Vec<UnifiedExternalFuncEntry> = vec![];
    let mut external_func_index_module_entries: Vec<ExternalFuncIndexModuleEntry> = vec![];

    for module_entry in module_entries {
        let mut external_func_index_entries: Vec<ExternalFuncIndexEntry> = vec![];

        for (original_external_library_index, external_library_entry) in
            module_entry.external_library_entries.iter().enumerate()
        {
            let unified_lib_idx_opt = unified_external_library_entries.iter().position(|entry| {
                entry.external_library_type == external_library_entry.external_library_type
                    && entry.name == external_library_entry.name
            });

            // add unified library entry when not found
            let unified_lib_idx = if let Some(idx) = unified_lib_idx_opt {
                idx
            } else {
                let idx = unified_external_library_entries.len();
                let unified_external_library_entry = UnifiedExternalLibraryEntry {
                    name: external_library_entry.name.clone(),
                    external_library_type: external_library_entry.external_library_type,
                };
                unified_external_library_entries.push(unified_external_library_entry);
                idx
            };

            // filter external functions by the specified external libray
            let external_func_entries_with_indices = module_entry
                .external_func_entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| {
                    entry.external_library_index == original_external_library_index
                })
                .collect::<Vec<_>>();

            for (original_external_func_index, external_func_entry) in
                external_func_entries_with_indices
            {
                let unified_func_idx_opt = unified_external_func_entries.iter().position(|entry| {
                    entry.unified_external_library_index == unified_lib_idx
                        && entry.name == external_func_entry.name
                });

                // add unified function entry when not found, and then
                // add external function index entry
                if unified_func_idx_opt.is_none() {
                    // add unified function entry
                    let unified_func_idx = unified_external_func_entries.len();
                    let unified_external_func_entry = UnifiedExternalFuncEntry {
                        name: external_func_entry.name.clone(),
                        unified_external_library_index: unified_lib_idx,
                    };
                    unified_external_func_entries.push(unified_external_func_entry);

                    // add external function index entry
                    let external_func_index_entry = ExternalFuncIndexEntry {
                        external_func_index: original_external_func_index,
                        unified_external_func_index: unified_func_idx,
                        type_index: external_func_entry.type_index,
                    };
                    external_func_index_entries.push(external_func_index_entry);
                }
            }
        }

        let external_func_index_module_entry = ExternalFuncIndexModuleEntry {
            index_entries: external_func_index_entries,
        };
        external_func_index_module_entries.push(external_func_index_module_entry);
    }

    Ok(IndexEntry {
        func_index_module_entries,
        data_index_module_entries,
        unified_external_library_entries,
        unified_external_func_entries,
        external_func_index_module_entries,
    })
}

#[cfg(test)]
mod tests {
    use ancvm_binary::module_image::{
        local_variable_section::{LocalListEntry, LocalVariableEntry},
        type_section::TypeEntry,
    };
    use ancvm_parser::{
        instruction_kind::init_instruction_kind_table, lexer::lex, parser::parse,
        peekable_iterator::PeekableIterator,
    };
    use ancvm_program::{program_settings::ProgramSettings, program_source::ProgramSource};
    use ancvm_runtime::{
        in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
    };
    use ancvm_types::{DataType, ForeignValue, MemoryDataType};

    use crate::assembler::assemble_module_node;

    use super::generate_image_binaries;

    fn assemble_single_module(source: &str) -> Vec<Vec<u8>> {
        init_instruction_kind_table();

        let mut chars = source.chars();
        let mut char_iter = PeekableIterator::new(&mut chars, 2);
        let mut tokens = lex(&mut char_iter).unwrap().into_iter();
        let mut token_iter = PeekableIterator::new(&mut tokens, 2);
        let module_node = parse(&mut token_iter).unwrap();

        let module_entry = assemble_module_node(&module_node).unwrap();
        let program_settings = ProgramSettings::default();
        generate_image_binaries(&vec![module_entry], &program_settings).unwrap()
    }

    #[test]
    fn test_assemble_function() {
        let module_binaries = assemble_single_module(
            r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
                (param $a i32) (param $b i32)
                (results i32)
                (local $c i32)
                (code
                    (i32.add
                        (local.load32_i32 $a)
                        (local.load32_i32 $b)
                    )
                )
            )
        )
        "#,
        );

        let program_source0 = InMemoryProgramSource::new(module_binaries);
        let program0 = program_source0.build_program().unwrap();

        let func_entry = program0.module_images[0]
            .get_func_section()
            .get_func_entry(0);

        assert_eq!(func_entry.type_index, 0);

        let type_entry = program0.module_images[0]
            .get_type_section()
            .get_type_entry(func_entry.type_index);

        assert_eq!(
            type_entry,
            TypeEntry {
                params: vec![DataType::I32, DataType::I32],
                results: vec![DataType::I32]
            }
        );

        assert_eq!(func_entry.local_list_index, 0);

        let local_list_entry = program0.module_images[0]
            .get_local_variable_section()
            .get_local_list_entry(func_entry.local_list_index);

        assert_eq!(
            local_list_entry,
            LocalListEntry {
                variable_entries: vec![
                    LocalVariableEntry {
                        memory_data_type: MemoryDataType::I32,
                        length: 4,
                        align: 4
                    },
                    LocalVariableEntry {
                        memory_data_type: MemoryDataType::I32,
                        length: 4,
                        align: 4
                    },
                    LocalVariableEntry {
                        memory_data_type: MemoryDataType::I32,
                        length: 4,
                        align: 4
                    }
                ]
            }
        );

        let mut thread_context0 = program0.create_thread_context();

        let result0 = process_function(
            &mut thread_context0,
            0,
            0,
            &[ForeignValue::UInt32(11), ForeignValue::UInt32(13)],
        );

        assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(24)]);
    }
}
