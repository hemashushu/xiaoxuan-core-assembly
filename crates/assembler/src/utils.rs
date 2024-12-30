// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use anc_image::{
    entry::{
        ExternalFunctionIndexEntry, ExternalFunctionIndexListEntry, ExternalLibraryEntry,
        ImportModuleEntry,
    },
    entry_writer::write_object_file,
    index_sections::{
        self,
        data_index_section::{DataIndexItem, DataIndexSection},
        external_function_index_section::ExternalFunctionIndexSection,
        external_function_section::UnifiedExternalFunctionSection,
        external_library_section::UnifiedExternalLibrarySection,
        external_type_section::{self, UnifiedExternalTypeSection},
        function_index_section::{FunctionIndexItem, FunctionIndexSection},
        index_property_section::IndexPropertySection,
    },
    module_image::{ImageType, ModuleImage, RangeItem, SectionEntry},
};
use anc_isa::{DataSectionType, RUNTIME_MAJOR_VERSION, RUNTIME_MINOR_VERSION};
use anc_parser_asm::parser::parse_from_str;

use crate::assembler::assemble_module_node;

pub fn helper_assemble_single_module(
    source_code: &str,
    import_module_entries: &[ImportModuleEntry],
    external_library_entries: &[ExternalLibraryEntry],
) -> Vec<u8> {
    let module_node = match parse_from_str(source_code) {
        Ok(node) => node,
        Err(parser_error) => {
            panic!("{}", parser_error.with_source(source_code));
        }
    };

    let image_common_entry = assemble_module_node(
        &module_node,
        "mymodule",
        import_module_entries,
        external_library_entries,
    )
    .unwrap();

    let mut buf: Vec<u8> = vec![];
    write_object_file(&image_common_entry, true, &mut buf).unwrap();
    buf
}

pub fn helper_make_single_module_app(source_code: &str) -> Vec<u8> {
    helper_make_single_module_app_with_external_library(source_code, &[])
}

