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
fn test_assemble_conversion_extend_and_truncate() {
    // (i64, i32)  ->  (i64, i64, i32)
    //  |    |          ^    ^    ^
    //  |    | extend   |    |    |
    //  |    \----------/----/    |
    //  \-------------------------/ truncate

    let binary0 = helper_make_single_module_app(
        r#"
        fn test (a0:i64, a1:i32) -> (i64, i64, i32) {
            extend_i32_s_to_i64(local_load_i32_s(a1))
            extend_i32_u_to_i64(local_load_i32_s(a1))
            truncate_i64_to_i32(local_load_i32_s(a0))
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

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(a0:f64, a1:f32) -> (f64, f32) {
            promote_f32_to_f64 (local_load_f32(a1))
            demote_f64_to_f32 (local_load_f64(a0))
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

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(a0:f32, a1:f64, a2:f32, a3:f64) ->
            (
            i32, i32, i64, i64
            i32, i32, i64, i64
            i32, i32, i64, i64
            i32, i32, i64, i64
            )
        {
            // group 0
            convert_f32_to_i32_s(local_load_f32(a0))
            convert_f32_to_i32_u(local_load_f32(a0))
            convert_f32_to_i64_s(local_load_f32(a0))
            convert_f32_to_i64_u(local_load_f32(a0))

            // group 1
            convert_f64_to_i32_s(local_load_f64(a1))
            convert_f64_to_i32_u(local_load_f64(a1))
            convert_f64_to_i64_s(local_load_f64(a1))
            convert_f64_to_i64_u(local_load_f64(a1))

            // group 2
            convert_f32_to_i32_s(local_load_f32(a2))
            convert_f32_to_i32_u(local_load_f32(a2))
            convert_f32_to_i64_s(local_load_f32(a2))
            convert_f32_to_i64_u(local_load_f32(a2))

            // group 3
            convert_f64_to_i32_s(local_load_f64(a3))
            convert_f64_to_i32_u(local_load_f64(a3))
            convert_f64_to_i64_s(local_load_f64(a3))
            convert_f64_to_i64_u(local_load_f64(a3))
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

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(a0:i32, a1:i64, a2:i32, a3:i64) ->
            (
            f32, f32, f64, f64
            f32, f32, f64, f64
            f32, f32, f64, f64
            f32, f32, f64, f64
            )
        {
            // group 0
            convert_i32_s_to_f32 (local_load_i32_s(a0))
            convert_i32_u_to_f32 (local_load_i32_s(a0))
            convert_i32_s_to_f64 (local_load_i32_s(a0))
            convert_i32_u_to_f64 (local_load_i32_s(a0))

            // group 1
            convert_i64_s_to_f32 (local_load_i64(a1))
            convert_i64_u_to_f32 (local_load_i64(a1))
            convert_i64_s_to_f64 (local_load_i64(a1))
            convert_i64_u_to_f64 (local_load_i64(a1))

            // group 2
            convert_i32_s_to_f32 (local_load_i32_s(a2))
            convert_i32_u_to_f32 (local_load_i32_s(a2))
            convert_i32_s_to_f64 (local_load_i32_s(a2))
            convert_i32_u_to_f64 (local_load_i32_s(a2))

            // group 3
            convert_i64_s_to_f32 (local_load_i64(a3))
            convert_i64_u_to_f32 (local_load_i64(a3))
            convert_i64_s_to_f64 (local_load_i64(a3))
            convert_i64_u_to_f64 (local_load_i64(a3))
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
