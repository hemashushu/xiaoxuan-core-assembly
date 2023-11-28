// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_assembler::utils::helper_generate_module_image_binaries_from_single_module_assembly;
use ancvm_binary::{
    bytecode_reader::print_bytecode_as_text,
    module_image::{
        local_variable_section::{LocalListEntry, LocalVariableEntry},
        type_section::TypeEntry,
    },
};
use ancvm_program::program_source::ProgramSource;
use ancvm_process::{
    in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
    InterpreterError, InterpreterErrorType,
};
use ancvm_types::{DataType, ForeignValue, MemoryDataType};

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

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
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
0x002e  00 01                       nop
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

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
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
    // fn (a/0:i32, b/1:i32) -> (i32,i32,i32,i32,i32,i32,i32,i32)
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

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
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

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
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

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
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
0x0046  00 01                       nop
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

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
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
0x0046  00 01                       nop
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
    // fn $max (i32, i32) -> (i32)
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

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $max
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
0x0022  00 01                       nop
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

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
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
0x006a  00 01                       nop
0x006c  80 01 00 00  29 00 00 00    i32.imm           0x00000029
0x0074  80 01 00 00  2b 00 00 00    i32.imm           0x0000002b
0x007c  00 0a                       end
0x007e  00 01                       nop
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

