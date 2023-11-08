// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_binary::module_image::{
    func_index_section::{FuncIndexEntry, FuncIndexModuleEntry, FuncIndexSection},
    func_section::FuncSection,
    local_variable_section::LocalVariableSection,
    type_section::TypeSection,
    ModuleImage, SectionEntry,
};
use ancvm_program::program_settings::ProgramSettings;

use crate::{AssembleError, IndexEntry, ModuleEntry};

pub fn generate_image_binaries(
    module_entries: &[ModuleEntry],
    program_settings: &ProgramSettings,
) -> Result<Vec<Vec<u8>>, AssembleError> {
    let module_index_entry = link(module_entries, program_settings)?;

    let mut image_binaries = vec![];

    for module_entry in module_entries {
        // type, local, func sections
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

        // func index, data index sections

        let (func_index_range_items, func_index_items) =
            FuncIndexSection::convert_from_entries(&module_index_entry.func_index_module_entries);
        let func_index_section = FuncIndexSection {
            ranges: &func_index_range_items,
            items: &func_index_items,
        };

        // build ModuleImage instance
        let section_entries: Vec<&dyn SectionEntry> = vec![
            &type_section,
            &local_variable_section,
            &func_section,
            &func_index_section,
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
    program_settings: &ProgramSettings,
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
                .map(|(func_pub_index, func_entry)| {
                    FuncIndexEntry::new(func_pub_index, module_index, func_pub_index)
                })
                .collect::<Vec<_>>();
            FuncIndexModuleEntry::new(entries)
        })
        .collect::<Vec<_>>();

    Ok(IndexEntry {
        func_index_module_entries,
    })
}

#[cfg(test)]
mod tests {
    use ancvm_assembly_parser::{
        instruction_kind::init_instruction_kind_table, lexer::lex, parser::parse,
        peekable_iterator::PeekableIterator,
    };
    use ancvm_program::{program_settings::ProgramSettings, program_source::ProgramSource};
    use ancvm_runtime::{
        in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
    };
    use ancvm_types::ForeignValue;

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
            (fn $main
                (param $a i32) (param $b i32)
                (results i32 i32)
                (code
                    (local.load32 $a)
                    (local.load32 $b)
                )
            )
        )
        "#,
        );

        let program_source0 = InMemoryProgramSource::new(module_binaries);
        let program0 = program_source0.build_program().unwrap();
        let mut thread_context0 = program0.create_thread_context();

        let result0 = process_function(
            &mut thread_context0,
            0,
            0,
            &[ForeignValue::UInt32(11), ForeignValue::UInt32(13)],
        );
        assert_eq!(
            result0.unwrap(),
            vec![ForeignValue::UInt32(11), ForeignValue::UInt32(13),]
        );
    }
}
