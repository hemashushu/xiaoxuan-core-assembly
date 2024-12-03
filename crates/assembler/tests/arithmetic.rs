// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
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
fn test_assemble_arithmetic_i32() {
    // numbers:
    //   - 0: 11
    //   - 1: 211
    //   - 2: -13

    // arithemtic:
    //   group 0:
    //   - add   0 1      -> 222
    //   - sub   1 0      -> 200
    //   - sub   0 1      -> -200
    //   - mul   0 1      -> 2321
    //
    //   group 1:
    //   - div_s 1 2      -> -16
    //   - div_u 1 2      -> 0
    //   - div_s 2 1      -> 0
    //   - div_u 2 1      -> 20355295 (= 4294967283/211)
    //   - rem_s 1 2      -> 3
    //   - rem_u 2 1      -> 38
    //
    //   group 2:
    //   - inc   0 amount:3     -> 14
    //   - dec   0 amount:3     -> 8
    //   - inc   2 amount:3     -> -10
    //   - dec   2 amount:3     -> -16
    //
    //   group 3:
    //   - add 0xffff_ffff 0x2  -> 0x1                  // -1 + 2 = 1
    //   - mul 0xf0e0_d0c0 0x2  -> 0xf0e0_d0c0 << 1
    //   - inc 0xffff_ffff 0x2  -> 0x1
    //   - dec 0x1         0x2  -> 0xffff_ffff
    //
    // (i32 i32 i32) -> (i32 i32 i32 i32  i32 i32 i32 i32 i32 i32  i32 i32 i32 i32  i32 i32 i32 i32)

    // note of the 'remainder':
    // (211 % -13) = 3
    //  ^      ^
    //  |      |divisor
    //  |dividend <--------- the result always takes the sign of the dividend.

    let binary0 = helper_make_single_module_app(
        r#"
        fn test
            (a0:i32, a1:i32, a2:i32)
            ->
            (
            i32, i32, i32, i32
            i32, i32, i32, i32, i32, i32
            i32, i32, i32, i32
            i32, i32, i32, i32)
            {
                // group 0
                add_i32(local_load_i32_s(a0), local_load_i32_s(a1))
                sub_i32(local_load_i32_s(a1), local_load_i32_s(a0))
                sub_i32(local_load_i32_s(a0), local_load_i32_s(a1))
                mul_i32(local_load_i32_s(a0), local_load_i32_s(a1))

                // group 1
                div_i32_s(local_load_i32_s(a1), local_load_i32_s(a2))
                div_i32_u(local_load_i32_s(a1), local_load_i32_s(a2))
                div_i32_s(local_load_i32_s(a2), local_load_i32_s(a1))
                div_i32_u(local_load_i32_s(a2), local_load_i32_s(a1))
                rem_i32_s(local_load_i32_s(a1), local_load_i32_s(a2))
                rem_i32_u(local_load_i32_s(a2), local_load_i32_s(a1))

                // group 2
                add_imm_i32(3, local_load_i32_s(a0))
                sub_imm_i32(3, local_load_i32_s(a0))
                add_imm_i32(3, local_load_i32_s(a2))
                sub_imm_i32(3, local_load_i32_s(a2))

                // group 3
                add_i32(imm_i32(0xffff_ffff), imm_i32(0x2))
                mul_i32(imm_i32(0xf0e0_d0c0), imm_i32(0x2))
                add_imm_i32(2, imm_i32(0xffff_ffff))
                sub_imm_i32(2, imm_i32(0x1))
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
            ForeignValue::U32(11),
            ForeignValue::U32(211),
            ForeignValue::U32(-13i32 as u32),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::U32(222),
            ForeignValue::U32(200),
            ForeignValue::U32(-200i32 as u32),
            ForeignValue::U32(2321),
            // group 1
            ForeignValue::U32(-16i32 as u32),
            ForeignValue::U32(0),
            ForeignValue::U32(0),
            ForeignValue::U32(20355295),
            ForeignValue::U32(3),
            ForeignValue::U32(38),
            // group 2
            ForeignValue::U32(14),
            ForeignValue::U32(8),
            ForeignValue::U32(-10i32 as u32),
            ForeignValue::U32(-16i32 as u32),
            // group 3
            ForeignValue::U32(0x1),
            ForeignValue::U32(0xf0e0_d0c0 << 1),
            ForeignValue::U32(0x1),
            ForeignValue::U32(0xffff_ffff),
        ]
    );
}