pub fn helper_make_single_module_app_with_external_library(
    source_code: &str,
    external_library_entries: &[ExternalLibraryEntry],
) -> Vec<u8> {
    let common_binary = helper_assemble_single_module(source_code, &[], external_library_entries);
    let common_module_image = ModuleImage::read(&common_binary).unwrap();

    // build the following index sections:
    //
    // - index_property_section
    // - module_list_section (empty)
    // - function_index_section
    // - data_index_section
    // - unified_external_type_section (clone from external_type_section)
    // - unified_external_library_section (clone from external_library_section)
    // - unified_external_function_section (clone from external_function_section)
    // - external_function_index_section

    // build function index

    let function_section = common_module_image.get_function_section();
    let function_count = function_section.items.len();

    let function_ranges: Vec<RangeItem> = vec![RangeItem {
        offset: 0,
        count: function_count as u32,
    }];

    let function_index_items: Vec<FunctionIndexItem> = (0..function_count)
        .map(|idx| {
            let idx_u32 = idx as u32;
            FunctionIndexItem::new(0, idx_u32)
        })
        .collect::<Vec<_>>();

    let function_index_section = FunctionIndexSection {
        ranges: &function_ranges,
        items: &function_index_items,
    };

    // build data index

    // the data index is ordered by:
    // 1. imported ro data
    // 2. imported rw data
    // 3. imported uninit data
    // 4. ro data
    // 5. rw data
    // 6. uninit data

    let ro_count = common_module_image
        .get_optional_read_only_data_section()
        .map(|section| section.items.len())
        .unwrap_or(0);

    let rw_count = common_module_image
        .get_optional_read_write_data_section()
        .map(|section| section.items.len())
        .unwrap_or(0);

    let uninit_count = common_module_image
        .get_optional_uninit_data_section()
        .map(|section| section.items.len())
        .unwrap_or(0);

    let data_ranges: Vec<RangeItem> = vec![RangeItem {
        offset: 0,
        count: (ro_count + rw_count + uninit_count) as u32,
    }];

    let mut data_index_items: Vec<DataIndexItem> = Vec::new();

    let ro_iter = (0..ro_count).map(|idx| (idx, DataSectionType::ReadOnly));
    let rw_iter = (0..rw_count).map(|idx| (idx, DataSectionType::ReadWrite));
    let uninit_iter = (0..uninit_count).map(|idx| (idx, DataSectionType::Uninit));

    for (idx, data_section_type) in ro_iter.chain(rw_iter).chain(uninit_iter) {
        data_index_items.push(DataIndexItem::new(0, idx as u32, data_section_type));
    }

    let data_index_section = DataIndexSection {
        ranges: &data_ranges,
        items: &data_index_items,
    };

    let index_property_section = IndexPropertySection {
        runtime_major_version: RUNTIME_MAJOR_VERSION,
        runtime_minor_version: RUNTIME_MINOR_VERSION,
        entry_function_public_index: 0,
    };

    let read_only_data_section = common_module_image
        .get_optional_read_only_data_section()
        .unwrap_or_default();
    let read_write_data_section = common_module_image
        .get_optional_read_write_data_section()
        .unwrap_or_default();
    let uninit_data_section = common_module_image
        .get_optional_uninit_data_section()
        .unwrap_or_default();
    let export_function_section = common_module_image
        .get_optional_export_function_section()
        .unwrap_or_default();
    let export_data_section = common_module_image
        .get_optional_export_data_section()
        .unwrap_or_default();

    // build unified external type/library/function sections

    let type_section = common_module_image.get_type_section();
    let external_type_items = type_section
        .items
        .iter()
        .map(|item| {
            external_type_section::TypeItem::new(
                item.params_count,
                item.results_count,
                item.params_offset,
                item.results_offset,
            )
        })
        .collect::<Vec<_>>();

    let unified_external_type_section = UnifiedExternalTypeSection {
        items: &external_type_items,
        types_data: type_section.types_data,
    };

    let external_library_section = common_module_image
        .get_optional_external_library_section()
        .unwrap_or_default();
    let external_library_items = external_library_section
        .items
        .iter()
        .map(|item| {
            index_sections::external_library_section::ExternalLibraryItem::new(
                item.name_offset,
                item.name_length,
                item.value_offset,
                item.value_length,
                item.external_library_dependent_type,
            )
        })
        .collect::<Vec<_>>();

    let unified_external_library_section = UnifiedExternalLibrarySection {
        items: &external_library_items,
        items_data: external_library_section.items_data,
    };

    let external_function_section = common_module_image
        .get_optional_external_function_section()
        .unwrap_or_default();
    let external_function_items = external_function_section
        .items
        .iter()
        .map(|item| {
            index_sections::external_function_section::ExternalFunctionItem::new(
                item.name_offset,
                item.name_length,
                item.external_library_index,
                item.type_index,
            )
        })
        .collect::<Vec<_>>();

    let unified_external_function_section = UnifiedExternalFunctionSection {
        items: &external_function_items,
        names_data: external_function_section.names_data,
    };

    // build external function index

    let external_function_index_entries = (0..external_function_items.len())
        .map(ExternalFunctionIndexEntry::new)
        .collect::<Vec<_>>();

    let external_function_index_list_entries = vec![ExternalFunctionIndexListEntry::new(
        external_function_index_entries,
    )];
    let (external_index_ranges, external_index_items) =
        ExternalFunctionIndexSection::convert_from_entries(&external_function_index_list_entries);
    let external_function_index_section = ExternalFunctionIndexSection {
        ranges: &external_index_ranges,
        items: &external_index_items,
    };

    // other sections

    let local_variable_section = common_module_image.get_local_variable_section();
    let common_property_section = common_module_image.get_common_property_section();

    let section_entries: Vec<&dyn SectionEntry> = vec![
        // common sections
        &type_section,
        &local_variable_section,
        &function_section,
        &read_only_data_section,
        &read_write_data_section,
        &uninit_data_section,
        &external_library_section,
        &external_function_section,
        // &import_function_section,
        // &import_data_section,
        &export_function_section,
        &export_data_section,
        &common_property_section,
        // index sections
        &function_index_section,
        &data_index_section,
        &unified_external_type_section,
        &unified_external_library_section,
        &unified_external_function_section,
        &external_function_index_section,
        &index_property_section,
    ];

    // build application module binary
    let (section_items, sections_data) =
        ModuleImage::convert_from_section_entries(&section_entries);
    let module_image = ModuleImage {
        image_type: ImageType::Application,
        items: &section_items,
        sections_data: &sections_data,
    };

    let mut buf: Vec<u8> = vec![];
    module_image.write(&mut buf).unwrap();
    buf
}
