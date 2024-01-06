// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancasm_assembler::utils::helper_generate_module_image_binary_from_str;
use ancvm_context::program_resource::ProgramResource;
use ancvm_processor::{
    in_memory_program_resource::InMemoryProgramResource, interpreter::process_function,
};
use ancvm_types::ForeignValue;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_conversion_extend_and_truncate() {
    // (i64, i32)  ->  (i64, i64, i32)
    //  |    |          ^    ^    ^
    //  |    | extend   |    |    |
    //  |    \----------/----/    |
    //  \-------------------------/ truncate

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test
                (param $a0 i64)
                (param $a1 i32)
                (results i64 i64 i32)
                (code
                    (i64.extend_i32_s (local.load32_i32 $a1))
                    (i64.extend_i32_u (local.load32_i32 $a1))
                    (i32.truncate_i64 (local.load32_i32 $a0))
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
            ForeignValue::U64(0x19171311_07050302u64),
            ForeignValue::U32(0x80706050u32),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::U64(0xffffffff_80706050u64),
            ForeignValue::U64(0x00000000_80706050u64),
            ForeignValue::U32(0x07050302u32),
        ]
    );
}

#[test]
fn test_assemble_conversion_demote_and_promote() {
    // (f64, f32)  ->  (f64, f32)
    //  |    |          ^    ^
    //  |    | promote  |    |
    //  |    \----------/    |
    //  \--------------------/ demote

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test
                (param $a0 f64)
                (param $a1 f32)
                (results f64 f32)
                (code
                    (f64.promote_f32 (local.load32_f32 $a1))
                    (f32.demote_f64 (local.load64_f64 $a0))
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
            ForeignValue::F64(std::f64::consts::PI),
            ForeignValue::F32(std::f32::consts::E),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::F64(std::f32::consts::E as f64),
            ForeignValue::F32(std::f64::consts::PI as f32),
        ]
    );
}

#[test]
fn test_assemble_conversion_float_to_int() {
    // (f32,              f64,            -f32,             -f64)
    //  |                 |                |                 |
    //  |                 |                |                 |
    //  |                 |                |                 |
    //  |---\---\---\     |---\---\---\    |---\---\---\     |---\---\---\
    //  |   |   |   |     |   |   |   |    |   |   |   |     |   |   |   |
    //  v   v   v   v     v   v   v   v    v   v   v   v     v   v   v   v
    // (i32 i32 i64 i64   i32 i32 i64 i64  i32 i32 i64 i64   i32 i32 i64 i64)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test
                (param $a0 f32)
                (param $a1 f64)
                (param $a2 f32)
                (param $a3 f64)
                (results
                    i32 i32 i64 i64
                    i32 i32 i64 i64
                    i32 i32 i64 i64
                    i32 i32 i64 i64)
                (code
                    ;; group 0
                    (i32.convert_f32_s (local.load32_f32 $a0))
                    (i32.convert_f32_u (local.load32_f32 $a0))
                    (i64.convert_f32_s (local.load32_f32 $a0))
                    (i64.convert_f32_u (local.load32_f32 $a0))
                    ;; group 1
                    (i32.convert_f64_s (local.load64_f64 $a1))
                    (i32.convert_f64_u (local.load64_f64 $a1))
                    (i64.convert_f64_s (local.load64_f64 $a1))
                    (i64.convert_f64_u (local.load64_f64 $a1))
                    ;; group 2
                    (i32.convert_f32_s (local.load32_f32 $a2))
                    (i32.convert_f32_u (local.load32_f32 $a2))
                    (i64.convert_f32_s (local.load32_f32 $a2))
                    (i64.convert_f32_u (local.load32_f32 $a2))
                    ;; group 3
                    (i32.convert_f64_s (local.load64_f64 $a3))
                    (i32.convert_f64_u (local.load64_f64 $a3))
                    (i64.convert_f64_s (local.load64_f64 $a3))
                    (i64.convert_f64_u (local.load64_f64 $a3))
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
            ForeignValue::F32(2.236),
            ForeignValue::F64(3.162),
            ForeignValue::F32(-5.099),
            ForeignValue::F64(-7.071),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::U32(2),
            ForeignValue::U32(2),
            ForeignValue::U64(2),
            ForeignValue::U64(2),
            // group 1
            ForeignValue::U32(3),
            ForeignValue::U32(3),
            ForeignValue::U64(3),
            ForeignValue::U64(3),
            // group 2
            ForeignValue::U32(-5i32 as u32),
            ForeignValue::U32(0),
            ForeignValue::U64(-5i64 as u64),
            ForeignValue::U64(0),
            // group 3
            ForeignValue::U32(-7i32 as u32),
            ForeignValue::U32(0),
            ForeignValue::U64(-7i64 as u64),
            ForeignValue::U64(0),
        ]
    );
}

