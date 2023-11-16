// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

mod utils;

use ancvm_binary::{
    bytecode_reader::print_bytecode_as_text,
    module_image::{
        local_variable_section::{LocalListEntry, LocalVariableEntry},
        type_section::TypeEntry,
    },
};
use ancvm_program::program_source::ProgramSource;
use ancvm_runtime::{
    in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
};
use ancvm_types::{DataType, ForeignValue, MemoryDataType};

use crate::utils::assemble_single_module;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_control_flow_block_equ_structure_for() {
    // fn () -> (i32, i32, i32, i32)
    //     (i32_imm 11)
    //     (i32_imm 13)
    //     (block 1 0) () -> ()
    //         (i32_imm 17)
    //         (i32_imm 19)
    //     end
    //     (i32_imm 23)
    //     (i32_imm 29)
    // end
    //
    // expect (11, 13, 23, 29)

    let module_binaries = assemble_single_module(
        r#"
        (module $app
            (runtime_version "1.0")
            (func $main
                (results
                    i32 i32 i32 i32)
                (code
                    (i32.imm 11)
                    (i32.imm 13)
                    (for
                        (do
                            (i32.imm 17)
                            (i32.imm 19)
                        )
                    )
                    (i32.imm 23)
                    (i32.imm 29)
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

    assert_eq!(
        bytecode_text,
        "\
0x0000  80 01 00 00  0b 00 00 00    i32.imm           0x0000000b
0x0008  80 01 00 00  0d 00 00 00    i32.imm           0x0000000d
0x0010  01 0a 00 00  01 00 00 00    block             type:1   local:0
        00 00 00 00
0x001c  80 01 00 00  11 00 00 00    i32.imm           0x00000011
0x0024  80 01 00 00  13 00 00 00    i32.imm           0x00000013
0x002c  00 0a                       end
0x002e  00 0c                       nop
0x0030  80 01 00 00  17 00 00 00    i32.imm           0x00000017
0x0038  80 01 00 00  1d 00 00 00    i32.imm           0x0000001d
0x0040  00 0a                       end"
    );

    assert_eq!(func_entry.type_index, 0);

    let func_type_entry = program0.module_images[0]
        .get_type_section()
        .get_type_entry(0);

    assert_eq!(
        func_type_entry,
        TypeEntry {
            params: vec![],
            results: vec![DataType::I32, DataType::I32, DataType::I32, DataType::I32]
        }
    );

    assert_eq!(func_entry.local_list_index, 0);

    let func_local_list_entry = program0.module_images[0]
        .get_local_variable_section()
        .get_local_list_entry(0);

    assert_eq!(
        func_local_list_entry,
        LocalListEntry {
            variable_entries: vec![]
        }
    );

    let block_type_entry = program0.module_images[0]
        .get_type_section()
        .get_type_entry(1);

    assert_eq!(
        block_type_entry,
        TypeEntry {
            params: vec![],
            results: vec![]
        }
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::UInt32(11),
            ForeignValue::UInt32(13),
            ForeignValue::UInt32(23),
            ForeignValue::UInt32(29),
        ]
    );
}

#[test]
fn test_assemble_control_flow_block_with_args_and_results_equ_structure_for() {
    // fn () -> (i32, i32, i32)
    //     (i32_imm 11)
    //     (i32_imm 13)
    //     (block 1 1) (i32) -> (i32)
    //         (local_load 0)
    //         (i32_imm 17)
    //         (i32_add)
    //     end
    //     (i32_imm 19)
    // end
    //
    // expect (11, 30, 19)

    let module_binaries = assemble_single_module(
        r#"
        (module $app
            (runtime_version "1.0")
            (func $main
                (results
                    i32 i32 i32)
                (code
                    (i32.imm 11)
                    (i32.imm 13)
                    (for (param $a i32) (result i32)
                        (i32.add
                            (local.load32_i32 $a)
                            (i32.imm 17)
                        )
                    )
                    (i32.imm 19)
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
        bytecode_text,
        "\
0x0000  80 01 00 00  0b 00 00 00    i32.imm           0x0000000b
0x0008  80 01 00 00  0d 00 00 00    i32.imm           0x0000000d
0x0010  01 0a 00 00  01 00 00 00    block             type:1   local:1
        01 00 00 00
0x001c  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x0024  80 01 00 00  11 00 00 00    i32.imm           0x00000011
0x002c  00 07                       i32.add
0x002e  00 0a                       end
0x0030  80 01 00 00  13 00 00 00    i32.imm           0x00000013
0x0038  00 0a                       end"
    );

    let block_type_entry = program0.module_images[0]
        .get_type_section()
        .get_type_entry(1);

    assert_eq!(
        block_type_entry,
        TypeEntry {
            params: vec![DataType::I32],
            results: vec![DataType::I32]
        }
    );

    let block_local_list_entry = program0.module_images[0]
        .get_local_variable_section()
        .get_local_list_entry(1);

    assert_eq!(
        block_local_list_entry,
        LocalListEntry {
            variable_entries: vec![LocalVariableEntry {
                memory_data_type: MemoryDataType::I32,
                length: 4,
                align: 4
            }]
        }
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::UInt32(11),
            ForeignValue::UInt32(30),
            ForeignValue::UInt32(19),
        ]
    );
}

#[test]
fn test_assemble_control_flow_block_with_local_vars_equ_structure_for() {
    // func (a/0:i32, b/1:i32) -> (i32,i32,i32,i32,i32,i32,i32,i32)
    //     (local c/2:i32, d/3:i32)
    //     ;; c=a+1                     ;; 20
    //     ;; d=b+1                     ;; 12
    //     (block 1 1) () -> (i32, i32, i32,i32)
    //         (local p/0:i32, q/1:i32)
    //         ;; a=a-1                 ;; 18
    //         ;; b=b-1                 ;; 10
    //         ;; p=c+d                 ;; 32
    //         ;; q=c-d                 ;; 8
    //         ;; load c
    //         ;; load d
    //         (block 2 1) (x/0:i32, y/1:i32) -> (i32,i32)
    //             ;; d=d+1             ;; 13
    //             ;; q=q-1             ;; 7
    //             ;; x+q               ;; 27 (ret 0)
    //             ;; y+p               ;; 44 (ret 1)
    //         end
    //         ;; load p (ret 2)
    //         ;; load q (ret 3)
    //     end
    //     ;; load a (ret 4)
    //     ;; load b (ret 5)
    //     ;; load c (ret 6)
    //     ;; load d (ret 7)
    // end
    //
    // expect (19, 11) -> (27, 44, 32, 7, 18, 10, 20, 13)

    let module_binaries = assemble_single_module(
        r#"
        (module $app
            (runtime_version "1.0")
            (func $main
                (param $a i32)
                (param $b i32)
                (results
                    i32 i32 i32 i32
                    i32 i32 i32 i32)
                (local $c i32)
                (local $d i32)
                (code
                    (local.store32 $c
                        (i32.inc 1 (local.load32_i32 $a)))
                    (local.store32 $d
                        (i32.inc 1 (local.load32_i32 $b)))

                    (for
                        (results i32 i32 i32 i32)
                        (local $p i32)
                        (local $q i32)
                        (do
                            (local.store32 $a
                                (i32.dec 1 (local.load32_i32 $a)))
                            (local.store32 $b
                                (i32.dec 1 (local.load32_i32 $b)))
                            (local.store32 $p
                                (i32.add
                                    (local.load32_i32 $c)
                                    (local.load32_i32 $d)
                                )
                            )
                            (local.store32 $q
                                (i32.sub
                                    (local.load32_i32 $c)
                                    (local.load32_i32 $d)
                                )
                            )
                            (local.load32_i32 $c)
                            (local.load32_i32 $d)

                            (for
                                (param $x i32)
                                (param $y i32)
                                (results i32 i32)
                                (do
                                    (local.store32 $d
                                        (i32.inc 1 (local.load32_i32 $d)))
                                    (local.store32 $q
                                        (i32.dec 1 (local.load32_i32 $q)))
                                    (i32.add
                                        (local.load32_i32 $x)
                                        (local.load32_i32 $q)
                                    )
                                    (i32.add
                                        (local.load32_i32 $y)
                                        (local.load32_i32 $p)
                                    )
                                )
                            )

                            (local.load32_i32 $p)
                            (local.load32_i32 $q)
                        )
                    )

                    (local.load32_i32 $a)
                    (local.load32_i32 $b)
                    (local.load32_i32 $c)
                    (local.load32_i32 $d)
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

    assert_eq!(
        bytecode_text,
        "\
0x0000  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x0008  07 07 01 00                 i32.inc           1
0x000c  09 02 00 00  00 00 02 00    local.store32     rev:0   off:0x00  idx:2
0x0014  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x001c  07 07 01 00                 i32.inc           1
0x0020  09 02 00 00  00 00 03 00    local.store32     rev:0   off:0x00  idx:3
0x0028  01 0a 00 00  01 00 00 00    block             type:1   local:1
        01 00 00 00
0x0034  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x003c  08 07 01 00                 i32.dec           1
0x0040  09 02 01 00  00 00 00 00    local.store32     rev:1   off:0x00  idx:0
0x0048  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
0x0050  08 07 01 00                 i32.dec           1
0x0054  09 02 01 00  00 00 01 00    local.store32     rev:1   off:0x00  idx:1
0x005c  02 02 01 00  00 00 02 00    local.load32_i32  rev:1   off:0x00  idx:2
0x0064  02 02 01 00  00 00 03 00    local.load32_i32  rev:1   off:0x00  idx:3
0x006c  00 07                       i32.add
0x006e  09 02 00 00  00 00 00 00    local.store32     rev:0   off:0x00  idx:0
0x0076  02 02 01 00  00 00 02 00    local.load32_i32  rev:1   off:0x00  idx:2
0x007e  02 02 01 00  00 00 03 00    local.load32_i32  rev:1   off:0x00  idx:3
0x0086  01 07                       i32.sub
0x0088  09 02 00 00  00 00 01 00    local.store32     rev:0   off:0x00  idx:1
0x0090  02 02 01 00  00 00 02 00    local.load32_i32  rev:1   off:0x00  idx:2
0x0098  02 02 01 00  00 00 03 00    local.load32_i32  rev:1   off:0x00  idx:3
0x00a0  01 0a 00 00  02 00 00 00    block             type:2   local:1
        01 00 00 00
0x00ac  02 02 02 00  00 00 03 00    local.load32_i32  rev:2   off:0x00  idx:3
0x00b4  07 07 01 00                 i32.inc           1
0x00b8  09 02 02 00  00 00 03 00    local.store32     rev:2   off:0x00  idx:3
0x00c0  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
0x00c8  08 07 01 00                 i32.dec           1
0x00cc  09 02 01 00  00 00 01 00    local.store32     rev:1   off:0x00  idx:1
0x00d4  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x00dc  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
0x00e4  00 07                       i32.add
0x00e6  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x00ee  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x00f6  00 07                       i32.add
0x00f8  00 0a                       end
0x00fa  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x0102  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x010a  00 0a                       end
0x010c  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x0114  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x011c  02 02 00 00  00 00 02 00    local.load32_i32  rev:0   off:0x00  idx:2
0x0124  02 02 00 00  00 00 03 00    local.load32_i32  rev:0   off:0x00  idx:3
0x012c  00 0a                       end"
    );

    assert_eq!(func_entry.type_index, 0);

    let func_type_entry = program0.module_images[0]
        .get_type_section()
        .get_type_entry(0);

    assert_eq!(
        func_type_entry,
        TypeEntry {
            params: vec![DataType::I32, DataType::I32],
            results: vec![
                DataType::I32,
                DataType::I32,
                DataType::I32,
                DataType::I32,
                DataType::I32,
                DataType::I32,
                DataType::I32,
                DataType::I32
            ]
        }
    );

    assert_eq!(func_entry.local_list_index, 0);

    let func_local_list_entry = program0.module_images[0]
        .get_local_variable_section()
        .get_local_list_entry(0);

    assert_eq!(
        func_local_list_entry,
        LocalListEntry {
            variable_entries: vec![
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_i32(),
            ]
        }
    );

    let block_0_type_entry = program0.module_images[0]
        .get_type_section()
        .get_type_entry(1);

    assert_eq!(
        block_0_type_entry,
        TypeEntry {
            params: vec![],
            results: vec![DataType::I32, DataType::I32, DataType::I32, DataType::I32]
        }
    );

    let block_0_local_list_entry = program0.module_images[0]
        .get_local_variable_section()
        .get_local_list_entry(1);

    assert_eq!(
        block_0_local_list_entry,
        LocalListEntry {
            variable_entries: vec![
                LocalVariableEntry::from_i32(),
                LocalVariableEntry::from_i32(),
            ]
        }
    );

    let block_1_type_entry = program0.module_images[0]
        .get_type_section()
        .get_type_entry(2);

    assert_eq!(
        block_1_type_entry,
        TypeEntry {
            params: vec![DataType::I32, DataType::I32],
            results: vec![DataType::I32, DataType::I32,]
        }
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::UInt32(19), ForeignValue::UInt32(11)],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::UInt32(27),
            ForeignValue::UInt32(44),
            ForeignValue::UInt32(32),
            ForeignValue::UInt32(7),
            ForeignValue::UInt32(18),
            ForeignValue::UInt32(10),
            ForeignValue::UInt32(20),
            ForeignValue::UInt32(13),
        ]
    );
}

#[test]
fn test_assemble_control_flow_break_function_equ_statement_return() {
    // fn () -> (i32, i32)
    //     (i32_imm 11)
    //     (i32_imm 13)
    //     (break 0)
    //     (i32_imm 17)
    //     (i32_imm 19)
    // end
    //
    // expect (11, 13)

    let module_binaries = assemble_single_module(
        r#"
        (module $app
            (runtime_version "1.0")
            (func $main
                (results
                    i32 i32)
                (code
                    (return
                        (i32.imm 11)
                        (i32.imm 13)
                    )
                    (i32.imm 23)
                    (i32.imm 29)
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
        bytecode_text,
        "\
0x0000  80 01 00 00  0b 00 00 00    i32.imm           0x0000000b
0x0008  80 01 00 00  0d 00 00 00    i32.imm           0x0000000d
0x0010  02 0a 00 00  00 00 00 00    break             rev:0   off:0x00
0x0018  80 01 00 00  17 00 00 00    i32.imm           0x00000017
0x0020  80 01 00 00  1d 00 00 00    i32.imm           0x0000001d
0x0028  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();
    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![ForeignValue::UInt32(11), ForeignValue::UInt32(13),]
    );
}

#[test]
fn test_assemble_control_flow_break_block_equ_statement_break() {
    // fn () -> (i32, i32, i32, i32)
    //     (i32_imm 11)
    //     (i32_imm 13)
    //     (block 1 1) () -> (i32, i32)
    //         (i32_imm 17)
    //         (i32_imm 19)
    //         (break 0)
    //         (i32_imm 23)
    //         (i32_imm 29)
    //     end
    //     (i32_imm 31)
    //     (i32_imm 37)
    // end
    //
    // expect (17, 19, 31, 37)

    let module_binaries = assemble_single_module(
        r#"
        (module $app
            (runtime_version "1.0")
            (func $main
                (results
                    i32 i32 i32 i32)
                (code
                    (i32.imm 11)
                    (i32.imm 13)
                    (for
                        (results i32 i32)
                        (do
                            (break
                                (i32.imm 17)
                                (i32.imm 19)
                            )
                            (i32.imm 23)
                            (i32.imm 29)
                        )
                    )
                    (i32.imm 31)
                    (i32.imm 37)
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
        bytecode_text,
        "\
0x0000  80 01 00 00  0b 00 00 00    i32.imm           0x0000000b
0x0008  80 01 00 00  0d 00 00 00    i32.imm           0x0000000d
0x0010  01 0a 00 00  01 00 00 00    block             type:1   local:0
        00 00 00 00
0x001c  80 01 00 00  11 00 00 00    i32.imm           0x00000011
0x0024  80 01 00 00  13 00 00 00    i32.imm           0x00000013
0x002c  02 0a 00 00  1a 00 00 00    break             rev:0   off:0x1a
0x0034  80 01 00 00  17 00 00 00    i32.imm           0x00000017
0x003c  80 01 00 00  1d 00 00 00    i32.imm           0x0000001d
0x0044  00 0a                       end
0x0046  00 0c                       nop
0x0048  80 01 00 00  1f 00 00 00    i32.imm           0x0000001f
0x0050  80 01 00 00  25 00 00 00    i32.imm           0x00000025
0x0058  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();
    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::UInt32(17),
            ForeignValue::UInt32(19),
            ForeignValue::UInt32(31),
            ForeignValue::UInt32(37),
        ]
    );
}

#[test]
fn test_assemble_control_flow_break_block_to_function_equ_statement_return() {
    // fn () -> (i32, i32)
    //     (i32_imm 11)
    //     (i32_imm 13)
    //     (block 1 1) () -> (i32 i32)
    //         (i32_imm 17)
    //         (i32_imm 19)
    //         (break 1)
    //         (i32_imm 23)
    //         (i32_imm 29)
    //     end
    //     (i32_imm 31)
    //     (i32_imm 37)
    // end
    //
    // expect (17, 19)

    let module_binaries = assemble_single_module(
        r#"
        (module $app
            (runtime_version "1.0")
            (func $main
                (results
                    i32 i32)
                (code
                    (i32.imm 11)
                    (i32.imm 13)
                    (for
                        (results i32 i32)
                        (do
                            (return
                                (i32.imm 17)
                                (i32.imm 19)
                            )
                            (i32.imm 23)
                            (i32.imm 29)
                        )
                    )
                    (i32.imm 31)
                    (i32.imm 37)
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
        bytecode_text,
        "\
0x0000  80 01 00 00  0b 00 00 00    i32.imm           0x0000000b
0x0008  80 01 00 00  0d 00 00 00    i32.imm           0x0000000d
0x0010  01 0a 00 00  00 00 00 00    block             type:0   local:0
        00 00 00 00
0x001c  80 01 00 00  11 00 00 00    i32.imm           0x00000011
0x0024  80 01 00 00  13 00 00 00    i32.imm           0x00000013
0x002c  02 0a 01 00  00 00 00 00    break             rev:1   off:0x00
0x0034  80 01 00 00  17 00 00 00    i32.imm           0x00000017
0x003c  80 01 00 00  1d 00 00 00    i32.imm           0x0000001d
0x0044  00 0a                       end
0x0046  00 0c                       nop
0x0048  80 01 00 00  1f 00 00 00    i32.imm           0x0000001f
0x0050  80 01 00 00  25 00 00 00    i32.imm           0x00000025
0x0058  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();
    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![ForeignValue::UInt32(17), ForeignValue::UInt32(19),]
    );
}

#[test]
fn test_assemble_control_flow_structure_when() {
    // func $max (i32, i32) -> (i32)
    //     (local $ret/2 i32)
    //
    //     (local_load32 0 0)
    //     (local_store32 0 2)
    //
    //     (local_load32 0 0)
    //     (local_load32 0 1)
    //     i32_lt_u
    //     (block_nez local_idx:1) ()->()
    //          (local_load32 1 1)
    //          (local_store32 1 2)
    //     end
    //     (local_load32 0 2)
    // end
    //
    // assert (11, 13) -> (13)
    // assert (19, 17) -> (19)

    let module_binaries = assemble_single_module(
        r#"
        (module $app
            (runtime_version "1.0")
            (func $max
                (param $a i32)
                (param $b i32)
                (results i32)
                (local $ret i32)
                (code
                    (local.store32 $ret
                        (local.load32_i32 $a))
                    (when
                        (i32.lt_u
                            (local.load32_i32 $a)
                            (local.load32_i32 $b)
                        )
                        (local.store32 $ret
                            (local.load32_i32 $b))
                    )
                    (local.load32_i32 $ret)
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
        bytecode_text,
        "\
0x0000  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x0008  09 02 00 00  00 00 02 00    local.store32     rev:0   off:0x00  idx:2
0x0010  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x0018  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x0020  05 06                       i32.lt_u
0x0022  00 0c                       nop
0x0024  04 0a 00 00  01 00 00 00    block_nez         local:1   off:0x1e
        1e 00 00 00
0x0030  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
0x0038  09 02 01 00  00 00 02 00    local.store32     rev:1   off:0x00  idx:2
0x0040  00 0a                       end
0x0042  02 02 00 00  00 00 02 00    local.load32_i32  rev:0   off:0x00  idx:2
0x004a  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::UInt32(11), ForeignValue::UInt32(13)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(13)]);

    let result1 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::UInt32(19), ForeignValue::UInt32(17)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::UInt32(19)]);
}

#[test]
fn test_assemble_control_flow_break_block_crossing_equ_statement_break() {
    // cross block breaking
    //
    // fn (i32) -> (i32 i32 i32 i32)
    //     (i32_imm 11)
    //     (i32_imm 13)
    //     (block 1 1) () -> (i32 i32)
    //         (i32_imm 17)
    //         (i32_imm 19)
    //         (local_load32_i32 1 0)  ;; true
    //         (block_nez 2 2) () -> (i32 i32)
    //             (i32_imm 23)
    //             (i32_imm 29)
    //             (break 1)
    //             (i32_imm 31)
    //             (i32_imm 37)
    //         end
    //         (i32_imm 41)
    //         (i32_imm 43)
    //     end
    //     (i32_imm 51)
    //     (i32_imm 53)
    // end
    //
    // expect (1) -> (23, 29, 51, 53)
    // expect (0) -> (41, 43, 51, 53)

    let module_binaries = assemble_single_module(
        r#"
        (module $app
            (runtime_version "1.0")
            (func $main
                (param $a i32)
                (results
                    i32 i32 i32 i32)
                (code
                    (i32.imm 11)
                    (i32.imm 13)
                    (for
                        (results i32 i32)
                        (do
                            (i32.imm 17)
                            (i32.imm 19)
                            (when
                                (local.load32_i32 $a)
                                (do
                                    (break
                                        (i32.imm 23)
                                        (i32.imm 29)
                                    )
                                    (i32.imm 31)
                                    (i32.imm 37)
                                )
                            )
                            (i32.imm 41)
                            (i32.imm 43)
                        )
                    )
                    (i32.imm 51)
                    (i32.imm 53)
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
        bytecode_text,
        "\
0x0000  80 01 00 00  0b 00 00 00    i32.imm           0x0000000b
0x0008  80 01 00 00  0d 00 00 00    i32.imm           0x0000000d
0x0010  01 0a 00 00  01 00 00 00    block             type:1   local:1
        01 00 00 00
0x001c  80 01 00 00  11 00 00 00    i32.imm           0x00000011
0x0024  80 01 00 00  13 00 00 00    i32.imm           0x00000013
0x002c  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x0034  04 0a 00 00  01 00 00 00    block_nez         local:1   off:0x36
        36 00 00 00
0x0040  80 01 00 00  17 00 00 00    i32.imm           0x00000017
0x0048  80 01 00 00  1d 00 00 00    i32.imm           0x0000001d
0x0050  02 0a 01 00  2e 00 00 00    break             rev:1   off:0x2e
0x0058  80 01 00 00  1f 00 00 00    i32.imm           0x0000001f
0x0060  80 01 00 00  25 00 00 00    i32.imm           0x00000025
0x0068  00 0a                       end
0x006a  00 0c                       nop
0x006c  80 01 00 00  29 00 00 00    i32.imm           0x00000029
0x0074  80 01 00 00  2b 00 00 00    i32.imm           0x0000002b
0x007c  00 0a                       end
0x007e  00 0c                       nop
0x0080  80 01 00 00  33 00 00 00    i32.imm           0x00000033
0x0088  80 01 00 00  35 00 00 00    i32.imm           0x00000035
0x0090  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(1)]);
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::UInt32(23),
            ForeignValue::UInt32(29),
            ForeignValue::UInt32(51),
            ForeignValue::UInt32(53),
        ]
    );

    let result0 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(0)]);
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::UInt32(41),
            ForeignValue::UInt32(43),
            ForeignValue::UInt32(51),
            ForeignValue::UInt32(53),
        ]
    );
}
