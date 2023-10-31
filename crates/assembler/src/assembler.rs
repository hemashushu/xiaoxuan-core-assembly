// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_assembly_parser::ast::ModuleNode;
use ancvm_binary::module_image::{
    data_index_section::DataIndexItem, func_index_section::FuncIndexItem,
    func_name_section::FuncNameEntry, func_section::FuncEntry,
    local_variable_section::LocalListEntry, type_section::TypeEntry,
};

use crate::AssembleError;

pub struct ModuleEntry {
    pub name: String,
    pub runtime_version_major: u16,
    pub runtime_version_minor: u16,

    // pub shared_packages: Vec<String>,
    pub type_entries: Vec<TypeEntry>,
    pub local_list_entries: Vec<LocalListEntry>,
    pub func_entries: Vec<FuncEntry>,

    pub func_name_entries: Vec<FuncNameEntry>,
}

pub struct ModuleIndexEntry {
    pub func_index_items: Vec<FuncIndexItem>,
    pub data_index_items: Vec<DataIndexItem>,
}

pub fn assemble_module_node(module_node: &ModuleNode) -> Result<ModuleEntry, AssembleError> {
    let name = module_node.name.clone();
    let runtime_version_major = module_node.runtime_version_major;
    let runtime_version_minor = module_node.runtime_version_minor;

    let module_entry = ModuleEntry {
        name,
        runtime_version_major,
        runtime_version_minor,
        type_entries: todo!(),
        local_list_entries: todo!(),
        func_entries: todo!(),
        func_name_entries: todo!(),
    };

    Ok(module_entry)
}

// fn parse_from_str(s: &str) -> Result<ModuleNode, CompileError> {
//     init_instruction_kind_table();
//
//     let mut chars = s.chars();
//     let mut char_iter = PeekableIterator::new(&mut chars, 2);
//     let mut tokens = lex(&mut char_iter)?.into_iter();
//     let mut token_iter = PeekableIterator::new(&mut tokens, 2);
//     parse(&mut token_iter)
// }

