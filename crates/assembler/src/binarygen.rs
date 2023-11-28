// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_binary::module_image::{
    data_index_section::DataIndexSection,
    data_name_section::DataNameSection,
    data_section::{ReadOnlyDataSection, ReadWriteDataSection, UninitDataSection},
    external_func_index_section::ExternalFuncIndexSection,
    external_func_section::ExternalFuncSection,
    external_library_section::ExternalLibrarySection,
    func_index_section::FuncIndexSection,
    func_name_section::FuncNameSection,
    func_section::FuncSection,
    local_variable_section::LocalVariableSection,
    type_section::TypeSection,
    unified_external_func_section::UnifiedExternalFuncSection,
    unified_external_library_section::UnifiedExternalLibrarySection,
    ModuleImage, SectionEntry,
};

use crate::{AssembleError, IndexEntry, ModuleEntry};

pub fn generate_module_image_binary(
    module_entry: &ModuleEntry,
    index_entry_opt: Option<&IndexEntry>,
) -> Result<Vec<u8>, AssembleError> {
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

    // func name section
    let (func_name_items, func_name_data) =
        FuncNameSection::convert_from_entries(&module_entry.func_name_entries);
    let func_name_section = FuncNameSection {
        items: &func_name_items,
        names_data: &func_name_data,
    };

    // data name section
    let (data_name_items, data_name_data) =
        DataNameSection::convert_from_entries(&module_entry.data_name_entries);
    let data_name_section = DataNameSection {
        items: &data_name_items,
        names_data: &data_name_data,
    };

    let mut section_entries: Vec<&dyn SectionEntry> = vec![
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
    ];

    let image_binary = if let Some(index_entry) = index_entry_opt {
        // func index
        let (func_index_range_items, func_index_items) =
            FuncIndexSection::convert_from_entries(&index_entry.func_index_module_entries);
        let func_index_section = FuncIndexSection {
            ranges: &func_index_range_items,
            items: &func_index_items,
        };

        // data index
        let (data_index_range_items, data_index_items) =
            DataIndexSection::convert_from_entries(&index_entry.data_index_module_entries);
        let data_index_section = DataIndexSection {
            ranges: &data_index_range_items,
            items: &data_index_items,
        };

        // unified external library
        let (unified_external_library_items, unified_external_library_names_data) =
            UnifiedExternalLibrarySection::convert_from_entries(
                &index_entry.unified_external_library_entries,
            );
        let unified_external_library_section = UnifiedExternalLibrarySection {
            items: &unified_external_library_items,
            names_data: &unified_external_library_names_data,
        };

        // unified external function
        let (unified_external_func_items, unified_external_func_names_data) =
            UnifiedExternalFuncSection::convert_from_entries(
                &index_entry.unified_external_func_entries,
            );
        let unified_external_func_section = UnifiedExternalFuncSection {
            items: &unified_external_func_items,
            names_data: &unified_external_func_names_data,
        };

        // external function index
        let (external_func_ranges, external_func_items) =
            ExternalFuncIndexSection::convert_from_entries(
                &index_entry.external_func_index_module_entries,
            );
        let external_func_index_section = ExternalFuncIndexSection {
            ranges: &external_func_ranges,
            items: &external_func_items,
        };

        let mut index_section_entries: Vec<&dyn SectionEntry> = vec![
            &func_index_section,
            &data_index_section,
            &unified_external_library_section,
            &unified_external_func_section,
            &external_func_index_section,
        ];

        section_entries.append(&mut index_section_entries);

        // build ModuleImage instance
        let (section_items, sections_data) = ModuleImage::convert_from_entries(&section_entries);
        let module_image = ModuleImage {
            name: &module_entry.name,
            items: &section_items,
            sections_data: &sections_data,
        };

        // save
        let mut image_binary: Vec<u8> = Vec::new();
        module_image.save(&mut image_binary).unwrap();
        image_binary
    } else {
        // build ModuleImage instance
        let (section_items, sections_data) = ModuleImage::convert_from_entries(&section_entries);
        let module_image = ModuleImage {
            name: &module_entry.name,
            items: &section_items,
            sections_data: &sections_data,
        };

        // save
        let mut image_binary: Vec<u8> = Vec::new();
        module_image.save(&mut image_binary).unwrap();
        image_binary
    };

    Ok(image_binary)
}

#[cfg(test)]
mod tests {
    use ancvm_binary::module_image::{
        local_variable_section::{LocalListEntry, LocalVariableEntry},
        type_section::TypeEntry,
    };

    use ancvm_process::{
        in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
    };
    use ancvm_program::program_source::ProgramSource;
    use ancvm_types::{DataType, ForeignValue, MemoryDataType};

    use crate::utils::helper_generate_module_image_binaries_from_single_module_assembly;

    #[test]
    fn test_binarygen() {
        let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
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