#[test]
fn test_assemble_control_flow_structure_if() {
    // fn $max (i32, i32) -> (i32)
    //     (local_load32 0 0)
    //     (local_load32 0 1)
    //     i32_gt_u
    //     (block_alt 1 1) ()->(i32)
    //         (local_load32 1 0)
    //     (break 0)
    //         (local_load32 1 1)
    //     end
    // end
    //
    // assert (11, 13) -> (13)
    // assert (19, 17) -> (19)

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
                (param $a i32)
                (param $b i32)
                (results i32)
                (code
                    (if
                        (result i32)
                        (i32.gt_u
                            (local.load32_i32 $a)
                            (local.load32_i32 $b)
                        )
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

    let bytecode_text = print_bytecode_as_text(&func_entry.code);
    // println!("{}", bytecode_text);

    assert_eq!(
        bytecode_text,
        "\
0x0000  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x0008  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x0010  07 06                       i32.gt_u
0x0012  00 01                       nop
0x0014  05 0a 00 00  01 00 00 00    block_alt         type:1   local:1   off:0x20
        01 00 00 00  20 00 00 00
0x0024  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x002c  02 0a 00 00  12 00 00 00    break             rev:0   off:0x12
0x0034  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
0x003c  00 0a                       end
0x003e  00 0a                       end"
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
fn test_assemble_control_flow_structure_if_nested() {
    // fn $level (i32) -> (i32)
    //     (local_load32 0 0)
    //     (i32_imm 85)
    //     i32_gt_u
    //     (block_alt 1 1) ()->(i32)            ;; block 1 1
    //         (i32_imm 65)                     ;; 'A' (85, 100]
    //     (break 0)
    //         (local_load32 1 0)
    //         (i32_imm 70)
    //         i32_gt_u
    //         (block_alt 2 2) ()->(i32)        ;; block 2 2
    //             (i32_imm 66)                 ;; 'B' (70,85]
    //         (break 0)
    //             (local_load32 2 0)
    //             (i32_imm 55)
    //             i32_gt_u
    //             (block_alt 3 3) ()->(i32)    ;; block 3 3
    //                 (i32_imm 67)             ;; 'C' (55, 70]
    //             (break 0)
    //                 (i32_imm 68)             ;; 'D' [0, 55]
    //             end
    //         end
    //     end
    // end
    //
    // assert (90) -> (65) 'A'
    // assert (80) -> (66) 'B'
    // assert (70) -> (67) 'C'
    // assert (60) -> (67) 'C'
    // assert (50) -> (68) 'D'
    // assert (40) -> (68) 'D'

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
                (param $a i32)
                (results i32)
                (code
                    (if
                        (result i32)
                        (i32.gt_u
                            (local.load32_i32 $a)
                            (i32.imm 85)
                        )
                        (i32.imm 65)            ;; 'A'
                        (if
                            (result i32)
                            (i32.gt_u
                                (local.load32_i32 $a)
                                (i32.imm 70)
                            )
                            (i32.imm 66)        ;; 'B'
                            (if
                                (result i32)
                                (i32.gt_u
                                    (local.load32_i32 $a)
                                    (i32.imm 55)
                                )
                                (i32.imm 67)    ;; 'C'
                                (i32.imm 68)    ;; 'D'
                            )
                        )
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

    let bytecode_text = print_bytecode_as_text(&func_entry.code);
    // println!("{}", bytecode_text);

    assert_eq!(
        bytecode_text,
        "\
0x0000  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x0008  80 01 00 00  55 00 00 00    i32.imm           0x00000055
0x0010  07 06                       i32.gt_u
0x0012  00 01                       nop
0x0014  05 0a 00 00  01 00 00 00    block_alt         type:1   local:1   off:0x20
        01 00 00 00  20 00 00 00
0x0024  80 01 00 00  41 00 00 00    i32.imm           0x00000041
0x002c  02 0a 00 00  7e 00 00 00    break             rev:0   off:0x7e
0x0034  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x003c  80 01 00 00  46 00 00 00    i32.imm           0x00000046
0x0044  07 06                       i32.gt_u
0x0046  00 01                       nop
0x0048  05 0a 00 00  01 00 00 00    block_alt         type:1   local:1   off:0x20
        01 00 00 00  20 00 00 00
0x0058  80 01 00 00  42 00 00 00    i32.imm           0x00000042
0x0060  02 0a 00 00  48 00 00 00    break             rev:0   off:0x48
0x0068  02 02 02 00  00 00 00 00    local.load32_i32  rev:2   off:0x00  idx:0
0x0070  80 01 00 00  37 00 00 00    i32.imm           0x00000037
0x0078  07 06                       i32.gt_u
0x007a  00 01                       nop
0x007c  05 0a 00 00  01 00 00 00    block_alt         type:1   local:1   off:0x20
        01 00 00 00  20 00 00 00
0x008c  80 01 00 00  43 00 00 00    i32.imm           0x00000043
0x0094  02 0a 00 00  12 00 00 00    break             rev:0   off:0x12
0x009c  80 01 00 00  44 00 00 00    i32.imm           0x00000044
0x00a4  00 0a                       end
0x00a6  00 0a                       end
0x00a8  00 0a                       end
0x00aa  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(90)]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(65)]);

    let result1 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(80)]);
    assert_eq!(result1.unwrap(), vec![ForeignValue::UInt32(66)]);

    let result2 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(70)]);
    assert_eq!(result2.unwrap(), vec![ForeignValue::UInt32(67)]);

    let result3 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(60)]);
    assert_eq!(result3.unwrap(), vec![ForeignValue::UInt32(67)]);

    let result4 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(50)]);
    assert_eq!(result4.unwrap(), vec![ForeignValue::UInt32(68)]);

    let result5 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(40)]);
    assert_eq!(result5.unwrap(), vec![ForeignValue::UInt32(68)]);
}

