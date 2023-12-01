// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_assembler::utils::helper_generate_module_image_binaries_from_single_module_assembly;
use ancvm_program::program_source::ProgramSource;
use ancvm_process::{
    in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
};
use ancvm_types::ForeignValue;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_fundamental_nop() {
    // () -> (i32)
    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
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

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::U32(11)]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(11)]);
}

#[test]
fn test_assemble_fundamental_zero() {
    // () -> (i32)
    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
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

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(0)]);
}

#[test]
fn test_assemble_fundamental_drop() {
    // () -> (i32)
    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
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

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(13)]);
}

#[test]
fn test_assemble_fundamental_duplicate() {
    // () -> (i32, i32)
    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
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

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![ForeignValue::U32(19), ForeignValue::U32(19)]
    );
}

#[test]
fn test_assemble_fundamental_swap() {
    // () -> (i32, i32)
    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
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

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![ForeignValue::U32(223), ForeignValue::U32(211)]
    );
}

#[test]
fn test_assemble_fundamental_select_nez_false() {
    // () -> (i32)
    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (result i32)
                (code
                    (select_nez
                        (i32.imm 11)    ;; when true
                        (i32.imm 13)    ;; when false
                        zero            ;; test
                    )
                )
            )
        )
        "#,
    );

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(13)]);
}

#[test]
fn test_assemble_fundamental_select_nez_true() {
    // () -> (i32)
    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (result i32)
                (code
                    (select_nez
                        (i32.imm 11)    ;; when true
                        (i32.imm 13)    ;; when false
                        (i32.imm 1)     ;; test
                    )
                )
            )
        )
        "#,
    );

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(11)]);
}

#[test]
fn test_assemble_fundamental_immediate_int() {
    // () -> (i32, i64, i32, i64)
    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (results i32 i64 i32 i64)
                (code
                    (i32.imm 23)
                    (i64.imm 0x29313741_43475359)
                    (i32.imm 0xffffff21)            ;; -223
                    (i64.imm 0xffffffff_ffffff1d)   ;; -227
                )
            )
        )
        "#,
    );

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

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
    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
            (module $app
                (runtime_version "1.0")
                (function $test (results f32 f64 f32 f64)
                    (code
                        (f32.imm 3.14159265358979323846264338327950288)     ;; Pi
                        (f64.imm 1.41421356237309504880168872420969808)     ;; sqrt(2)
                        (f32.imm -2.71828182845904523536028747135266250)    ;; -E
                        (f64.imm -0.52359877559829887307710723054658381)    ;; -Pi/6
                    )
                )
            )
            "#,
    );

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

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
    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
            (module $app
                (runtime_version "1.0")
                (function $test (results f32 f64 f32 f64)
                    (code
                        (f32.imm 0x40490fdb)            ;; Pi
                        (f64.imm 0x3ff6a09e_667f3bcd)   ;; sqrt(2)
                        (f32.imm 0xc02df854)            ;; -E
                        (f64.imm 0xbfe0c152_382d7366)   ;; -Pi/6
                    )
                )
            )
            "#,
    );

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

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
