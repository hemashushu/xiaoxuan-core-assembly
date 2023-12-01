// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_assembler::utils::helper_generate_module_image_binaries_from_single_module_assembly;
use ancvm_process::{
    in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
};
use ancvm_program::program_source::ProgramSource;
use ancvm_types::ForeignValue;

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
    //   - add 0xffff_ffff 0x2  -> 0x1                  ;; -1 + 2 = 1
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

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test
                (param $a0 i32)
                (param $a1 i32)
                (param $a2 i32)
                (results
                    i32 i32 i32 i32
                    i32 i32 i32 i32 i32 i32
                    i32 i32 i32 i32
                    i32 i32 i32 i32)
                (code
                    ;; group 0
                    (i32.add (local.load32_i32 $a0) (local.load32_i32 $a1))
                    (i32.sub (local.load32_i32 $a1) (local.load32_i32 $a0))
                    (i32.sub (local.load32_i32 $a0) (local.load32_i32 $a1))
                    (i32.mul (local.load32_i32 $a0) (local.load32_i32 $a1))

                    ;; group 1
                    (i32.div_s (local.load32_i32 $a1) (local.load32_i32 $a2))
                    (i32.div_u (local.load32_i32 $a1) (local.load32_i32 $a2))
                    (i32.div_s (local.load32_i32 $a2) (local.load32_i32 $a1))
                    (i32.div_u (local.load32_i32 $a2) (local.load32_i32 $a1))
                    (i32.rem_s (local.load32_i32 $a1) (local.load32_i32 $a2))
                    (i32.rem_u (local.load32_i32 $a2) (local.load32_i32 $a1))

                    ;; group 2
                    (i32.inc 3 (local.load32_i32 $a0))
                    (i32.dec 3 (local.load32_i32 $a0))
                    (i32.inc 3 (local.load32_i32 $a2))
                    (i32.dec 3 (local.load32_i32 $a2))

                    ;; group 3
                    (i32.add (i32.imm 0xffff_ffff) (i32.imm 0x2))
                    (i32.mul (i32.imm 0xf0e0_d0c0) (i32.imm 0x2))
                    (i32.inc 2 (i32.imm 0xffff_ffff))
                    (i32.dec 2 (i32.imm 0x1))
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

#[test]
fn test_assemble_arithmetic_i64() {
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
    //   - div_u 2 1      -> 87425327363552377 (= 18446744073709551603/211)
    //   - rem_s 1 2      -> 3
    //   - rem_u 2 1      -> 56
    //
    //   group 2:
    //   - inc   0 amount:3     -> 14
    //   - dec   0 amount:3     -> 8
    //   - inc   2 amount:3     -> -10
    //   - dec   2 amount:3     -> -16
    //
    //   group 3:
    //   - add 0xffff_ffff_ffff_ffff 0x2  -> 0x1    ;; -1 + 2 = 1
    //   - mul 0xf0e0_d0c0_b0a0_9080 0x2  -> 0xf0e0_d0c0_b0a0_9080 << 1
    //   - inc 0xffff_ffff_ffff_ffff 0x2  -> 0x1
    //   - dec 0x1                   0x2  -> 0xffff_ffff_ffff_ffff
    //
    // (i64 i64 i64) -> (i64 i64 i64 i64  i64 i64 i64 i64 i64 i64  i64 i64 i64 i64  i64 i64 i64 i64)

    // note of the 'remainder':
    // (211 % -13) = 3
    //  ^      ^
    //  |      |divisor
    //  |dividend <--------- the result always takes the sign of the dividend.

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test
                (param $a0 i64)
                (param $a1 i64)
                (param $a2 i64)
                (results
                    i64 i64 i64 i64
                    i64 i64 i64 i64 i64 i64
                    i64 i64 i64 i64
                    i64 i64 i64 i64)
                (code
                    ;; group 0
                    (i64.add (local.load64_i64 $a0) (local.load64_i64 $a1))
                    (i64.sub (local.load64_i64 $a1) (local.load64_i64 $a0))
                    (i64.sub (local.load64_i64 $a0) (local.load64_i64 $a1))
                    (i64.mul (local.load64_i64 $a0) (local.load64_i64 $a1))

                    ;; group 1
                    (i64.div_s (local.load64_i64 $a1) (local.load64_i64 $a2))
                    (i64.div_u (local.load64_i64 $a1) (local.load64_i64 $a2))
                    (i64.div_s (local.load64_i64 $a2) (local.load64_i64 $a1))
                    (i64.div_u (local.load64_i64 $a2) (local.load64_i64 $a1))
                    (i64.rem_s (local.load64_i64 $a1) (local.load64_i64 $a2))
                    (i64.rem_u (local.load64_i64 $a2) (local.load64_i64 $a1))

                    ;; group 2
                    (i64.inc 3 (local.load64_i64 $a0))
                    (i64.dec 3 (local.load64_i64 $a0))
                    (i64.inc 3 (local.load64_i64 $a2))
                    (i64.dec 3 (local.load64_i64 $a2))

                    ;; group 3
                    (i64.add (i64.imm 0xffff_ffff_ffff_ffff) (i64.imm 0x2))
                    (i64.mul (i64.imm 0xf0e0_d0c0_b0a0_9080) (i64.imm 0x2))
                    (i64.inc 2 (i64.imm 0xffff_ffff_ffff_ffff))
                    (i64.dec 2 (i64.imm 0x1))
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
            ForeignValue::U64(11),
            ForeignValue::U64(211),
            ForeignValue::U64(-13i64 as u64),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::U64(222),
            ForeignValue::U64(200),
            ForeignValue::U64(-200_i64 as u64),
            ForeignValue::U64(2321),
            // group 1
            ForeignValue::U64(-16i64 as u64),
            ForeignValue::U64(0),
            ForeignValue::U64(0),
            ForeignValue::U64(87425327363552377),
            ForeignValue::U64(3),
            ForeignValue::U64(56),
            // group 2
            ForeignValue::U64(14),
            ForeignValue::U64(8),
            ForeignValue::U64(-10i64 as u64),
            ForeignValue::U64(-16i64 as u64),
            // group 3
            ForeignValue::U64(0x1),
            ForeignValue::U64(0xf0e0_d0c0_b0a0_9080 << 1),
            ForeignValue::U64(0x1),
            ForeignValue::U64(0xffff_ffff_ffff_ffff),
        ]
    );
}