#[test]
fn test_assemble_control_flow_structure_branch() {
    // fn $level (i32) -> (i32)
    //     (block 1 1) ()->(i32)        ;; block 1 1
    //                                  ;; case 1
    //         (local_load32 0 0)
    //         (i32_imm 85)
    //         i32_gt_u
    //         (block_nez 2) ()->()     ;; block 2 2
    //             (i32_imm 65)         ;; 'A' (85, 100]
    //             (break 1)
    //         end
    //                                  ;; case 2
    //         (local_load32 0 0)
    //         (i32_imm 70)
    //         i32_gt_u
    //         (block_nez 3) ()->()     ;; block 3 3
    //             (i32_imm 66)         ;; 'B' (70,85]
    //             (break 1)
    //         end
    //                                  ;; case 3
    //         (local_load32 0 0)
    //         (i32_imm 55)
    //         i32_gt_u
    //         (block_nez 4) ()->()     ;; block 4 4
    //             (i32_imm 67)         ;; 'C' (55, 70]
    //             (break 1)
    //         end
    //                                  ;; default
    //         (i32_imm 68)             ;; 'D' [0, 55]
    //     end
    // end
    //
    // assert (90) -> (65) 'A'
    // assert (80) -> (66) 'B'
    // assert (70) -> (67) 'C'
    // assert (60) -> (67) 'C'
    // assert (50) -> (68) 'D'
    // assert (40) -> (68) 'D'

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
                (param $a i32)
                (results i32)
                (code
                    (branch
                        (result i32)
                        (case
                            (i32.gt_u
                                (local.load32_i32 $a)
                                (i32.imm 85)
                            )
                            (i32.imm 65)    ;; 'A'
                        )
                        (case
                            (i32.gt_u
                                (local.load32_i32 $a)
                                (i32.imm 70)
                            )
                            (i32.imm 66)    ;; 'B'
                        )
                        (case
                            (i32.gt_u
                                (local.load32_i32 $a)
                                (i32.imm 55)
                            )
                            (i32.imm 67)    ;; 'C'
                        )
                        (default
                            (i32.imm 68)    ;; 'D'
                        )
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

    let bytecode_text = print_bytecode_as_text(&func_entry.code);
    // println!("{}", bytecode_text);

    assert_eq!(
        bytecode_text,
        "\
0x0000  01 0a 00 00  01 00 00 00    block             type:1   local:1
        01 00 00 00
0x000c  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x0014  80 01 00 00  55 00 00 00    i32.imm           0x00000055
0x001c  07 06                       i32.gt_u
0x001e  00 01                       nop
0x0020  04 0a 00 00  01 00 00 00    block_nez         local:1   off:0x1e
        1e 00 00 00
0x002c  80 01 00 00  41 00 00 00    i32.imm           0x00000041
0x0034  02 0a 01 00  7e 00 00 00    break             rev:1   off:0x7e
0x003c  00 0a                       end
0x003e  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x0046  00 01                       nop
0x0048  80 01 00 00  46 00 00 00    i32.imm           0x00000046
0x0050  07 06                       i32.gt_u
0x0052  00 01                       nop
0x0054  04 0a 00 00  01 00 00 00    block_nez         local:1   off:0x1e
        1e 00 00 00
0x0060  80 01 00 00  42 00 00 00    i32.imm           0x00000042
0x0068  02 0a 01 00  4a 00 00 00    break             rev:1   off:0x4a
0x0070  00 0a                       end
0x0072  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x007a  00 01                       nop
0x007c  80 01 00 00  37 00 00 00    i32.imm           0x00000037
0x0084  07 06                       i32.gt_u
0x0086  00 01                       nop
0x0088  04 0a 00 00  01 00 00 00    block_nez         local:1   off:0x1e
        1e 00 00 00
0x0094  80 01 00 00  43 00 00 00    i32.imm           0x00000043
0x009c  02 0a 01 00  16 00 00 00    break             rev:1   off:0x16
0x00a4  00 0a                       end
0x00a6  00 01                       nop
0x00a8  80 01 00 00  44 00 00 00    i32.imm           0x00000044
0x00b0  00 0a                       end
0x00b2  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(90)]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(65)]);

    let result1 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(80)]);
    assert_eq!(result1.unwrap(), vec![ForeignValue::UInt32(66)]);

    let result2 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(70)]);
    assert_eq!(result2.unwrap(), vec![ForeignValue::UInt32(67)]);

    let result3 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(60)]);
    assert_eq!(result3.unwrap(), vec![ForeignValue::UInt32(67)]);

    let result4 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(50)]);
    assert_eq!(result4.unwrap(), vec![ForeignValue::UInt32(68)]);

    let result5 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(40)]);
    assert_eq!(result5.unwrap(), vec![ForeignValue::UInt32(68)]);
}

