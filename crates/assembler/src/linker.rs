// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_binary::module_image::{
    data_index_section::{DataIndexEntry, DataIndexModuleEntry, DataIndexSection},
    data_section::{ReadOnlyDataSection, ReadWriteDataSection, UninitDataSection},
    func_index_section::{FuncIndexEntry, FuncIndexModuleEntry, FuncIndexSection},
    func_section::FuncSection,
    local_variable_section::LocalVariableSection,
    type_section::TypeSection,
    ModuleImage, SectionEntry,
};
use ancvm_program::program_settings::ProgramSettings;
use ancvm_types::DataSectionType;

use crate::{AssembleError, IndexEntry, ModuleEntry};

pub fn generate_image_binaries(
    module_entries: &[ModuleEntry],
    program_settings: &ProgramSettings,
) -> Result<Vec<Vec<u8>>, AssembleError> {
    let module_index_entry = link(module_entries, program_settings)?;

    let mut image_binaries = vec![];

    for module_entry in module_entries {
        // type, local, func, data sections
        let (type_items, type_data) = TypeSection::convert_from_entries(&module_entry.type_entries);
        let type_section = TypeSection {
            items: &type_items,
            types_data: &type_data,
        };

        let (local_list_items, local_list_data) =
            LocalVariableSection::convert_from_entries(&module_entry.local_list_entries);
        let local_variable_section = LocalVariableSection {
            lists: &local_list_items,
            list_data: &local_list_data,
        };

        let (func_items, func_data) = FuncSection::convert_from_entries(&module_entry.func_entries);
        let func_section = FuncSection {
            items: &func_items,
            codes_data: &func_data,
        };

        let (read_only_data_items, read_only_data) =
            ReadOnlyDataSection::convert_from_entries(&module_entry.read_only_data_entries);
        let read_only_data_section = ReadOnlyDataSection {
            items: &read_only_data_items,
            datas_data: &read_only_data,
        };

        let (read_write_data_items, read_write_data) =
            ReadWriteDataSection::convert_from_entries(&module_entry.read_write_data_entries);
        let read_write_data_section = ReadWriteDataSection {
            items: &read_write_data_items,
            datas_data: &read_write_data,
        };

        let uninit_data_items =
            UninitDataSection::convert_from_entries(&module_entry.uninit_data_entries);
        let uninit_data_section = UninitDataSection {
            items: &uninit_data_items,
        };

        // func, data index section

        let (func_index_range_items, func_index_items) =
            FuncIndexSection::convert_from_entries(&module_index_entry.func_index_module_entries);
        let func_index_section = FuncIndexSection {
            ranges: &func_index_range_items,
            items: &func_index_items,
        };

        let (data_index_range_items, data_index_items) =
            DataIndexSection::convert_from_entries(&module_index_entry.data_index_module_entries);
        let data_index_section = DataIndexSection {
            ranges: &data_index_range_items,
            items: &data_index_items,
        };

        // build ModuleImage instance
        let section_entries: Vec<&dyn SectionEntry> = vec![
            &type_section,
            &local_variable_section,
            &func_section,
            &read_only_data_section,
            &read_write_data_section,
            &uninit_data_section,
            &func_index_section,
            &data_index_section,
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

    Ok(IndexEntry {
        func_index_module_entries,
        data_index_module_entries,
    })
}

#[cfg(test)]
mod tests {
    use ancvm_binary::{
        bytecode_reader::print_bytecode_as_text,
        module_image::{
            local_variable_section::{LocalListEntry, LocalVariableEntry},
            type_section::TypeEntry,
        },
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
    fn test_assemble_process_function() {
        let module_binaries = assemble_single_module(
            r#"
        (module $app
            (runtime_version "1.0")
            (func $main
                (param $a i32) (param $b i32)
                (results i32)
                (local $c i32)
                (code
                    (local.store32 $c
                        (i32.add
                            (local.load32_i32 $a)
                            (local.load32_i32 $b)
                        )
                    )
                    (local.load32_i32 $c)
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

        let bytecode_text = print_bytecode_as_text(&func_entry.code);

        // println!("{}", bytecode_text);

        assert_eq!(
            "\
0x0000  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x0008  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x0010  00 07                       i32.add
0x0012  09 02 00 00  00 00 02 00    local.store32     rev:0   off:0x00  idx:2
0x001a  02 02 00 00  00 00 02 00    local.load32_i32  rev:0   off:0x00  idx:2
0x0022  00 0a                       end",
            bytecode_text
        );

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
