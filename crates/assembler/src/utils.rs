// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use anc_image::{
    index_sections::{
        data_index_section::{DataIndexItem, DataIndexSection},
        function_index_section::{FunctionIndexItem, FunctionIndexSection},
        index_property_section::IndexPropertySection,
    },
    module_image::{ImageType, ModuleImage, RangeItem, SectionEntry},
};
use anc_isa::{DataSectionType, RUNTIME_MAJOR_VERSION, RUNTIME_MINOR_VERSION};
use anc_parser_asm::parser::parse_from_str;

use crate::{
    assembler::{assemble_module_node, create_virtual_dependency_module},
    imggen::generate_object_file,
};

pub fn helper_assemble_single_module(source: &str) -> Vec<u8> {
    let module_node = match parse_from_str(source) {
        Ok(node) => node,
        Err(parser_error) => {
            panic!("{}", parser_error.with_source(source));
        }
    };

    let import_module_entries = vec![create_virtual_dependency_module()];
    let external_library_entries = vec![];

    let image_common_entry = assemble_module_node(
        &module_node,
        "mymodule",
        &import_module_entries,
        &external_library_entries,
    )
    .unwrap();

    let mut buf: Vec<u8> = vec![];
    generate_object_file(&image_common_entry, &mut buf).unwrap();
    buf
}

pub fn helper_make_single_module_app(source: &str) -> Vec<u8> {
    let common_binary = helper_assemble_single_module(source);
    let common_module_image = ModuleImage::load(&common_binary).unwrap();

    // build the following index sections:
    //
    // - index_property_section
    // - (empty) module_list_section
    // - function_index_section
    // - data_index_section
    // - (empty) unified_external_type_section
    // - (empty) unified_external_library_section
    // - (empty) unified_external_function_section
    // - (empty) external_function_index_section

    // build function index
    let function_count = common_module_image.get_function_section().items.len();

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

    let type_section = common_module_image.get_type_section();
    let local_variable_section = common_module_image.get_local_variable_section();
    let function_section = common_module_image.get_function_section();
    let common_property_section = common_module_image.get_common_property_section();

    let read_only_data_section = common_module_image
        .get_optional_read_only_data_section()
        .unwrap_or_default();
    let read_write_data_section = common_module_image
        .get_optional_read_write_data_section()
        .unwrap_or_default();
    let uninit_data_section = common_module_image
        .get_optional_uninit_data_section()
        .unwrap_or_default();
    let function_name_section = common_module_image
        .get_optional_function_name_path_section()
        .unwrap_or_default();
    let data_name_section = common_module_image
        .get_optional_data_name_path_section()
        .unwrap_or_default();

    let section_entries: Vec<&dyn SectionEntry> = vec![
        &type_section,
        &local_variable_section,
        &function_section,
        &read_only_data_section,
        &read_write_data_section,
        &uninit_data_section,
        // &external_library_section,
        // &external_function_section,
        // &import_function_section,
        // &import_data_section,
        &function_name_section,
        &data_name_section,
        &common_property_section,
        &function_index_section,
        &data_index_section,
        &index_property_section,
    ];

    // build application module binary
    let (section_items, sections_data) = ModuleImage::convert_from_entries(&section_entries);
    let module_image = ModuleImage {
        image_type: ImageType::Application,
        items: &section_items,
        sections_data: &sections_data,
    };

    let mut buf: Vec<u8> = vec![];
    module_image.save(&mut buf).unwrap();
    buf
}