#[test]
fn test_assemble_control_flow_structure_branch_without_default_arm() {
    // fn $level (i32) -> (i32)
    //     (block 1 1) ()->(i32)        ;; block 1 1
    //                                  ;; case 1
    //         (local_load32 0 0)
    //         (i32_imm 85)
    //         i32_gt_u
    //         (block_nez 2) ()->()     ;; block 2 2
    //             (i32_imm 65)         ;; 'A' (85, 100]
    //             (break 1)
    //         end
    //                                  ;; case 2
    //         (local_load32 0 0)
    //         (i32_imm 70)
    //         i32_gt_u
    //         (block_nez 3) ()->()     ;; block 3 3
    //             (i32_imm 66)         ;; 'B' (70,85]
    //             (break 1)
    //         end
    //         unreachable
    //     end
    // end
    //
    // assert (90) -> (65) 'A'
    // assert (80) -> (66) 'B'
    // assert (70) -> unreachable
    // assert (60) -> unreachable

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
                (param $a i32)
                (results i32)
                (code
                    (branch
                        (result i32)
                        (case
                            (i32.gt_u
                                (local.load32_i32 $a)
                                (i32.imm 85)
                            )
                            (i32.imm 65)    ;; 'A'
                        )
                        (case
                            (i32.gt_u
                                (local.load32_i32 $a)
                                (i32.imm 70)
                            )
                            (i32.imm 66)    ;; 'B'
                        )
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

    let bytecode_text = print_bytecode_as_text(&func_entry.code);
    // println!("{}", bytecode_text);

    assert_eq!(
        bytecode_text,
        "\
0x0000  01 0a 00 00  01 00 00 00    block             type:1   local:1
        01 00 00 00
0x000c  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x0014  80 01 00 00  55 00 00 00    i32.imm           0x00000055
0x001c  07 06                       i32.gt_u
0x001e  00 01                       nop
0x0020  04 0a 00 00  01 00 00 00    block_nez         local:1   off:0x1e
        1e 00 00 00
0x002c  80 01 00 00  41 00 00 00    i32.imm           0x00000041
0x0034  02 0a 01 00  4a 00 00 00    break             rev:1   off:0x4a
0x003c  00 0a                       end
0x003e  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x0046  00 01                       nop
0x0048  80 01 00 00  46 00 00 00    i32.imm           0x00000046
0x0050  07 06                       i32.gt_u
0x0052  00 01                       nop
0x0054  04 0a 00 00  01 00 00 00    block_nez         local:1   off:0x1e
        1e 00 00 00
0x0060  80 01 00 00  42 00 00 00    i32.imm           0x00000042
0x0068  02 0a 01 00  16 00 00 00    break             rev:1   off:0x16
0x0070  00 0a                       end
0x0072  00 01                       nop
0x0074  01 0c 00 00  00 01 00 00    unreachable       code:256
0x007c  00 0a                       end
0x007e  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(90)]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(65)]);

    let result1 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(80)]);
    assert_eq!(result1.unwrap(), vec![ForeignValue::UInt32(66)]);

    let result2 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(70)]);
    assert!(matches!(
        result2,
        Err(InterpreterError {
            error_type: InterpreterErrorType::Unreachable(0x100)
        })
    ));

    let result3 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(60)]);
    assert!(matches!(
        result3,
        Err(InterpreterError {
            error_type: InterpreterErrorType::Unreachable(0x100)
        })
    ));
}

