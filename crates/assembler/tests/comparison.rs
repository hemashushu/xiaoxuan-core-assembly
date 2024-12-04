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
fn test_assemble_comparison_i32() {
    // numbers:
    //   - 0: 0
    //   - 1: 11
    //   - 2: 13
    //   - 3: -7
    // comparison:
    //   group 0:
    //   - eqz  0         -> 1
    //   - eqz  1         -> 0
    //   - nez  0         -> 0
    //   - nez  1         -> 1
    //
    //   group 1:
    //   - eq   1  2      -> 0
    //   - ne   1  2      -> 1
    //   - eq   1  1      -> 1
    //   - ne   1  1      -> 0
    //
    //   group 2:
    //   - lt_s 2  3      -> 0
    //   - lt_u 2  3      -> 1
    //   - gt_s 2  3      -> 1
    //   - gt_u 2  3      -> 0
    //
    //   group 3:
    //   - le_s 2  1      -> 0
    //   - le_u 2  1      -> 0
    //   - le_s 1  1      -> 1
    //   - le_u 1  1      -> 1
    //
    //   group 4:
    //   - ge_s 1  2      -> 0
    //   - ge_u 1  2      -> 0
    //   - ge_s 1  1      -> 1
    //   - ge_u 1  1      -> 1
    //
    // (i32 i32 i32 i32) -> (i32 i32 i32 i32  i32 i32 i32 i32  i32 i32 i32 i32  i32 i32 i32 i32)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(a0: i32, a1: i32, a2: i32, a3: i32) ->
            (
            i32, i32, i32, i32
            i32, i32, i32, i32
            i32, i32, i32, i32
            i32, i32, i32, i32
            i32, i32, i32, i32
            )
        {
            // group 0
            eqz_i32(local_load_i32_s(a0))
            eqz_i32(local_load_i32_s(a1))
            nez_i32(local_load_i32_s(a0))
            nez_i32(local_load_i32_s(a1))

            // group 1
            eq_i32(local_load_i32_s(a1), local_load_i32_s(a2))
            ne_i32(local_load_i32_s(a1), local_load_i32_s(a2))
            eq_i32(local_load_i32_s(a1), local_load_i32_s(a1))
            ne_i32(local_load_i32_s(a1), local_load_i32_s(a1))

            // group 2
            lt_i32_s(local_load_i32_s(a2), local_load_i32_s(a3))
            lt_i32_u(local_load_i32_s(a2), local_load_i32_s(a3))
            gt_i32_s(local_load_i32_s(a2), local_load_i32_s(a3))
            gt_i32_u(local_load_i32_s(a2), local_load_i32_s(a3))

            // group 3
            le_i32_s (local_load_i32_s(a2), local_load_i32_s(a1))
            le_i32_u (local_load_i32_s(a2), local_load_i32_s(a1))
            le_i32_s (local_load_i32_s(a1), local_load_i32_s(a1))
            le_i32_u (local_load_i32_s(a1), local_load_i32_s(a1))

            // group 4
            ge_i32_s(local_load_i32_s(a1), local_load_i32_s(a2))
            ge_i32_u(local_load_i32_s(a1), local_load_i32_s(a2))
            ge_i32_s(local_load_i32_s(a1), local_load_i32_s(a1))
            ge_i32_u(local_load_i32_s(a1), local_load_i32_s(a1))
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
            ForeignValue::U32(0),
            ForeignValue::U32(11),
            ForeignValue::U32(13),
            ForeignValue::U32(-7i32 as u32),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::U32(1),
            ForeignValue::U32(0),
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            // group 1
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            ForeignValue::U32(1),
            ForeignValue::U32(0),
            // group 2
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            ForeignValue::U32(1),
            ForeignValue::U32(0),
            // group 3
            ForeignValue::U32(0),
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            ForeignValue::U32(1),
            // group 4
            ForeignValue::U32(0),
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            ForeignValue::U32(1),
        ]
    );
}

