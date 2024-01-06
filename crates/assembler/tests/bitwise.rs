// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancasm_assembler::utils::helper_generate_module_image_binary_from_str;
use ancvm_processor::{
    in_memory_program_resource::InMemoryProgramResource, interpreter::process_function,
};
use ancvm_context::program_resource::ProgramResource;
use ancvm_types::ForeignValue;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_bitwise_i32() {
    // numbers:
    //   - 0: 0xff0000ff
    //   - 1: 0xf0f000ff
    //   - 2: 0x00f00000
    //   - 3: 0x80000000

    // arithemtic:
    //   group 0:
    //   - and       0 1      -> 0xf00000ff
    //   - or        0 1      -> 0xfff000ff
    //   - xor       0 1      -> 0x0ff00000
    //
    //   group 1:
    //   - shift_l   2 imm:4    -> 0x0f000000
    //   - shift_r_s 3 imm:16   -> 0xffff8000
    //   - shift_r_u 3 imm:16   -> 0x00008000
    //
    //   group 2:
    //   - shift_l   2 imm:24   -> 0x00000000
    //   - rotate_l  2 imm:24   -> 0x0000f000
    //   - shift_r_u 2 imm:28   -> 0x00000000
    //   - rotate_r  2 imm:28   -> 0x0f000000
    //
    //   group 3:
    //   - not       0        -> 0x00ffff00
    //   - cls       0        -> 8
    //   - cls       1        -> 4
    //   - clz       2        -> 8
    //   - ctz       2        -> 20
    //   - ones      2        -> 4
    //
    // (i32 i32 i32 i32) -> (i32 i32 i32  i32 i32 i32  i32 i32 i32 i32  i32 i32 i32 i32 i32 i32)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test
                (param $a0 f32)
                (param $a1 f32)
                (param $a2 f32)
                (param $a3 f32)
                (results
                    i32 i32 i32
                    i32 i32 i32
                    i32 i32 i32 i32
                    i32 i32 i32 i32 i32 i32)
                (code
                    (i32.and (local.load32_i32 $a0) (local.load32_i32 $a1))
                    (i32.or (local.load32_i32 $a0) (local.load32_i32 $a1))
                    (i32.xor (local.load32_i32 $a0) (local.load32_i32 $a1))

                    (i32.shift_left (local.load32_i32 $a2) (i32.imm 4))
                    (i32.shift_right_s (local.load32_i32 $a3) (i32.imm 16))
                    (i32.shift_right_u (local.load32_i32 $a3) (i32.imm 16))

                    (i32.shift_left (local.load32_i32 $a2) (i32.imm 24))
                    (i32.rotate_left (local.load32_i32 $a2) (i32.imm 24))
                    (i32.shift_right_u (local.load32_i32 $a2) (i32.imm 28))
                    (i32.rotate_right (local.load32_i32 $a2) (i32.imm 28))

                    (i32.not (local.load32_i32 $a0))
                    (i32.leading_ones (local.load32_i32 $a0))
                    (i32.leading_ones (local.load32_i32 $a1))
                    (i32.leading_zeros (local.load32_i32 $a2))
                    (i32.trailing_zeros (local.load32_i32 $a2))
                    (i32.count_ones (local.load32_i32 $a2))
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::U32(0xff0000ff),
            ForeignValue::U32(0xf0f000ff),
            ForeignValue::U32(0x00f00000),
            ForeignValue::U32(0x80000000),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::U32(0xf00000ff),
            ForeignValue::U32(0xfff000ff),
            ForeignValue::U32(0x0ff00000),
            // group 1
            ForeignValue::U32(0x0f000000),
            ForeignValue::U32(0xffff8000),
            ForeignValue::U32(0x00008000),
            // group 2
            ForeignValue::U32(0x00000000),
            ForeignValue::U32(0x0000f000),
            ForeignValue::U32(0x00000000),
            ForeignValue::U32(0x0f000000),
            // group 3
            ForeignValue::U32(0x00ffff00),
            ForeignValue::U32(8),
            ForeignValue::U32(4),
            ForeignValue::U32(8),
            ForeignValue::U32(20),
            ForeignValue::U32(4),
        ]
    );
}