#[test]
fn test_assemble_control_flow_structure_loop() {
    // fn $accu (n/0:i32) -> (i32)
    //     (local sum/1:i32)
    //     (block 1 1) ()->()
    //                              ;; break if n==0
    //         (local_load32 1 0)
    //         i32_eqz
    //         (block_nez 2)
    //             (break 1)
    //         end
    //                              ;; sum = sum + n
    //         (local_load32 1 0)
    //         (local_load32 1 1)
    //         i32_add
    //         (local_store32 1 1)
    //                              ;; n = n - 1
    //         (local_load32 1 0)
    //         (i32_dec 1)
    //         (local_store32 1 0)
    //                              ;; recur
    //         (recur 0)
    //     end
    //     (local_load32 0 1)
    // end
    //
    // assert (10) -> (55)
    // assert (100) -> (5050)

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
                (param $n i32)
                (results i32)
                (local $sum i32)
                (code
                    (for
                        (do
                            (when
                                (i32.eqz (local.load32_i32 $n))
                                (break)
                            )
                            (local.store32 $sum
                                (i32.add
                                    (local.load32_i32 $sum)
                                    (local.load32_i32 $n)
                                )
                            )
                            (local.store32 $n
                                (i32.dec 1
                                    (local.load32_i32 $n)
                                )
                            )
                            (recur)
                        )
                    )
                    (local.load32_i32 $sum)
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
0x0000  01 0a 00 00  01 00 00 00    block             type:1   local:1
        01 00 00 00
0x000c  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x0014  00 06                       i32.eqz
0x0016  00 01                       nop
0x0018  04 0a 00 00  01 00 00 00    block_nez         local:1   off:0x16
        16 00 00 00
0x0024  02 0a 01 00  42 00 00 00    break             rev:1   off:0x42
0x002c  00 0a                       end
0x002e  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
0x0036  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x003e  00 07                       i32.add
0x0040  09 02 01 00  00 00 01 00    local.store32     rev:1   off:0x00  idx:1
0x0048  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x0050  08 07 01 00                 i32.dec           1
0x0054  09 02 01 00  00 00 00 00    local.store32     rev:1   off:0x00  idx:0
0x005c  03 0a 00 00  50 00 00 00    recur             rev:0   off:0x50
0x0064  00 0a                       end
0x0066  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x006e  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(10)]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(55)]);

    let result1 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(100)]);
    assert_eq!(result1.unwrap(), vec![ForeignValue::UInt32(5050)]);
}

#[test]
fn test_assemble_control_flow_structure_loop_with_block_parameters() {
    // fn $accu (count/0:i32) -> (i32)
    //     zero                     ;; sum
    //     (local_load32 0 0)       ;; count
    //     (block 1 1) (sum/0:i32, n/1:i32)->(i32)
    //                              ;; break if n==0
    //         (local_load32 0 1)
    //         i32_eqz
    //         (block_nez)
    //             (local_load32 0 1)
    //             (break 1)
    //         end
    //                              ;; sum + n
    //         (local_load32 0 0)
    //         (local_load32 0 1)
    //         i32_add
    //                              ;; n - 1
    //         (local_load32 0 1)
    //         (i32_dec 1)
    //                              ;; recur
    //         (recur 0)
    //     end
    // end
    //
    // assert (10) -> (55)
    // assert (100) -> (5050)

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
                (param $count i32)
                (results i32)
                (code
                    zero                        ;; for arg 'sum'
                    (local.load32_i32 $count)   ;; for arg 'n'
                    (for
                        (param $sum i32)
                        (param $n i32)
                        (result i32)
                        (do
                            (when
                                (i32.eqz (local.load32_i32 $n))
                                (break (local.load32_i32 $sum))
                            )

                            (recur
                                (i32.add
                                    (local.load32_i32 $sum)
                                    (local.load32_i32 $n)
                                )

                                (i32.dec 1
                                    (local.load32_i32 $n)
                                )
                            )
                        )
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

    let bytecode_text = print_bytecode_as_text(&func_entry.code);
    // println!("{}", bytecode_text);

    assert_eq!(
        bytecode_text,
        "\
0x0000  01 01                       zero
0x0002  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x000a  00 01                       nop
0x000c  01 0a 00 00  01 00 00 00    block             type:1   local:1
        01 00 00 00
0x0018  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x0020  00 06                       i32.eqz
0x0022  00 01                       nop
0x0024  04 0a 00 00  02 00 00 00    block_nez         local:2   off:0x1e
        1e 00 00 00
0x0030  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x0038  02 0a 01 00  32 00 00 00    break             rev:1   off:0x32
0x0040  00 0a                       end
0x0042  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x004a  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x0052  00 07                       i32.add
0x0054  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x005c  08 07 01 00                 i32.dec           1
0x0060  03 0a 00 00  48 00 00 00    recur             rev:0   off:0x48
0x0068  00 0a                       end
0x006a  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(10)]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(55)]);

    let result1 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(100)]);
    assert_eq!(result1.unwrap(), vec![ForeignValue::UInt32(5050)]);
}

