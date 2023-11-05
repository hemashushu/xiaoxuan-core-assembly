// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_program::program_source::ProgramSource;
use ancvm_runtime::{
    in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
};
use ancvm_types::ForeignValue;

mod utils;

use crate::utils::assemble_single_module;

#[test]
fn test_assemble_fundamental_zero() {
    // () -> (i32)
    let module_binaries = assemble_single_module(
        r#"
        (module "main"
            (runtime_version "1.0")
            (fn $main (result i32)
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
    assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(0)]);
}

#[test]
fn test_assemble_fundamental_drop() {
    // () -> (i32)
    let module_binaries = assemble_single_module(
        r#"
        (module "main"
            (runtime_version "1.0")
            (fn $main (result i32)
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
    assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(13)]);
}

#[test]
fn test_assemble_fundamental_duplicate() {
    // () -> (i32, i32)
    let module_binaries = assemble_single_module(
        r#"
        (module "main"
            (runtime_version "1.0")
            (fn $main (results i32 i32)
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
        vec![ForeignValue::UInt32(19), ForeignValue::UInt32(19)]
    );
}

#[test]
fn test_assemble_fundamental_swap() {
    // () -> (i32, i32)
    let module_binaries = assemble_single_module(
        r#"
        (module "main"
            (runtime_version "1.0")
            (fn $main (results i32 i32)
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
        vec![ForeignValue::UInt32(223), ForeignValue::UInt32(211)]
    );
}

#[test]
fn test_assemble_fundamental_select_nez_false() {
    // () -> (i32)
    let module_binaries = assemble_single_module(
        r#"
        (module "main"
            (runtime_version "1.0")
            (fn $main (result i32)
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
    assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(13)]);
}

#[test]
fn test_assemble_fundamental_select_nez_true() {
    // () -> (i32)
    let module_binaries = assemble_single_module(
        r#"
        (module "main"
            (runtime_version "1.0")
            (fn $main (result i32)
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
    assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(11)]);
}

#[test]
fn test_assemble_fundamental_immediate_int() {
    // () -> (i32, i64, i32, i64)
    let module_binaries = assemble_single_module(
        r#"
        (module "main"
            (runtime_version "1.0")
            (fn $main (results i32 i64 i32 i64)
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
            ForeignValue::UInt32(23),
            ForeignValue::UInt64(0x29313741_43475359u64),
            ForeignValue::UInt32((-223i32) as u32),
            ForeignValue::UInt64((-227i64) as u64)
        ]
    );
}

#[test]
fn test_assemble_fundamental_immediate_float() {
    // () -> (f32, f64, f32, f64)
    let module_binaries = assemble_single_module(
        r#"
            (module "main"
                (runtime_version "1.0")
                (fn $main (results f32 f64 f32 f64)
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
            ForeignValue::Float32(std::f32::consts::PI),
            ForeignValue::Float64(std::f64::consts::SQRT_2),
            ForeignValue::Float32(-std::f32::consts::E),
            ForeignValue::Float64(-std::f64::consts::FRAC_PI_6),
        ]
    );
}

#[test]
fn test_assemble_fundamental_immediate_float_hex() {
    // () -> (f32, f64, f32, f64)
    let module_binaries = assemble_single_module(
        r#"
            (module "main"
                (runtime_version "1.0")
                (fn $main (results f32 f64 f32 f64)
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
            ForeignValue::Float32(std::f32::consts::PI),
            ForeignValue::Float64(std::f64::consts::SQRT_2),
            ForeignValue::Float32(-std::f32::consts::E),
            ForeignValue::Float64(-std::f64::consts::FRAC_PI_6),
        ]
    );
}
