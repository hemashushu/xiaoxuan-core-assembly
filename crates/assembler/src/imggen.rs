// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::io::Write;

use anc_image::{
    common_sections::{
        common_property_section::CommonPropertySection,
        data_name_section::DataNameSection,
        data_section::{ReadOnlyDataSection, ReadWriteDataSection, UninitDataSection},
        external_function_section::ExternalFunctionSection,
        external_library_section::ExternalLibrarySection,
        function_name_section::FunctionNameSection,
        function_section::FunctionSection,
        import_data_section::ImportDataSection,
        import_function_section::ImportFunctionSection,
        local_variable_section::LocalVariableSection,
        type_section::TypeSection,
    },
    module_image::{ImageType, ModuleImage, SectionEntry},
};

use crate::{entry::ImageCommonEntry, AssembleError};

pub fn generate_object_file(
    image_common_entry: &ImageCommonEntry,
    writer: &mut dyn Write,
) -> Result<(), AssembleError> {
    // property section
    let common_property_section = CommonPropertySection::new(
        &image_common_entry.name,
        image_common_entry.import_data_entries.len() as u32,
        image_common_entry.import_function_entries.len() as u32,
    );

    // type section
    let (type_items, types_data) =
        TypeSection::convert_from_entries(&image_common_entry.type_entries);
    let type_section = TypeSection {
        items: &type_items,
        types_data: &types_data,
    };

    // local variable section
    let (local_list_items, local_list_data) =
        LocalVariableSection::convert_from_entries(&image_common_entry.local_variable_list_entries);
    let local_variable_section = LocalVariableSection {
        list_items: &local_list_items,
        list_data: &local_list_data,
    };

    // function section
    let (function_items, function_codes_data) =
        FunctionSection::convert_from_entries(&image_common_entry.function_entries);
    let function_section = FunctionSection {
        items: &function_items,
        codes_data: &function_codes_data,
    };

    // ro data section
    let (read_only_data_items, read_only_data) =
        ReadOnlyDataSection::convert_from_entries(&image_common_entry.read_only_data_entries);
    let read_only_data_section = ReadOnlyDataSection {
        items: &read_only_data_items,
        datas_data: &read_only_data,
    };

    // rw data section
    let (read_write_data_items, read_write_data) =
        ReadWriteDataSection::convert_from_entries(&image_common_entry.read_write_data_entries);
    let read_write_data_section = ReadWriteDataSection {
        items: &read_write_data_items,
        datas_data: &read_write_data,
    };

    // uninitialized data section
    let uninit_data_items =
        UninitDataSection::convert_from_entries(&image_common_entry.uninit_data_entries);
    let uninit_data_section = UninitDataSection {
        items: &uninit_data_items,
    };

    // external library section
    let (external_library_items, external_library_names_data) =
        ExternalLibrarySection::convert_from_entries(&image_common_entry.external_library_entries);
    let external_library_section = ExternalLibrarySection {
        items: &external_library_items,
        items_data: &external_library_names_data,
    };

    // external function section
    let (external_function_items, external_function_names_data) =
        ExternalFunctionSection::convert_from_entries(
            &image_common_entry.external_function_entries,
        );
    let external_function_section = ExternalFunctionSection {
        items: &external_function_items,
        names_data: &external_function_names_data,
    };

    // import function section
    let (import_function_items, import_function_data) =
        ImportFunctionSection::convert_from_entries(&image_common_entry.import_function_entries);
    let import_function_section = ImportFunctionSection {
        items: &import_function_items,
        full_names_data: &import_function_data,
    };

    // import data entries
    let (import_data_items, import_data) =
        ImportDataSection::convert_from_entries(&image_common_entry.import_data_entries);
    let import_data_section = ImportDataSection {
        items: &import_data_items,
        full_names_data: &import_data,
    };

    // func name section
    let (function_name_items, function_name_data) =
        FunctionNameSection::convert_from_entries(&image_common_entry.function_name_entries);
    let function_name_section = FunctionNameSection {
        items: &function_name_items,
        full_names_data: &function_name_data,
    };

    // data name section
    let (data_name_items, data_name_data) =
        DataNameSection::convert_from_entries(&image_common_entry.data_name_entries);
    let data_name_section = DataNameSection {
        items: &data_name_items,
        full_names_data: &data_name_data,
    };

    let section_entries: Vec<&dyn SectionEntry> = vec![
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
        &common_property_section,
    ];

    // build object file binary
    let (section_items, sections_data) = ModuleImage::convert_from_entries(&section_entries);
    let module_image = ModuleImage {
        image_type: ImageType::ObjectFile,
        items: &section_items,
        sections_data: &sections_data,
    };

    // save
    module_image.save(writer).unwrap();
    Ok(())
}