#[test]
fn test_assemble_control_flow_structure_loop_with_if() {
    // fn $accu (count/0:i32) -> (i32)
    //     zero                     ;; sum
    //     (local_load32 0 0)       ;; count
    //     (block 1 1) (sum/0:i32, n/1:i32)->(i32)
    //                              ;; if n==0
    //         (local_load32 0 1)
    //         i32_eqz
    //         (block_alt)
    //             (local_load32 0 1)
    //             (break 1)
    //         (break 0)
    //                              ;; sum + n
    //             (local_load32 0 0)
    //             (local_load32 0 1)
    //             i32_add
    //                              ;; n - 1
    //             (local_load32 0 1)
    //             (i32_dec 1)
    //                              ;; recur
    //             (recur 0)
    //         end
    //     end
    // end
    //
    // assert (10) -> (55)
    // assert (100) -> (5050)

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
                (param $count i32)
                (results i32)
                (code
                    zero                        ;; for arg 'sum'
                    (local.load32_i32 $count)   ;; for arg 'n'
                    (for
                        (param $sum i32)
                        (param $n i32)
                        (result i32)
                        (do
                            (if
                                (i32.eqz (local.load32_i32 $n))
                                (break (local.load32_i32 $sum))
                                (recur
                                    (i32.add
                                        (local.load32_i32 $sum)
                                        (local.load32_i32 $n)
                                    )

                                    (i32.dec 1
                                        (local.load32_i32 $n)
                                    )
                                )
                            )
                        )
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

    let bytecode_text = print_bytecode_as_text(&func_entry.code);
    // println!("{}", bytecode_text);

    assert_eq!(
        bytecode_text,
        "\
0x0000  01 01                       zero
0x0002  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x000a  00 01                       nop
0x000c  01 0a 00 00  01 00 00 00    block             type:1   local:1
        01 00 00 00
0x0018  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x0020  00 06                       i32.eqz
0x0022  00 01                       nop
0x0024  05 0a 00 00  02 00 00 00    block_alt         type:2   local:2   off:0x28
        02 00 00 00  28 00 00 00
0x0034  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x003c  02 0a 01 00  3c 00 00 00    break             rev:1   off:0x3c
0x0044  02 0a 00 00  32 00 00 00    break             rev:0   off:0x32
0x004c  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x0054  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
0x005c  00 07                       i32.add
0x005e  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
0x0066  08 07 01 00                 i32.dec           1
0x006a  00 01                       nop
0x006c  03 0a 01 00  54 00 00 00    recur             rev:1   off:0x54
0x0074  00 0a                       end
0x0076  00 0a                       end
0x0078  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(10)]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(55)]);

    let result1 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::UInt32(100)]);
    assert_eq!(result1.unwrap(), vec![ForeignValue::UInt32(5050)]);
}

