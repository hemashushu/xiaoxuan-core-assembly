// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use anc_assembler::utils::helper_make_single_module_app;
use anc_context::resource::Resource;
use anc_isa::ForeignValue;
use anc_processor::{
    handler::Handler, in_memory_resource::InMemoryResource, process::process_function,
};
use pretty_assertions::assert_eq;

#[test]
fn test_assemble_bitwise_i32() {
    // numbers:
    //   - 0: 0xff00_00ff
    //   - 1: 0xf0f0_00ff
    //   - 2: 0x00f0_0000
    //   - 3: 0x8000_0000

    // arithemtic:
    //   group 0:
    //   - and       0 1      -> 0xf000_00ff
    //   - or        0 1      -> 0xfff0_00ff
    //   - xor       0 1      -> 0x0ff0_0000
    //   - not       0        -> 0x00ff_ff00
    //
    //   group 1:
    //   - shift_l   2 imm:4    -> 0x0f00_0000
    //   - shift_r_s 3 imm:16   -> 0xffff_8000
    //   - shift_r_u 3 imm:16   -> 0x0000_8000
    //
    //   group 2:
    //   - shift_l   2 imm:24   -> 0x0000_0000
    //   - rotate_l  2 imm:24   -> 0x0000_f000
    //   - shift_r_u 2 imm:28   -> 0x0000_0000
    //   - rotate_r  2 imm:28   -> 0x0f00_0000
    //
    //   group 3:
    //   - cls       0        -> 8
    //   - cls       1        -> 4
    //   - clz       2        -> 8
    //   - ctz       2        -> 20
    //   - ones      2        -> 4
    //
    // (i32 i32 i32 i32) -> (i32 i32 i32 i32  i32 i32 i32  i32 i32 i32 i32  i32 i32 i32 i32 i32)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(a0:f32, a1:f32, a2:f32, a3:f32) ->
            (
            i32, i32, i32, i32
            i32, i32, i32,
            i32, i32, i32, i32
            i32, i32, i32, i32, i32
            )
        {
            and(local_load_i32_s(a0), local_load_i32_s(a1))
            or(local_load_i32_s(a0), local_load_i32_s(a1))
            xor(local_load_i32_s(a0), local_load_i32_s(a1))
            not(local_load_i32_s(a0))

            shift_left_i32 (local_load_i32_s(a2), imm_i32(4))
            shift_right_i32_s (local_load_i32_s(a3), imm_i32(16))
            shift_right_i32_u (local_load_i32_s(a3), imm_i32(16))

            shift_left_i32 (local_load_i32_s(a2), imm_i32(24))
            rotate_left_i32 (local_load_i32_s(a2), imm_i32(24))
            shift_right_i32_u (local_load_i32_s(a2), imm_i32(28))
            rotate_right_i32 (local_load_i32_s(a2), imm_i32(28))

            count_leading_ones_i32 (local_load_i32_s(a0))
            count_leading_ones_i32 (local_load_i32_s(a1))
            count_leading_zeros_i32 (local_load_i32_s(a2))
            count_trailing_zeros_i32 (local_load_i32_s(a2))
            count_ones_i32 (local_load_i32_s(a2))
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &handler,
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
            ForeignValue::U32(0x00ffff00),
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
    //   - not       0        -> 0x00ff00ff_ff00ff00
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
    //   - cls       0        -> 8
    //   - cls       1        -> 4
    //   - clz       2        -> 16
    //   - ctz       2        -> 40
    //   - ones      2        -> 8
    //
    // (i64 i64 i64 i64) -> (i64 i64 i64  i64 i64 i64  i64 i64 i64 i64  i64 i32 i32 i32 i32 i32)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(a0:f64, a1:f64, a2:f64, a3:f64) ->
            (
            i64, i64, i64, i64
            i64, i64, i64,
            i64, i64, i64, i64
            i32, i32, i32, i32, i32
            )
        {
            and (local_load_i64(a0), local_load_i64(a1))
            or (local_load_i64(a0), local_load_i64(a1))
            xor (local_load_i64(a0), local_load_i64(a1))
            not (local_load_i64(a0))

            shift_left_i64 (local_load_i64(a2), imm_i32(8))
            shift_right_i64_s (local_load_i64(a3), imm_i32(16))
            shift_right_i64_u (local_load_i64(a3), imm_i32(16))

            shift_left_i64 (local_load_i64(a2), imm_i32(32))
            rotate_left_i64 (local_load_i64(a2), imm_i32(32))
            shift_right_i64_u (local_load_i64(a2), imm_i32(56))
            rotate_right_i64 (local_load_i64(a2), imm_i32(56))

            count_leading_ones_i64 (local_load_i64(a0))
            count_leading_ones_i64 (local_load_i64(a1))
            count_leading_zeros_i64 (local_load_i64(a2))
            count_trailing_zeros_i64 (local_load_i64(a2))
            count_ones_i64 (local_load_i64(a2))
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &handler,
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
            ForeignValue::U64(0x00ff00ff_ff00ff00),
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
            ForeignValue::U32(8),
            ForeignValue::U32(4),
            ForeignValue::U32(16),
            ForeignValue::U32(40),
            ForeignValue::U32(8),
        ]
    );
}