#[cfg(test)]
mod tests {
    use anc_isa::OperandDataType;
    use pretty_assertions::assert_eq;

    use anc_image::{
        bytecode_reader::format_bytecode_as_text,
        entry::{LocalVariableEntry, LocalVariableListEntry, TypeEntry},
        module_image::ModuleImage,
    };

    use crate::utils::helper_assemble_single_module;

    fn generate_binary(source: &str) -> Vec<u8> {
        helper_assemble_single_module(source, &[], &[])
    }

    #[test]
    fn test_image_generate_base() {
        // todo: add 'data' statements

        let image_binary = generate_binary(
            r#"
pub fn foo () -> i32 {
    call(add
        imm_i32(0x11)
        imm_i32(0x13)
    )
}

fn add(left:i32, right:i32) -> i32 {
    add_i32(
        local_load_i32_s(left)
        local_load_i32_s(right)
    )
}
        "#,
        );

        let common_module_image = ModuleImage::load(&image_binary).unwrap();

        // check types

        let type_section = common_module_image.get_type_section();
        assert_eq!(
            type_section.get_type_entry(0),
            TypeEntry::new(vec![], vec![])
        );
        assert_eq!(
            type_section.get_type_entry(1),
            TypeEntry::new(vec![], vec![OperandDataType::I32])
        );
        assert_eq!(
            type_section.get_type_entry(2),
            TypeEntry::new(
                vec![OperandDataType::I32, OperandDataType::I32],
                vec![OperandDataType::I32]
            )
        );

        // check local variable list

        let local_variable_section = common_module_image.get_local_variable_section();
        assert_eq!(
            local_variable_section.get_local_variable_list_entry(0),
            LocalVariableListEntry::new(vec![])
        );
        assert_eq!(
            local_variable_section.get_local_variable_list_entry(1),
            LocalVariableListEntry::new(vec![
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_i32()
            ])
        );

        // todo: check data entries

        // check functions

        let function_section = common_module_image.get_function_section();

        let function0 = function_section.get_function_entry(0);
        assert_eq!(function0.type_index, 1);
        assert_eq!(function0.local_variable_list_index, 0);
        assert_eq!(
            format_bytecode_as_text(&function0.code),
            "\
0x0000  40 01 00 00  11 00 00 00    imm_i32           0x00000011
0x0008  40 01 00 00  13 00 00 00    imm_i32           0x00000013
0x0010  00 04 00 00  01 00 00 00    call              idx:1
0x0018  c0 03                       end"
        );

        let function1 = function_section.get_function_entry(1);
        assert_eq!(function1.type_index, 2);
        assert_eq!(function1.local_variable_list_index, 1);
        assert_eq!(
            format_bytecode_as_text(&function1.code),
            "\
0x0000  81 01 00 00  00 00 00 00    local_load_i32_s  rev:0   off:0x00  idx:0
0x0008  81 01 00 00  00 00 01 00    local_load_i32_s  rev:0   off:0x00  idx:1
0x0010  00 03                       add_i32
0x0012  c0 03                       end"
        );

        // check function full name
        let function_name_section = common_module_image
            .get_optional_function_name_section()
            .unwrap();

        assert_eq!(
            function_name_section.get_item_full_name_and_export(0),
            ("mymodule::foo", true)
        );

        assert_eq!(
            function_name_section.get_item_full_name_and_export(1),
            ("mymodule::add", false)
        );

        // todo: check data full name
    }

    #[test]
    fn test_image_generate_with_import_and_external() {
        // TODO
    }
}