#[test]
fn test_assemble_control_flow_function_tail_call() {
    // fn $accu (sum/0:i32, n/1:i32) -> (i32)
    //                              ;; sum = sum + n
    //     (local_load32 0 0)
    //     (local_load32 0 1)
    //     i32_add
    //     (local_store32 0 0)
    //                              ;; n = n - 1
    //     (local_load32 0 1)
    //     (i32_dec 1)
    //     (local_store32 0 1)
    //                              ;; if n > 0 recur (sum,n)
    //     (local_load32 0 1)
    //     zero
    //     i32_gt_u
    //     (block_nez 1) () -> ()
    //         (local_load32 0 0)
    //         (local_load32 0 1)
    //         (recur 1)
    //     end
    //     (local_load32 0 0)       ;; load sum
    // end
    //
    // assert (0, 10) -> (55)
    // assert (0, 100) -> (5050)

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
                (param $sum i32)
                (param $n i32)
                (results i32)
                (code
                    (local.store32 $sum
                        (i32.add
                            (local.load32_i32 $sum)
                            (local.load32_i32 $n)
                        )
                    )
                    (local.store32 $n
                        (i32.dec 1
                            (local.load32_i32 $n)
                        )
                    )
                    (when
                        (i32.gt_u
                            (local.load32_i32 $n)
                            zero
                        )
                        (rerun
                            (local.load32_i32 $sum)
                            (local.load32_i32 $n)
                        )
                    )
                    (local.load32_i32 $sum)
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
0x0008  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x0010  00 07                       i32.add
0x0012  09 02 00 00  00 00 00 00    local.store32     rev:0   off:0x00  idx:0
0x001a  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x0022  08 07 01 00                 i32.dec           1
0x0026  09 02 00 00  00 00 01 00    local.store32     rev:0   off:0x00  idx:1
0x002e  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x0036  01 01                       zero
0x0038  07 06                       i32.gt_u
0x003a  00 01                       nop
0x003c  04 0a 00 00  01 00 00 00    block_nez         local:1   off:0x26
        26 00 00 00
0x0048  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x0050  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
0x0058  03 0a 01 00  00 00 00 00    recur             rev:1   off:0x00
0x0060  00 0a                       end
0x0062  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
0x006a  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::UInt32(0), ForeignValue::UInt32(10)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(55)]);

    let result1 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::UInt32(0), ForeignValue::UInt32(100)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::UInt32(5050)]);
}

#[test]
fn test_assemble_control_flow_function_tail_call_with_if() {
    // fn $accu (sum:i32, n:i32) -> (i32)
    //     (local_load32 0 1)               ;; load n
    //     i32_eqz
    //     (block_alt 1 1) () -> (i32)      ;; if n == 0
    //         (local_load32 1 0)           ;; then sum
    //     (break 0)                        ;; else
    //                                      ;; sum + n
    //         (local_load32 1 0)
    //         (local_load32 1 1)
    //         i32_add
    //                                      ;; n - 1
    //         (local_load32 1 1)
    //         (i32_dec 1)
    //         (recur 1)                    ;; recur
    //     end
    // end
    //
    // assert (0, 10) -> (55)
    // assert (0, 100) -> (5050)

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
                (param $sum i32)
                (param $n i32)
                (results i32)
                (code
                    (if
                        (result i32)
                        (i32.eqz
                            (local.load32_i32 $n)
                        )
                        (local.load32_i32 $sum)
                        (rerun
                            (i32.add
                                (local.load32_i32 $sum)
                                (local.load32_i32 $n)
                            )
                            (i32.dec 1
                                (local.load32_i32 $n)
                            )

                        )
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

    let bytecode_text = print_bytecode_as_text(&func_entry.code);
    // println!("{}", bytecode_text);

    assert_eq!(
        bytecode_text,
        "\
0x0000  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x0008  00 06                       i32.eqz
0x000a  00 01                       nop
0x000c  05 0a 00 00  01 00 00 00    block_alt         type:1   local:1   off:0x20
        01 00 00 00  20 00 00 00
0x001c  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x0024  02 0a 00 00  32 00 00 00    break             rev:0   off:0x32
0x002c  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
0x0034  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
0x003c  00 07                       i32.add
0x003e  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
0x0046  08 07 01 00                 i32.dec           1
0x004a  00 01                       nop
0x004c  03 0a 01 00  00 00 00 00    recur             rev:1   off:0x00
0x0054  00 0a                       end
0x0056  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::UInt32(0), ForeignValue::UInt32(10)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(55)]);

    let result1 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::UInt32(0), ForeignValue::UInt32(100)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::UInt32(5050)]);
}
