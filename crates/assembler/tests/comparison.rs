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

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
                (param $a0 i32)
                (param $a1 i32)
                (param $a2 i32)
                (param $a3 i32)
                (results
                    i32 i32 i32 i32
                    i32 i32 i32 i32
                    i32 i32 i32 i32
                    i32 i32 i32 i32
                    i32 i32 i32 i32)
                (code
                    ;; group 0
                    (i32.eqz (local.load32_i32 $a0))
                    (i32.eqz (local.load32_i32 $a1))
                    (i32.nez (local.load32_i32 $a0))
                    (i32.nez (local.load32_i32 $a1))
                    ;; group 1
                    (i32.eq (local.load32_i32 $a1) (local.load32_i32 $a2))
                    (i32.ne (local.load32_i32 $a1) (local.load32_i32 $a2))
                    (i32.eq (local.load32_i32 $a1) (local.load32_i32 $a1))
                    (i32.ne (local.load32_i32 $a1) (local.load32_i32 $a1))
                    ;; group 2
                    (i32.lt_s (local.load32_i32 $a2) (local.load32_i32 $a3))
                    (i32.lt_u (local.load32_i32 $a2) (local.load32_i32 $a3))
                    (i32.gt_s (local.load32_i32 $a2) (local.load32_i32 $a3))
                    (i32.gt_u (local.load32_i32 $a2) (local.load32_i32 $a3))
                    ;; group 3
                    (i32.le_s (local.load32_i32 $a2) (local.load32_i32 $a1))
                    (i32.le_u (local.load32_i32 $a2) (local.load32_i32 $a1))
                    (i32.le_s (local.load32_i32 $a1) (local.load32_i32 $a1))
                    (i32.le_u (local.load32_i32 $a1) (local.load32_i32 $a1))
                    ;; group 4
                    (i32.ge_s (local.load32_i32 $a1) (local.load32_i32 $a2))
                    (i32.ge_u (local.load32_i32 $a1) (local.load32_i32 $a2))
                    (i32.ge_s (local.load32_i32 $a1) (local.load32_i32 $a1))
                    (i32.ge_u (local.load32_i32 $a1) (local.load32_i32 $a1))
                )
            )
        )
        "#,
    );

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(11),
            ForeignValue::UInt32(13),
            ForeignValue::UInt32(-7i32 as u32),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            // group 1
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(0),
            // group 2
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(0),
            // group 3
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(1),
            // group 4
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(1),
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

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
                (param $a0 i64)
                (param $a1 i64)
                (param $a2 i64)
                (param $a3 i64)
                (results
                    i32 i32 i32 i32
                    i32 i32 i32 i32
                    i32 i32 i32 i32
                    i32 i32 i32 i32
                    i32 i32 i32 i32)
                (code
                    ;; group 0
                    (i64.eqz (local.load64_i64 $a0))
                    (i64.eqz (local.load64_i64 $a1))
                    (i64.nez (local.load64_i64 $a0))
                    (i64.nez (local.load64_i64 $a1))
                    ;; group 1
                    (i64.eq (local.load64_i64 $a1) (local.load64_i64 $a2))
                    (i64.ne (local.load64_i64 $a1) (local.load64_i64 $a2))
                    (i64.eq (local.load64_i64 $a1) (local.load64_i64 $a1))
                    (i64.ne (local.load64_i64 $a1) (local.load64_i64 $a1))
                    ;; group 2
                    (i64.lt_s (local.load64_i64 $a2) (local.load64_i64 $a3))
                    (i64.lt_u (local.load64_i64 $a2) (local.load64_i64 $a3))
                    (i64.gt_s (local.load64_i64 $a2) (local.load64_i64 $a3))
                    (i64.gt_u (local.load64_i64 $a2) (local.load64_i64 $a3))
                    ;; group 3
                    (i64.le_s (local.load64_i64 $a2) (local.load64_i64 $a1))
                    (i64.le_u (local.load64_i64 $a2) (local.load64_i64 $a1))
                    (i64.le_s (local.load64_i64 $a1) (local.load64_i64 $a1))
                    (i64.le_u (local.load64_i64 $a1) (local.load64_i64 $a1))
                    ;; group 4
                    (i64.ge_s (local.load64_i64 $a1) (local.load64_i64 $a2))
                    (i64.ge_u (local.load64_i64 $a1) (local.load64_i64 $a2))
                    (i64.ge_s (local.load64_i64 $a1) (local.load64_i64 $a1))
                    (i64.ge_u (local.load64_i64 $a1) (local.load64_i64 $a1))
                )
            )
        )
        "#,
    );

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::UInt64(0),
            ForeignValue::UInt64(11),
            ForeignValue::UInt64(13),
            ForeignValue::UInt64(-7i64 as u64),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            // group 1
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(0),
            // group 2
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(0),
            // group 3
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(1),
            // group 4
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(1),
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

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
            (module $app
                (runtime_version "1.0")
                (fn $test
                    (param $a0 f32)
                    (param $a1 f32)
                    (results
                        i32 i32 i32 i32
                        i32 i32 i32 i32 i32 i32
                        i32 i32 i32 i32)
                    (code
                        ;; group 0
                        (f32.eq (local.load32_f32 $a0) (local.load32_f32 $a1))
                        (f32.ne (local.load32_f32 $a0) (local.load32_f32 $a1))
                        (f32.eq (local.load32_f32 $a0) (local.load32_f32 $a0))
                        (f32.ne (local.load32_f32 $a0) (local.load32_f32 $a0))

                        ;; group 1
                        (f32.lt (local.load32_f32 $a0) (local.load32_f32 $a1))
                        (f32.lt (local.load32_f32 $a1) (local.load32_f32 $a0))
                        (f32.lt (local.load32_f32 $a0) (local.load32_f32 $a0))
                        (f32.gt (local.load32_f32 $a0) (local.load32_f32 $a1))
                        (f32.gt (local.load32_f32 $a1) (local.load32_f32 $a0))
                        (f32.gt (local.load32_f32 $a0) (local.load32_f32 $a0))

                        ;; group 2
                        (f32.le (local.load32_f32 $a1) (local.load32_f32 $a0))
                        (f32.le (local.load32_f32 $a0) (local.load32_f32 $a0))
                        (f32.ge (local.load32_f32 $a0) (local.load32_f32 $a1))
                        (f32.ge (local.load32_f32 $a0) (local.load32_f32 $a0))
                    )
                )
            )
            "#,
    );

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::Float32(1.414f32),
            ForeignValue::Float32(1.732f32),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(0),
            // group 1
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(0),
            // group 2
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
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

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
            (module $app
                (runtime_version "1.0")
                (fn $test
                    (param $a0 f64)
                    (param $a1 f64)
                    (results
                        i32 i32 i32 i32
                        i32 i32 i32 i32 i32 i32
                        i32 i32 i32 i32)
                    (code
                        ;; group 0
                        (f64.eq (local.load64_f64 $a0) (local.load64_f64 $a1))
                        (f64.ne (local.load64_f64 $a0) (local.load64_f64 $a1))
                        (f64.eq (local.load64_f64 $a0) (local.load64_f64 $a0))
                        (f64.ne (local.load64_f64 $a0) (local.load64_f64 $a0))

                        ;; group 1
                        (f64.lt (local.load64_f64 $a0) (local.load64_f64 $a1))
                        (f64.lt (local.load64_f64 $a1) (local.load64_f64 $a0))
                        (f64.lt (local.load64_f64 $a0) (local.load64_f64 $a0))
                        (f64.gt (local.load64_f64 $a0) (local.load64_f64 $a1))
                        (f64.gt (local.load64_f64 $a1) (local.load64_f64 $a0))
                        (f64.gt (local.load64_f64 $a0) (local.load64_f64 $a0))

                        ;; group 2
                        (f64.le (local.load64_f64 $a1) (local.load64_f64 $a0))
                        (f64.le (local.load64_f64 $a0) (local.load64_f64 $a0))
                        (f64.ge (local.load64_f64 $a0) (local.load64_f64 $a1))
                        (f64.ge (local.load64_f64 $a0) (local.load64_f64 $a0))
                    )
                )
            )
            "#,
    );

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::Float64(1.414f64),
            ForeignValue::Float64(1.732f64),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(0),
            // group 1
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(0),
            // group 2
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
            ForeignValue::UInt32(0),
            ForeignValue::UInt32(1),
        ]
    );
}