#[test]
fn test_assemble_arithmetic_f32() {
    // numbers:
    //   - 0: 1.414
    //   - 1: 4.123

    // arithemtic:
    //   - add 0 1      -> 5.537
    //   - sub 1 0      -> 2.709
    //   - mul 0 1      -> 5.829922
    //   - div 1 0      -> 2.91584158416

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test
                (param $a0 f32)
                (param $a1 f32)
                (results
                    f32 f32 f32 f32)
                (code
                    (f32.add (local.load32_f32 $a0) (local.load32_f32 $a1))
                    (f32.sub (local.load32_f32 $a1) (local.load32_f32 $a0))
                    (f32.mul (local.load32_f32 $a0) (local.load32_f32 $a1))
                    (f32.div (local.load32_f32 $a1) (local.load32_f32 $a0))
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
        &[ForeignValue::F32(1.414), ForeignValue::F32(4.123)],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::F32(5.537),
            ForeignValue::F32(2.709),
            ForeignValue::F32(5.829922),
            ForeignValue::F32(2.915_841_6),
        ]
    );
}

#[test]
fn test_assemble_arithmetic_f64() {
    // numbers:
    //   - 0: 1.414
    //   - 1: 4.123

    // arithemtic:
    //   - add 0 1      -> 5.537
    //   - sub 1 0      -> 2.709
    //   - mul 0 1      -> 5.829922
    //   - div 1 0      -> 2.91584158416
    //
    // (f64 f64) -> (f64 f64 f64 f64)

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test
                (param $a0 f64)
                (param $a1 f64)
                (results
                    f64 f64 f64 f64)
                (code
                    (f64.add (local.load64_f64 $a0) (local.load64_f64 $a1))
                    (f64.sub (local.load64_f64 $a1) (local.load64_f64 $a0))
                    (f64.mul (local.load64_f64 $a0) (local.load64_f64 $a1))
                    (f64.div (local.load64_f64 $a1) (local.load64_f64 $a0))
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
        &[ForeignValue::F64(1.414), ForeignValue::F64(4.123)],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::F64(5.537),
            ForeignValue::F64(2.7090000000000005),
            ForeignValue::F64(5.829922),
            ForeignValue::F64(2.915841584158416),
        ]
    );
}