#[test]
fn test_assemble_conversion_int_to_float() {
    // (i32,              i64,            -i32,             -i64)
    //  |                 |                |                 |
    //  |                 |                |                 |
    //  |                 |                |                 |
    //  |---\---\---\     |---\---\---\    |---\---\---\     |---\---\---\
    //  |   |   |   |     |   |   |   |    |   |   |   |     |   |   |   |
    //  v   v   v   v     v   v   v   v    v   v   v   v     v   v   v   v
    // (f32 f32 f64 f64   f32 f32 f64 f64  f32 f32 f64 f64   f32 f32 f64 f64)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test
                (param $a0 i32)
                (param $a1 i64)
                (param $a2 i32)
                (param $a3 i64)
                (results
                    f32 f32 f64 f64
                    f32 f32 f64 f64
                    f32 f32 f64 f64
                    f32 f32 f64 f64)
                (code
                    ;; group 0
                    (f32.convert_i32_s (local.load32_i32 $a0))
                    (f32.convert_i32_u (local.load32_i32 $a0))
                    (f64.convert_i32_s (local.load32_i32 $a0))
                    (f64.convert_i32_u (local.load32_i32 $a0))

                    ;; group 1
                    (f32.convert_i64_s (local.load64_i64 $a1))
                    (f32.convert_i64_u (local.load64_i64 $a1))
                    (f64.convert_i64_s (local.load64_i64 $a1))
                    (f64.convert_i64_u (local.load64_i64 $a1))

                    ;; group 2
                    (f32.convert_i32_s (local.load32_i32 $a2))
                    (f32.convert_i32_u (local.load32_i32 $a2))
                    (f64.convert_i32_s (local.load32_i32 $a2))
                    (f64.convert_i32_u (local.load32_i32 $a2))

                    ;; group 3
                    (f32.convert_i64_s (local.load64_i64 $a3))
                    (f32.convert_i64_u (local.load64_i64 $a3))
                    (f64.convert_i64_s (local.load64_i64 $a3))
                    (f64.convert_i64_u (local.load64_i64 $a3))
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
            ForeignValue::U32(11),
            ForeignValue::U64(13),
            ForeignValue::U32(-17i32 as u32),
            ForeignValue::U64(-19i64 as u64),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::F32(11.0),
            ForeignValue::F32(11.0),
            ForeignValue::F64(11.0),
            ForeignValue::F64(11.0),
            // group 1
            ForeignValue::F32(13.0),
            ForeignValue::F32(13.0),
            ForeignValue::F64(13.0),
            ForeignValue::F64(13.0),
            // group 2
            ForeignValue::F32(-17.0),
            ForeignValue::F32(-17i32 as u32 as f32),
            ForeignValue::F64(-17.0),
            ForeignValue::F64(-17i32 as u32 as f64),
            // group 3
            ForeignValue::F32(-19.0),
            ForeignValue::F32(-19i64 as u64 as f32),
            ForeignValue::F64(-19.0),
            ForeignValue::F64(-19i64 as u64 as f64),
        ]
    );
}
