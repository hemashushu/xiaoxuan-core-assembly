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
fn test_assemble_fundamental_nop() {
    // () -> (i32)
    let binary0 = helper_make_single_module_app(
        r#"
        fn test()->() nop()
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    assert!(result0.is_ok());
}

/*
#[test]
fn test_assemble_fundamental_zero() {
    // () -> (i32)
    let binary0 = helper_make_single_module_app(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (result i32)
                (code
                    zero
                )
            )
        )
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(0)]);
}

#[test]
fn test_assemble_fundamental_drop() {
    // () -> (i32)
    let binary0 = helper_make_single_module_app(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (result i32)
                (code
                    (i32.imm 13)
                    (drop
                        (i32.imm 17)
                    )
                )
            )
        )
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(13)]);
}

#[test]
fn test_assemble_fundamental_duplicate() {
    // () -> (i32, i32)
    let binary0 = helper_make_single_module_app(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (results i32 i32)
                (code
                    (duplicate
                        (i32.imm 19)
                    )
                )
            )
        )
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![ForeignValue::U32(19), ForeignValue::U32(19)]
    );
}

#[test]
fn test_assemble_fundamental_swap() {
    // () -> (i32, i32)
    let binary0 = helper_make_single_module_app(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (results i32 i32)
                (code
                    (swap
                        (i32.imm 211)
                        (i32.imm 223)
                    )
                )
            )
        )
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![ForeignValue::U32(223), ForeignValue::U32(211)]
    );
}

#[test]
fn test_assemble_fundamental_select_nez_false() {
    // () -> (i32)
    let binary0 = helper_make_single_module_app(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (result i32)
                (code
                    (select_nez
                        (i32.imm 11)    // when true
                        (i32.imm 13)    // when false
                        zero            // test
                    )
                )
            )
        )
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(13)]);
}

#[test]
fn test_assemble_fundamental_select_nez_true() {
    // () -> (i32)
    let binary0 = helper_make_single_module_app(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (result i32)
                (code
                    (select_nez
                        (i32.imm 11)    // when true
                        (i32.imm 13)    // when false
                        (i32.imm 1)     // test
                    )
                )
            )
        )
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(11)]);
}
*/

#[test]
fn test_assemble_fundamental_immediate_integer() {
    // () -> (i32, i64, i32, i64)
    let binary0 = helper_make_single_module_app(
        r#"
        fn test ()->(i32, i64, i32, i64)
        {
            imm_i32(23)
            imm_i64(0x29313741_43475359_i64)
            imm_i32(0xffffff21)                 // -223
            imm_i64(0xffffffff_ffffff1d_i64)    // -227
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::U32(23),
            ForeignValue::U64(0x29313741_43475359u64),
            ForeignValue::U32((-223i32) as u32),
            ForeignValue::U64((-227i64) as u64)
        ]
    );
}

#[test]
fn test_assemble_fundamental_immediate_float() {
    // () -> (f32, f64, f32, f64)
    let binary0 = helper_make_single_module_app(
        r#"
        fn test()->(f32, f64, f32, f64) {
            imm_f32(3.14159265358979323846264338327950288_f32)      // Pi
            imm_f64(1.41421356237309504880168872420969808)          // sqrt(2)
            imm_f32(-2.71828182845904523536028747135266250_f32)     // -E
            imm_f64(-0.52359877559829887307710723054658381)         // -Pi/6
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::F32(std::f32::consts::PI),
            ForeignValue::F64(std::f64::consts::SQRT_2),
            ForeignValue::F32(-std::f32::consts::E),
            ForeignValue::F64(-std::f64::consts::FRAC_PI_6),
        ]
    );
}

#[test]
fn test_assemble_fundamental_immediate_float_hex() {
    // () -> (f32, f64, f32, f64)
    let binary0 = helper_make_single_module_app(
        r#"
        fn test () -> (f32, f64, f32, f64) {
            imm_f32(3.1415927_f32)
            imm_f64(2.718281828459045)
            imm_f32(0x1.921fb6p1_f32)
            imm_f64(0x1.5bf0a8b145769p1)
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::F32(std::f32::consts::PI),
            ForeignValue::F64(std::f64::consts::E),
            ForeignValue::F32(std::f32::consts::PI),
            ForeignValue::F64(std::f64::consts::E),
        ]
    );
}