#[test]
fn test_assemble_comparison_i64() {
    // numbers:
    //   - 0: 0
    //   - 1: 11
    //   - 2: 13
    //   - 3: -7
    // comparison:
    //   group 0:
    //   - eqz  0         -> 1
    //   - eqz  1         -> 0
    //   - nez  0         -> 0
    //   - nez  1         -> 1
    //
    //   group 1:
    //   - eq   1  2      -> 0
    //   - ne   1  2      -> 1
    //   - eq   1  1      -> 1
    //   - ne   1  1      -> 0
    //
    //   group 2:
    //   - lt_s 2  3      -> 0
    //   - lt_u 2  3      -> 1
    //   - gt_s 2  3      -> 1
    //   - gt_u 2  3      -> 0
    //
    //   group 3:
    //   - le_s 2  1      -> 0
    //   - le_u 2  1      -> 0
    //   - le_s 1  1      -> 1
    //   - le_u 1  1      -> 1
    //
    //   group 4:
    //   - ge_s 1  2      -> 0
    //   - ge_u 1  2      -> 0
    //   - ge_s 1  1      -> 1
    //   - ge_u 1  1      -> 1
    //
    // (i64 i64 i64 i64) -> (i32 i32 i32 i32  i32 i32 i32 i32  i32 i32 i32 i32  i32 i32 i32 i32)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(a0:i64, a1:i64, a2:i64, a3:i64) ->
            (
            i32, i32, i32, i32
            i32, i32, i32, i32
            i32, i32, i32, i32
            i32, i32, i32, i32
            i32, i32, i32, i32
            )
        {
            // group 0
            eqz_i64 (local_load_i64(a0))
            eqz_i64 (local_load_i64(a1))
            nez_i64 (local_load_i64(a0))
            nez_i64 (local_load_i64(a1))

            // group 1
            eq_i64 (local_load_i64(a1), local_load_i64(a2))
            ne_i64 (local_load_i64(a1), local_load_i64(a2))
            eq_i64 (local_load_i64(a1), local_load_i64(a1))
            ne_i64 (local_load_i64(a1), local_load_i64(a1))

            // group 2
            lt_i64_s (local_load_i64(a2), local_load_i64(a3))
            lt_i64_u (local_load_i64(a2), local_load_i64(a3))
            gt_i64_s (local_load_i64(a2), local_load_i64(a3))
            gt_i64_u (local_load_i64(a2), local_load_i64(a3))

            // group 3
            le_i64_s (local_load_i64(a2), local_load_i64(a1))
            le_i64_u (local_load_i64(a2), local_load_i64(a1))
            le_i64_s (local_load_i64(a1), local_load_i64(a1))
            le_i64_u (local_load_i64(a1), local_load_i64(a1))

            // group 4
            ge_i64_s (local_load_i64(a1), local_load_i64(a2))
            ge_i64_u (local_load_i64(a1), local_load_i64(a2))
            ge_i64_s (local_load_i64(a1), local_load_i64(a1))
            ge_i64_u (local_load_i64(a1), local_load_i64(a1))
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
            ForeignValue::U64(0),
            ForeignValue::U64(11),
            ForeignValue::U64(13),
            ForeignValue::U64(-7i64 as u64),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::U32(1),
            ForeignValue::U32(0),
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            // group 1
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            ForeignValue::U32(1),
            ForeignValue::U32(0),
            // group 2
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            ForeignValue::U32(1),
            ForeignValue::U32(0),
            // group 3
            ForeignValue::U32(0),
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            ForeignValue::U32(1),
            // group 4
            ForeignValue::U32(0),
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            ForeignValue::U32(1),
        ]
    );
}

