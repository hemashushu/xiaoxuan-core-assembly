// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_binary::module_image::{
    data_index_section::DataIndexSection,
    data_name_section::DataNameSection,
    data_section::{ReadOnlyDataSection, ReadWriteDataSection, UninitDataSection},
    // exit_function_list_section::ExitFunctionListSection,
    external_function_index_section::ExternalFunctionIndexSection,
    external_function_section::ExternalFunctionSection,
    external_library_section::ExternalLibrarySection,
    function_index_section::FunctionIndexSection,
    function_name_section::FunctionNameSection,
    function_section::FunctionSection,
    import_data_section::ImportDataSection,
    import_function_section::ImportFunctionSection,
    local_variable_section::LocalVariableSection,
    property_section::PropertySection,
    // start_function_list_section::StartFunctionListSection,
    type_section::TypeSection,
    unified_external_function_section::UnifiedExternalFunctionSection,
    unified_external_library_section::UnifiedExternalLibrarySection,
    ModuleImage, SectionEntry, MODULE_NAME_BUFFER_LENGTH,
};
use ancvm_types::{
    entry::{IndexEntry, ModuleEntry},
    RUNTIME_MAJOR_VERSION, RUNTIME_MINOR_VERSION,
};

use crate::AssembleError;

pub fn generate_module_image_binary(
    module_entry: &ModuleEntry,
    index_entry_opt: Option<&IndexEntry>,
) -> Result<Vec<u8>, AssembleError> {
    // property section
    let name = &module_entry.name;
    let name_bytes = name.as_bytes();
    let mut module_name_buffer = [0u8; MODULE_NAME_BUFFER_LENGTH];
    unsafe {
        std::ptr::copy(
            name_bytes.as_ptr(),
            module_name_buffer.as_mut_ptr(),
            name_bytes.len(),
        )
    };

    let constructor_function_public_index = module_entry
        .constructor_function_public_index
        .unwrap_or(u32::MAX);
    let destructor_function_public_index = module_entry
        .destructor_function_public_index
        .unwrap_or(u32::MAX);
    let entry_function_public_index =
        index_entry_opt.map_or(u32::MAX, |idx_entry| idx_entry.entry_function_public_index);

    let property_section = PropertySection {
        entry_function_public_index,
        constructor_function_public_index,
        destructor_function_public_index,
        runtime_major_version: RUNTIME_MAJOR_VERSION,
        runtime_minor_version: RUNTIME_MINOR_VERSION,
        module_name_length: name_bytes.len() as u32,
        module_name_buffer,
    };

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
    let (function_items, function_data) =
        FunctionSection::convert_from_entries(&module_entry.function_entries);
    let function_section = FunctionSection {
        items: &function_items,
        codes_data: &function_data,
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
    let (external_function_items, external_function_names_data) =
        ExternalFunctionSection::convert_from_entries(&module_entry.external_function_entries);
    let external_function_section = ExternalFunctionSection {
        items: &external_function_items,
        names_data: &external_function_names_data,
    };

    // import function section
    let (import_function_items, import_function_data) =
        ImportFunctionSection::convert_from_entries(&module_entry.import_function_entries);
    let import_function_section = ImportFunctionSection {
        items: &import_function_items,
        names_data: &import_function_data,
    };

    // import data entries
    let (import_data_items, import_data) =
        ImportDataSection::convert_from_entries(&module_entry.import_data_entries);
    let import_data_section = ImportDataSection {
        items: &import_data_items,
        names_data: &import_data,
    };

    // func name section
    let (function_name_items, function_name_data) =
        FunctionNameSection::convert_from_entries(&module_entry.function_name_entries);
    let function_name_section = FunctionNameSection {
        items: &function_name_items,
        names_data: &function_name_data,
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
        &function_section,
        &read_only_data_section,
        &read_write_data_section,
        &uninit_data_section,
        &external_library_section,
        &external_function_section,
        &import_function_section,
        &import_data_section,
        &function_name_section,
        &data_name_section,
        &property_section,
    ];

    if let Some(index_entry) = index_entry_opt {
        // build application binary

        // func index
        let (function_index_range_items, function_index_items) =
            FunctionIndexSection::convert_from_entries(&index_entry.function_index_module_entries);
        let function_index_section = FunctionIndexSection {
            ranges: &function_index_range_items,
            items: &function_index_items,
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
        let (unified_external_function_items, unified_external_function_names_data) =
            UnifiedExternalFunctionSection::convert_from_entries(
                &index_entry.unified_external_function_entries,
            );
        let unified_external_function_section = UnifiedExternalFunctionSection {
            items: &unified_external_function_items,
            names_data: &unified_external_function_names_data,
        };

        // external function index
        let (external_function_ranges, external_function_items) =
            ExternalFunctionIndexSection::convert_from_entries(
                &index_entry.external_function_index_module_entries,
            );
        let external_function_index_section = ExternalFunctionIndexSection {
            ranges: &external_function_ranges,
            items: &external_function_items,
        };

//         let start_function_list_section = StartFunctionListSection {
//             items: &index_entry.start_function_public_indices,
//         };
//
//         let exit_function_list_section = ExitFunctionListSection {
//             items: &index_entry.exit_function_public_indices,
//         };

        // let property_section = PropertySection {
        //     entry_function_public_index: index_entry.entry_function_public_index,
        //     constructor_function_public_index: index_entry
        // };

        let mut index_section_entries: Vec<&dyn SectionEntry> = vec![
            &function_index_section,
            &data_index_section,
            &unified_external_library_section,
            &unified_external_function_section,
            &external_function_index_section,
            // &start_function_list_section,
            // &exit_function_list_section,
        ];

        section_entries.append(&mut index_section_entries);

        // build ModuleImage instance
        let (section_items, sections_data) = ModuleImage::convert_from_entries(&section_entries);
        let module_image = ModuleImage {
            items: &section_items,
            sections_data: &sections_data,
        };

        // save
        let mut image_binary: Vec<u8> = Vec::new();
        module_image.save(&mut image_binary).unwrap();

        Ok(image_binary)
    } else {
        // build share module binary

        let (section_items, sections_data) = ModuleImage::convert_from_entries(&section_entries);
        let module_image = ModuleImage {
            items: &section_items,
            sections_data: &sections_data,
        };

        // save
        let mut image_binary: Vec<u8> = Vec::new();
        module_image.save(&mut image_binary).unwrap();
        Ok(image_binary)
    }
}

#[cfg(test)]
mod tests {
    use ancvm_context::program_resource::ProgramResource;
    use ancvm_processor::{
        in_memory_program_resource::InMemoryProgramResource, interpreter::process_function,
    };
    use ancvm_types::{
        entry::{LocalListEntry, LocalVariableEntry, TypeEntry},
        DataType, ForeignValue, MemoryDataType,
    };

    use crate::utils::helper_generate_module_image_binary_from_str;

    #[test]
    fn test_binarygen_base() {
        let module_binary = helper_generate_module_image_binary_from_str(
            r#"
        (module $app
            (runtime_version "1.0")
            (function $test
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

        let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
        let process_context0 = program_resource0.create_process_context().unwrap();

        let function_entry = process_context0.module_images[0]
            .get_function_section()
            .get_function_entry(0);

        assert_eq!(function_entry.type_index, 0);

        let type_entry = process_context0.module_images[0]
            .get_type_section()
            .get_type_entry(function_entry.type_index);

        assert_eq!(
            type_entry,
            TypeEntry {
                params: vec![DataType::I32, DataType::I32],
                results: vec![DataType::I32]
            }
        );

        assert_eq!(function_entry.local_variable_list_index, 0);

        let local_list_entry = process_context0.module_images[0]
            .get_local_variable_section()
            .get_local_list_entry(function_entry.local_variable_list_index);

        assert_eq!(
            local_list_entry,
            LocalListEntry {
                local_variable_entries: vec![
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

        let mut thread_context0 = process_context0.create_thread_context();

        let result0 = process_function(
            &mut thread_context0,
            0,
            0,
            &[ForeignValue::U32(11), ForeignValue::U32(13)],
        );

        assert_eq!(result0.unwrap(), vec![ForeignValue::U32(24)]);
    }

    #[test]
    fn test_binarygen_other_sections_todo() {
        // todo
    }
}