/*
fn test_multithread_thread_id() {
    let code0 = BytecodeWriter::new()
        .write_opcode_i32(Opcode::envcall, EnvCallCode::thread_id as u32)
        .write_opcode(Opcode::end)
        .to_bytes();

    let binary0 = build_module_binary_with_single_function(
        vec![DataType::I32], // params
        vec![DataType::I32], // results
        vec![],              // local vars
        code0,
    );

    let program_source0 = InMemoryProgramSource::new(vec![binary0]);
    let multithread_program0 = MultithreadProgram::new(program_source0);
    let child_thread_id0 = create_thread(&multithread_program0, 0, 0, vec![]);

    const FIRST_CHILD_THREAD_ID: u32 = 1;

    CHILD_THREADS.with(|child_threads_cell| {
        let mut child_threads = child_threads_cell.borrow_mut();
        let opt_child_thread = child_threads.remove(&child_thread_id0);
        let child_thread = opt_child_thread.unwrap();

        let result0 = child_thread.join_handle.join().unwrap();

        assert_eq!(
            result0.unwrap(),
            vec![ForeignValue::UInt32(FIRST_CHILD_THREAD_ID)]
        );
    });
}

fn test_module_common_sections_save_and_load() {
    // build TypeSection instance

    let mut type_entries: Vec<TypeEntry> = Vec::new();
    let type0 = vec![DataType::I32, DataType::I64];
    let type1 = vec![DataType::F32];
    let type2 = vec![];
    let type3 = vec![DataType::F64];

    type_entries.push(TypeEntry {
        params: type0.clone(),
        results: type1.clone(),
    });

    type_entries.push(TypeEntry {
        params: type2.clone(),
        results: type3.clone(),
    });

    let (type_items, types_data) = TypeSection::convert_from_entries(&type_entries);
    let type_section = TypeSection {
        items: &type_items,
        types_data: &types_data,
    };

    // build FuncSection instance

    let mut func_entries: Vec<FuncEntry> = Vec::new();
    let code0: Vec<u8> = vec![1u8, 2, 3, 5, 7]; // arbitrary code, not actual byte codes
    let code1: Vec<u8> = vec![11u8, 13, 17, 19, 23, 29]; // arbitrary code, not actual byte codes

    func_entries.push(FuncEntry {
        type_index: 2,
        local_list_index: 3,
        code: code0.clone(),
    });
    func_entries.push(FuncEntry {
        type_index: 5,
        local_list_index: 7,
        code: code1.clone(),
    });

    let (func_items, codes_data) = FuncSection::convert_from_entries(&func_entries);
    let func_section = FuncSection {
        items: &func_items,
        codes_data: &codes_data,
    };

    // build LocalVariableSection instance

    // note:
    // the local variable list should include the function arguments, the
    // data below does not follow this rule, but it's ok in the unit test scenario.
    let local_var_list_entries = vec![
        LocalListEntry::new(vec![
            LocalVariableEntry::from_i32(),
            LocalVariableEntry::from_i64(),
        ]),
        LocalListEntry::new(vec![LocalVariableEntry::from_bytes(12, 4)]),
    ];

    let (local_var_lists, local_var_list_data) =
        LocalVariableSection::convert_from_entries(&local_var_list_entries);
    let local_var_section = LocalVariableSection {
        lists: &local_var_lists,
        list_data: &local_var_list_data,
    };

    // build ModuleImage instance
    let section_entries: Vec<&dyn SectionEntry> =
        vec![&type_section, &func_section, &local_var_section];
    let (section_items, sections_data) = ModuleImage::convert_from_entries(&section_entries);
    let module_image = ModuleImage {
        name: "main",
        items: &section_items,
        sections_data: &sections_data,
    };

    // save
    let mut image_data: Vec<u8> = Vec::new();
    module_image.save(&mut image_data).unwrap();

    assert_eq!(&image_data[0..8], IMAGE_MAGIC_NUMBER);
    assert_eq!(&image_data[8..10], &[0, 0]); // image minor version number, little endian
    assert_eq!(&image_data[10..12], &[1, 0]); // image major version number, little endian
    assert_eq!(&image_data[12..14], &[0, 0]); // runtime minor version number, little endian
    assert_eq!(&image_data[14..16], &[1, 0]); // runtime major version number, little endian

    // name
    assert_eq!(&image_data[16..18], &[4, 0]); // name length
    assert_eq!(&image_data[18..20], &[0, 0]); // padding
    assert_eq!(&image_data[20..24], &b"main".to_vec()); // name

    // header length = 276 bytes

    // section count
    assert_eq!(&image_data[276..280], &[3, 0, 0, 0]); // item count
    assert_eq!(&image_data[280..284], &[0, 0, 0, 0]); // padding

    let remains = &image_data[284..];

    // section table length = 12 (the record length) * 3
    let (section_table_data, remains) = remains.split_at(36);

    assert_eq!(
        section_table_data,
        &[
            0x10u8, 0, 0, 0, // section id, type section
            0, 0, 0, 0, // offset 0
            36, 0, 0, 0, // length 0
            //
            0x11u8, 0, 0, 0, // section id, func section
            36, 0, 0, 0, // offset 1
            52, 0, 0, 0, // length 1
            //
            0x12u8, 0, 0, 0, // section id, local variable section
            88, 0, 0, 0, // offset 1
            68, 0, 0, 0, // length 1
        ]
    );

    let (type_section_data, remains) = remains.split_at(36);
    assert_eq!(
        type_section_data,
        &[
            2u8, 0, 0, 0, // item count
            0, 0, 0, 0, // padding
            //
            2, 0, // param len 0
            1, 0, // result len 0
            0, 0, 0, 0, // param offset 0
            2, 0, 0, 0, // result offset 0
            //
            0, 0, // param len 1
            1, 0, // result len 1
            3, 0, 0, 0, // param offset 1
            3, 0, 0, 0, // result offset 1
            //
            0, // I32
            1, // I64
            2, // F32
            3, // F64
        ]
    );

    let (func_section_data, remains) = remains.split_at(52);
    assert_eq!(
        func_section_data,
        &[
            2, 0, 0, 0, // item count
            0, 0, 0, 0, // padding
            //
            0, 0, 0, 0, // code offset 0
            5, 0, 0, 0, // code len 0
            2, 0, 0, 0, // func type index 0
            3, 0, 0, 0, // local variable index 0
            //
            5, 0, 0, 0, // code offset 1
            6, 0, 0, 0, // code len 1
            5, 0, 0, 0, // func type index 1
            7, 0, 0, 0, // local variable index 1
            //
            1, 2, 3, 5, 7, // code 0
            11, 13, 17, 19, 23, 29, // code 1
            //
            0, // padding
        ]
    );

    assert_eq!(
        remains,
        &[
            // header
            2, 0, 0, 0, // item count
            0, 0, 0, 0, // 4 bytes padding
            // table
            0, 0, 0, 0, // offset
            2, 0, 0, 0, // count
            16, 0, 0, 0, // alloc bytes
            //
            24, 0, 0, 0, // offset (2 items * 12 bytes/item)
            1, 0, 0, 0, // count
            16, 0, 0, 0, // alloc bytes
            //
            // data
            //
            // list 0
            0, 0, 0, 0, // var offset (i32)
            4, 0, 0, 0, // var len
            0, // data type
            0, // padding
            4, 0, // align
            //
            8, 0, 0, 0, // var offset (i64)
            8, 0, 0, 0, // var len
            1, // data type
            0, // padding
            8, 0, // align
            //
            // list 1
            0, 0, 0, 0, // var offset
            12, 0, 0, 0, // var len
            4, // data type
            0, // padding
            4, 0, // align
        ]
    );

    // load
    let module_image_restore = ModuleImage::load(&image_data).unwrap();
    assert_eq!(module_image_restore.items.len(), 3);

    // check type

    let type_section_restore = module_image_restore.get_type_section();
    assert_eq!(type_section_restore.items.len(), 2);

    assert_eq!(
        type_section_restore.get_item_params_and_results(0),
        (type0.as_ref(), type1.as_ref(),)
    );

    assert_eq!(
        type_section_restore.get_item_params_and_results(1),
        (type2.as_ref(), type3.as_ref(),)
    );

    // check func

    let func_section_restore = module_image_restore.get_func_section();
    assert_eq!(func_section_restore.items.len(), 2);

    assert_eq!(
        func_section_restore.get_item_type_index_and_local_variable_index_and_code(0),
        (2, 3, code0.as_ref(),)
    );

    assert_eq!(
        func_section_restore.get_item_type_index_and_local_variable_index_and_code(1),
        (5, 7, code1.as_ref(),)
    );

    // check local vars

    let local_var_section_restore = module_image_restore.get_local_variable_section();
    assert_eq!(local_var_section_restore.lists.len(), 2);

    assert_eq!(
        local_var_section_restore.get_local_list(0),
        &[
            LocalVariableItem::new(0, 4, MemoryDataType::I32, 4),
            LocalVariableItem::new(8, 8, MemoryDataType::I64, 8),
        ]
    );

    assert_eq!(
        local_var_section_restore.get_local_list(1),
        &[LocalVariableItem::new(0, 12, MemoryDataType::BYTES, 4),]
    );
}
*/