#[test]
fn test_assemble_comparison_f32() {
    // numbers:
    //   - 0: 1.414
    //   - 1: 1.732
    // comparison:
    //   group 0:
    //   - eq 0  1        -> 0
    //   - ne 0  1        -> 1
    //   - eq 0  0        -> 1
    //   - ne 0  0        -> 0
    //
    //   group 1:
    //   - lt 0  1        -> 1
    //   - lt 1  0        -> 0
    //   - lt 0  0        -> 0
    //   - gt 0  1        -> 0
    //   - gt 1  0        -> 1
    //   - gt 0  0        -> 0
    //
    //   group 2:
    //   - le 1  0        -> 0
    //   - le 0  0        -> 1
    //   - ge 0  1        -> 0
    //   - ge 0  0        -> 1
    //
    // (f32 f32) -> (i32 i32 i32 i32  i32 i32 i32 i32 i32 i32  i32 i32 i32 i32)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(a0: f32,a1: f32) ->
            (
            i32, i32, i32, i32
            i32, i32, i32, i32, i32, i32
            i32, i32, i32, i32
            )
        {
            // group 0
            eq_f32 (local_load_f32(a0), local_load_f32(a1))
            ne_f32 (local_load_f32(a0), local_load_f32(a1))
            eq_f32 (local_load_f32(a0), local_load_f32(a0))
            ne_f32 (local_load_f32(a0), local_load_f32(a0))

            // group 1
            lt_f32 (local_load_f32(a0), local_load_f32(a1))
            lt_f32 (local_load_f32(a1), local_load_f32(a0))
            lt_f32 (local_load_f32(a0), local_load_f32(a0))
            gt_f32 (local_load_f32(a0), local_load_f32(a1))
            gt_f32 (local_load_f32(a1), local_load_f32(a0))
            gt_f32 (local_load_f32(a0), local_load_f32(a0))

            // group 2
            le_f32 (local_load_f32(a1), local_load_f32(a0))
            le_f32 (local_load_f32(a0), local_load_f32(a0))
            ge_f32 (local_load_f32(a0), local_load_f32(a1))
            ge_f32 (local_load_f32(a0), local_load_f32(a0))
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
        &[ForeignValue::F32(1.414f32), ForeignValue::F32(1.732f32)],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            ForeignValue::U32(1),
            ForeignValue::U32(0),
            // group 1
            ForeignValue::U32(1),
            ForeignValue::U32(0),
            ForeignValue::U32(0),
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            ForeignValue::U32(0),
            // group 2
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            ForeignValue::U32(0),
            ForeignValue::U32(1),
        ]
    );
}

#[test]
fn test_assemble_comparison_f64() {
    // numbers:
    //   - 0: 1.414
    //   - 1: 1.732
    // comparison:
    //   group 0:
    //   - eq 0  1        -> 0
    //   - ne 0  1        -> 1
    //   - eq 0  0        -> 1
    //   - ne 0  0        -> 0
    //
    //   group 1:
    //   - lt 0  1        -> 1
    //   - lt 1  0        -> 0
    //   - lt 0  0        -> 0
    //   - gt 0  1        -> 0
    //   - gt 1  0        -> 1
    //   - gt 0  0        -> 0
    //
    //   group 2:
    //   - le 1  0        -> 0
    //   - le 0  0        -> 1
    //   - ge 0  1        -> 0
    //   - ge 0  0        -> 1
    //
    // (f32 f32) -> (i32 i32 i32 i32  i32 i32 i32 i32 i32 i32  i32 i32 i32 i32)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(a0:f64, a1:f64) ->
            (
            i32, i32, i32, i32
            i32, i32, i32, i32, i32, i32
            i32, i32, i32, i32
            )
        {
            // group 0
            eq_f64 (local_load_f64(a0), local_load_f64(a1))
            ne_f64 (local_load_f64(a0), local_load_f64(a1))
            eq_f64 (local_load_f64(a0), local_load_f64(a0))
            ne_f64 (local_load_f64(a0), local_load_f64(a0))

            // group 1
            lt_f64 (local_load_f64(a0), local_load_f64(a1))
            lt_f64 (local_load_f64(a1), local_load_f64(a0))
            lt_f64 (local_load_f64(a0), local_load_f64(a0))
            gt_f64 (local_load_f64(a0), local_load_f64(a1))
            gt_f64 (local_load_f64(a1), local_load_f64(a0))
            gt_f64 (local_load_f64(a0), local_load_f64(a0))

            // group 2
            le_f64 (local_load_f64(a1), local_load_f64(a0))
            le_f64 (local_load_f64(a0), local_load_f64(a0))
            ge_f64 (local_load_f64(a0), local_load_f64(a1))
            ge_f64 (local_load_f64(a0), local_load_f64(a0))
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
        &[ForeignValue::F64(1.414f64), ForeignValue::F64(1.732f64)],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            ForeignValue::U32(1),
            ForeignValue::U32(0),
            // group 1
            ForeignValue::U32(1),
            ForeignValue::U32(0),
            ForeignValue::U32(0),
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            ForeignValue::U32(0),
            // group 2
            ForeignValue::U32(0),
            ForeignValue::U32(1),
            ForeignValue::U32(0),
            ForeignValue::U32(1),
        ]
    );
}