#[test]
fn test_assemble_bitwise_i64() {
    // numbers:
    //   - 0: 0xff00ff00_00ff00ff
    //   - 1: 0xf0f00f0f_00ff00ff
    //   - 2: 0x0000ff00_00000000
    //   - 3: 0x80000000_00000000

    // arithemtic:
    //   group 0:
    //   - and       0 1      -> 0xf0000f00_00ff00ff
    //   - or        0 1      -> 0xfff0ff0f_00ff00ff
    //   - xor       0 1      -> 0x0ff0f00f_00000000
    //
    //   group 1:
    //   - shift_l   2 8      -> 0x00ff0000_00000000
    //   - shift_r_s 3 16     -> 0xffff8000_00000000
    //   - shift_r_u 3 16     -> 0x00008000_00000000
    //
    //   group 2:
    //   - shift_l   2 32     -> 0x00000000_00000000
    //   - rotate_l  2 32     -> 0x00000000_0000ff00
    //   - shift_r_u 2 56     -> 0x00000000_00000000
    //   - rotate_r  2 56     -> 0x00ff0000_00000000
    //
    //   group 3:
    //   - not       0        -> 0x00ff00ff_ff00ff00
    //   - cls       0        -> 8
    //   - cls       1        -> 4
    //   - lz        2        -> 16
    //   - tz        2        -> 40
    //   - ones      2        -> 8
    //
    // (i64 i64 i64 i64) -> (i64 i64 i64  i64 i64 i64  i64 i64 i64 i64  i64 i32 i32 i32 i32 i32)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test
                (param $a0 f64)
                (param $a1 f64)
                (param $a2 f64)
                (param $a3 f64)
                (results
                    i64 i64 i64
                    i64 i64 i64
                    i64 i64 i64 i64
                    i64 i32 i32 i32 i32 i32)
                (code
                    (i64.and (local.load64_i64 $a0) (local.load64_i64 $a1))
                    (i64.or (local.load64_i64 $a0) (local.load64_i64 $a1))
                    (i64.xor (local.load64_i64 $a0) (local.load64_i64 $a1))

                    (i64.shift_left (local.load64_i64 $a2) (i32.imm 8))
                    (i64.shift_right_s (local.load64_i64 $a3) (i32.imm 16))
                    (i64.shift_right_u (local.load64_i64 $a3) (i32.imm 16))

                    (i64.shift_left (local.load64_i64 $a2) (i32.imm 32))
                    (i64.rotate_left (local.load64_i64 $a2) (i32.imm 32))
                    (i64.shift_right_u (local.load64_i64 $a2) (i32.imm 56))
                    (i64.rotate_right (local.load64_i64 $a2) (i32.imm 56))

                    (i64.not (local.load64_i64 $a0))
                    (i64.leading_ones (local.load64_i64 $a0))
                    (i64.leading_ones (local.load64_i64 $a1))
                    (i64.leading_zeros (local.load64_i64 $a2))
                    (i64.trailing_zeros (local.load64_i64 $a2))
                    (i64.count_ones (local.load64_i64 $a2))
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::U64(0xff00ff00_00ff00ff),
            ForeignValue::U64(0xf0f00f0f_00ff00ff),
            ForeignValue::U64(0x0000ff00_00000000),
            ForeignValue::U64(0x80000000_00000000),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::U64(0xf0000f00_00ff00ff),
            ForeignValue::U64(0xfff0ff0f_00ff00ff),
            ForeignValue::U64(0x0ff0f00f_00000000),
            // group 1
            ForeignValue::U64(0x00ff0000_00000000),
            ForeignValue::U64(0xffff8000_00000000),
            ForeignValue::U64(0x00008000_00000000),
            // group 2
            ForeignValue::U64(0x00000000_00000000),
            ForeignValue::U64(0x00000000_0000ff00),
            ForeignValue::U64(0x00000000_00000000),
            ForeignValue::U64(0x00ff0000_00000000),
            // group 3
            ForeignValue::U64(0x00ff00ff_ff00ff00),
            ForeignValue::U32(8),
            ForeignValue::U32(4),
            ForeignValue::U32(16),
            ForeignValue::U32(40),
            ForeignValue::U32(8),
        ]
    );
}
