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
fn test_assemble_fundamental_nop() {
    // () -> (i32)
    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test
                (param $a i32)
                (result i32)
                (code
                    nop
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::U32(11)]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(11)]);
}

#[test]
fn test_assemble_fundamental_zero() {
    // () -> (i32)
    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test (result i32)
                (code
                    zero
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(0)]);
}

#[test]
fn test_assemble_fundamental_drop() {
    // () -> (i32)
    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
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

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(13)]);
}

/*
#[test]
fn test_assemble_fundamental_duplicate() {
    // () -> (i32, i32)
    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
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

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
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
    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
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

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![ForeignValue::U32(223), ForeignValue::U32(211)]
    );
}
*/

#[test]
fn test_assemble_fundamental_select_nez_false() {
    // () -> (i32)
    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
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

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(13)]);
}

#[test]
fn test_assemble_fundamental_select_nez_true() {
    // () -> (i32)
    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
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

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(11)]);
}

#[test]
fn test_assemble_fundamental_immediate_int() {
    // () -> (i32, i64, i32, i64)
    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test (results i32 i64 i32 i64)
                (code
                    (i32.imm 23)
                    (i64.imm 0x29313741_43475359)
                    (i32.imm 0xffffff21)            // -223
                    (i64.imm 0xffffffff_ffffff1d)   // -227
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
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
    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
            (module $app
                (compiler_version "1.0")
                (function $test (results f32 f64 f32 f64)
                    (code
                        (f32.imm 3.14159265358979323846264338327950288)     // Pi
                        (f64.imm 1.41421356237309504880168872420969808)     // sqrt(2)
                        (f32.imm -2.71828182845904523536028747135266250)    // -E
                        (f64.imm -0.52359877559829887307710723054658381)    // -Pi/6
                    )
                )
            )
            "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
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
    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
            (module $app
                (compiler_version "1.0")
                (function $test (results f32 f64 f32 f64)
                    (code
                        (f32.imm 3.1415927)
                        (f64.imm 2.718281828459045)
                        (f32.imm 0x1.921fb6p1)
                        (f64.imm 0x1.5bf0a8b145769p1)
                    )
                )
            )
            "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
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