// #[test]
// fn test_assemble_arithmetic_i64() {
//     // numbers:
//     //   - 0: 11
//     //   - 1: 211
//     //   - 2: -13
//
//     // arithemtic:
//     //   group 0:
//     //   - add   0 1      -> 222
//     //   - sub   1 0      -> 200
//     //   - sub   0 1      -> -200
//     //   - mul   0 1      -> 2321
//     //
//     //   group 1:
//     //   - div_s 1 2      -> -16
//     //   - div_u 1 2      -> 0
//     //   - div_s 2 1      -> 0
//     //   - div_u 2 1      -> 87425327363552377 (= 18446744073709551603/211)
//     //   - rem_s 1 2      -> 3
//     //   - rem_u 2 1      -> 56
//     //
//     //   group 2:
//     //   - inc   0 amount:3     -> 14
//     //   - dec   0 amount:3     -> 8
//     //   - inc   2 amount:3     -> -10
//     //   - dec   2 amount:3     -> -16
//     //
//     //   group 3:
//     //   - add 0xffff_ffff_ffff_ffff 0x2  -> 0x1    // -1 + 2 = 1
//     //   - mul 0xf0e0_d0c0_b0a0_9080 0x2  -> 0xf0e0_d0c0_b0a0_9080 << 1
//     //   - inc 0xffff_ffff_ffff_ffff 0x2  -> 0x1
//     //   - dec 0x1                   0x2  -> 0xffff_ffff_ffff_ffff
//     //
//     // (i64 i64 i64) -> (i64 i64 i64 i64  i64 i64 i64 i64 i64 i64  i64 i64 i64 i64  i64 i64 i64 i64)
//
//     // note of the 'remainder':
//     // (211 % -13) = 3
//     //  ^      ^
//     //  |      |divisor
//     //  |dividend <--------- the result always takes the sign of the dividend.
//
//     let module_binary = helper_generate_module_image_binary_from_str(
//         r#"
//         (module $app
//             (runtime_version "1.0")
//             (function $test
//                 (param $a0 i64)
//                 (param $a1 i64)
//                 (param $a2 i64)
//                 (results
//                     i64 i64 i64 i64
//                     i64 i64 i64 i64 i64 i64
//                     i64 i64 i64 i64
//                     i64 i64 i64 i64)
//                 (code
//                     // group 0
//                     (i64.add (local.load64_i64 $a0) (local.load64_i64 $a1))
//                     (i64.sub (local.load64_i64 $a1) (local.load64_i64 $a0))
//                     (i64.sub (local.load64_i64 $a0) (local.load64_i64 $a1))
//                     (i64.mul (local.load64_i64 $a0) (local.load64_i64 $a1))
//
//                     // group 1
//                     (i64.div_s (local.load64_i64 $a1) (local.load64_i64 $a2))
//                     (i64.div_u (local.load64_i64 $a1) (local.load64_i64 $a2))
//                     (i64.div_s (local.load64_i64 $a2) (local.load64_i64 $a1))
//                     (i64.div_u (local.load64_i64 $a2) (local.load64_i64 $a1))
//                     (i64.rem_s (local.load64_i64 $a1) (local.load64_i64 $a2))
//                     (i64.rem_u (local.load64_i64 $a2) (local.load64_i64 $a1))
//
//                     // group 2
//                     (i64.inc (local.load64_i64 $a0) 3)
//                     (i64.dec (local.load64_i64 $a0) 3)
//                     (i64.inc (local.load64_i64 $a2) 3)
//                     (i64.dec (local.load64_i64 $a2) 3)
//
//                     // group 3
//                     (i64.add (i64.imm 0xffff_ffff_ffff_ffff) (i64.imm 0x2))
//                     (i64.mul (i64.imm 0xf0e0_d0c0_b0a0_9080) (i64.imm 0x2))
//                     (i64.inc (i64.imm 0xffff_ffff_ffff_ffff) 2)
//                     (i64.dec (i64.imm 0x1) 2)
//                 )
//             )
//         )
//         "#,
//     );
//
//     let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
//     let process_context0 = program_resource0.create_process_context().unwrap();
//     let mut thread_context0 = process_context0.create_thread_context();
//
//     let result0 = process_function(
//         &mut thread_context0,
//         0,
//         0,
//         &[
//             ForeignValue::U64(11),
//             ForeignValue::U64(211),
//             ForeignValue::U64(-13i64 as u64),
//         ],
//     );
//     assert_eq!(
//         result0.unwrap(),
//         vec![
//             // group 0
//             ForeignValue::U64(222),
//             ForeignValue::U64(200),
//             ForeignValue::U64(-200_i64 as u64),
//             ForeignValue::U64(2321),
//             // group 1
//             ForeignValue::U64(-16i64 as u64),
//             ForeignValue::U64(0),
//             ForeignValue::U64(0),
//             ForeignValue::U64(87425327363552377),
//             ForeignValue::U64(3),
//             ForeignValue::U64(56),
//             // group 2
//             ForeignValue::U64(14),
//             ForeignValue::U64(8),
//             ForeignValue::U64(-10i64 as u64),
//             ForeignValue::U64(-16i64 as u64),
//             // group 3
//             ForeignValue::U64(0x1),
//             ForeignValue::U64(0xf0e0_d0c0_b0a0_9080 << 1),
//             ForeignValue::U64(0x1),
//             ForeignValue::U64(0xffff_ffff_ffff_ffff),
//         ]
//     );
// }
//
// #[test]
// fn test_assemble_arithmetic_f32() {
//     // numbers:
//     //   - 0: 1.414
//     //   - 1: 4.123
//
//     // arithemtic:
//     //   - add 0 1      -> 5.537
//     //   - sub 1 0      -> 2.709
//     //   - mul 0 1      -> 5.829922
//     //   - div 1 0      -> 2.91584158416
//
//     let module_binary = helper_generate_module_image_binary_from_str(
//         r#"
//         (module $app
//             (runtime_version "1.0")
//             (function $test
//                 (param $a0 f32)
//                 (param $a1 f32)
//                 (results
//                     f32 f32 f32 f32)
//                 (code
//                     (f32.add (local.load32_f32 $a0) (local.load32_f32 $a1))
//                     (f32.sub (local.load32_f32 $a1) (local.load32_f32 $a0))
//                     (f32.mul (local.load32_f32 $a0) (local.load32_f32 $a1))
//                     (f32.div (local.load32_f32 $a1) (local.load32_f32 $a0))
//                 )
//             )
//         )
//         "#,
//     );
//
//     let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
//     let process_context0 = program_resource0.create_process_context().unwrap();
//     let mut thread_context0 = process_context0.create_thread_context();
//
//     let result0 = process_function(
//         &mut thread_context0,
//         0,
//         0,
//         &[ForeignValue::F32(1.414), ForeignValue::F32(4.123)],
//     );
//     assert_eq!(
//         result0.unwrap(),
//         vec![
//             ForeignValue::F32(5.537),
//             ForeignValue::F32(2.709),
//             ForeignValue::F32(5.829922),
//             ForeignValue::F32(2.915_841_6),
//         ]
//     );
// }
//
// #[test]
// fn test_assemble_arithmetic_f64() {
//     // numbers:
//     //   - 0: 1.414
//     //   - 1: 4.123
//
//     // arithemtic:
//     //   - add 0 1      -> 5.537
//     //   - sub 1 0      -> 2.709
//     //   - mul 0 1      -> 5.829922
//     //   - div 1 0      -> 2.91584158416
//     //
//     // (f64 f64) -> (f64 f64 f64 f64)
//
//     let module_binary = helper_generate_module_image_binary_from_str(
//         r#"
//         (module $app
//             (runtime_version "1.0")
//             (function $test
//                 (param $a0 f64)
//                 (param $a1 f64)
//                 (results
//                     f64 f64 f64 f64)
//                 (code
//                     (f64.add (local.load64_f64 $a0) (local.load64_f64 $a1))
//                     (f64.sub (local.load64_f64 $a1) (local.load64_f64 $a0))
//                     (f64.mul (local.load64_f64 $a0) (local.load64_f64 $a1))
//                     (f64.div (local.load64_f64 $a1) (local.load64_f64 $a0))
//                 )
//             )
//         )
//         "#,
//     );
//
//     let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
//     let process_context0 = program_resource0.create_process_context().unwrap();
//     let mut thread_context0 = process_context0.create_thread_context();
//
//     let result0 = process_function(
//         &mut thread_context0,
//         0,
//         0,
//         &[ForeignValue::F64(1.414), ForeignValue::F64(4.123)],
//     );
//     assert_eq!(
//         result0.unwrap(),
//         vec![
//             ForeignValue::F64(5.537),
//             ForeignValue::F64(2.7090000000000005),
//             ForeignValue::F64(5.829922),
//             ForeignValue::F64(2.915841584158416),
//         ]
//     );
// }
