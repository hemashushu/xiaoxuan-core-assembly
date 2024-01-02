// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancasm_assembler::utils::helper_generate_module_image_binary_from_str;
use ancvm_process::{
    in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
};
use ancvm_program::program_source::ProgramSource;
use ancvm_types::ForeignValue;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_math_i32() {
    // numbers:
    //   - 0: 11
    //   - 1: -11
    //
    // functions:
    //   - abs      0   -> 11
    //   - abs      1   -> 11
    //   - neg      0   -> -11
    //   - neg      1   -> 11
    //
    // (i32 i32) -> (i32 i32 i32 i32)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
            (module $app
                (compiler_version "1.0")
                (function $test
                    (param $a0 i32)
                    (param $a1 i32)
                    (results
                        i32 i32 i32 i32)
                    (code
                        (i32.abs (local.load32_i32 $a0))
                        (i32.abs (local.load32_i32 $a1))
                        (i32.neg (local.load32_i32 $a0))
                        (i32.neg (local.load32_i32 $a1))
                    )
                )
            )
            "#,
    );

    let program_source0 = InMemoryProgramSource::new(vec![module_binary]);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(11), ForeignValue::U32(-11i32 as u32)],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::U32(11),
            ForeignValue::U32(11),
            ForeignValue::U32(-11i32 as u32),
            ForeignValue::U32(11),
        ]
    );
}

#[test]
fn test_assemble_math_i64() {
    // numbers:
    //   - 0: 11
    //   - 1: -11
    //
    // functions:
    //   - abs      0   -> 11
    //   - abs      1   -> 11
    //   - neg      0   -> -11
    //   - neg      1   -> 11
    //
    // (i64 i64) -> (i64 i64 i64 i64)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
            (module $app
                (compiler_version "1.0")
                (function $test
                    (param $a0 i64)
                    (param $a1 i64)
                    (results
                        i64 i64 i64 i64)
                    (code
                        (i64.abs (local.load64_i64 $a0))
                        (i64.abs (local.load64_i64 $a1))
                        (i64.neg (local.load64_i64 $a0))
                        (i64.neg (local.load64_i64 $a1))
                    )
                )
            )
            "#,
    );

    let program_source0 = InMemoryProgramSource::new(vec![module_binary]);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U64(11), ForeignValue::U64(-11i64 as u64)],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::U64(11),
            ForeignValue::U64(11),
            ForeignValue::U64(-11i64 as u64),
            ForeignValue::U64(11),
        ]
    );
}

#[test]
fn test_assemble_math_f32_part_a() {
    // numbers:
    //   - 0: 1.414
    //   - 1: -1.732
    //   - 2: 2.4
    //   - 3: 2.5
    //   - 4: 2.6
    //   - 5: 5.5
    //   - 6: -2.4
    //   - 7: -2.5
    //   - 8: -2.6
    //   - 9: -5.5
    //
    // functions:
    //   - abs      0   -> 1.414
    //   - abs      1   -> 1.732
    //   - neg      0   -> -1.414
    //   - neg      1   -> 1.732
    //
    //   - ceil     2   -> 3.0
    //   - ceil     4   -> 3.0
    //   - ceil     6   -> -2.0
    //   - ceil     8   -> -2.0
    //
    //   - floor    2   -> 2.0
    //   - floor    4   -> 2.0
    //   - floor    6   -> -3.0
    //   - floor    8   -> -3.0
    //
    //   - round_half_away_from_zero    2   -> 2.0
    //   - round_half_away_from_zero    3   -> 3.0
    //   - round_half_away_from_zero    4   -> 3.0
    //   - round_half_away_from_zero    5   -> 6.0
    //   - round_half_away_from_zero    6   -> -2.0
    //   - round_half_away_from_zero    7   -> -3.0
    //   - round_half_away_from_zero    8   -> -3.0
    //   - round_half_away_from_zero    9   -> -6.0
    //
    //   - round_half_to_even    2   -> 2.0
    //   - round_half_to_even    3   -> 2.0
    //   - round_half_to_even    4   -> 3.0
    //   - round_half_to_even    5   -> 6.0
    //   - round_half_to_even    6   -> -2.0
    //   - round_half_to_even    7   -> -2.0
    //   - round_half_to_even    8   -> -3.0
    //   - round_half_to_even    9   -> -6.0
    //
    // (f32 f32 f32 f32  f32 f32 f32 f32) ->
    // (f32 f32 f32 f32  f32 f32 f32 f32  f32 f32 f32 f32
    //  f32 f32 f32 f32 f32 f32 f32 f32
    //  f32 f32 f32 f32 f32 f32 f32 f32)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
            (module $app
                (compiler_version "1.0")
                (function $test
                    (param $a0 f32)
                    (param $a1 f32)
                    (param $a2 f32)
                    (param $a3 f32)
                    (param $a4 f32)
                    (param $a5 f32)
                    (param $a6 f32)
                    (param $a7 f32)
                    (param $a8 f32)
                    (param $a9 f32)
                    (results
                        f32 f32 f32 f32
                        f32 f32 f32 f32
                        f32 f32 f32 f32
                        f32 f32 f32 f32 f32 f32 f32 f32
                        f32 f32 f32 f32 f32 f32 f32 f32)
                    (code
                        (f32.abs (local.load32_f32 $a0))
                        (f32.abs (local.load32_f32 $a1))
                        (f32.neg (local.load32_f32 $a0))
                        (f32.neg (local.load32_f32 $a1))

                        (f32.ceil (local.load32_f32 $a2))
                        (f32.ceil (local.load32_f32 $a4))
                        (f32.ceil (local.load32_f32 $a6))
                        (f32.ceil (local.load32_f32 $a8))

                        (f32.floor (local.load32_f32 $a2))
                        (f32.floor (local.load32_f32 $a4))
                        (f32.floor (local.load32_f32 $a6))
                        (f32.floor (local.load32_f32 $a8))

                        (f32.round_half_away_from_zero (local.load32_f32 $a2))
                        (f32.round_half_away_from_zero (local.load32_f32 $a3))
                        (f32.round_half_away_from_zero (local.load32_f32 $a4))
                        (f32.round_half_away_from_zero (local.load32_f32 $a5))
                        (f32.round_half_away_from_zero (local.load32_f32 $a6))
                        (f32.round_half_away_from_zero (local.load32_f32 $a7))
                        (f32.round_half_away_from_zero (local.load32_f32 $a8))
                        (f32.round_half_away_from_zero (local.load32_f32 $a9))

                        (f32.round_half_to_even (local.load32_f32 $a2))
                        (f32.round_half_to_even (local.load32_f32 $a3))
                        (f32.round_half_to_even (local.load32_f32 $a4))
                        (f32.round_half_to_even (local.load32_f32 $a5))
                        (f32.round_half_to_even (local.load32_f32 $a6))
                        (f32.round_half_to_even (local.load32_f32 $a7))
                        (f32.round_half_to_even (local.load32_f32 $a8))
                        (f32.round_half_to_even (local.load32_f32 $a9))
                    )
                )
            )
            "#,
    );

    let program_source0 = InMemoryProgramSource::new(vec![module_binary]);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::F32(1.414),
            ForeignValue::F32(-1.732),
            ForeignValue::F32(2.4),
            ForeignValue::F32(2.5),
            ForeignValue::F32(2.6),
            ForeignValue::F32(5.5),
            ForeignValue::F32(-2.4),
            ForeignValue::F32(-2.5),
            ForeignValue::F32(-2.6),
            ForeignValue::F32(-5.5),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::F32(1.414),
            ForeignValue::F32(1.732),
            ForeignValue::F32(-1.414),
            ForeignValue::F32(1.732),
            // group 1
            ForeignValue::F32(3.0),
            ForeignValue::F32(3.0),
            ForeignValue::F32(-2.0),
            ForeignValue::F32(-2.0),
            // group 2
            ForeignValue::F32(2.0),
            ForeignValue::F32(2.0),
            ForeignValue::F32(-3.0),
            ForeignValue::F32(-3.0),
            // group 3
            ForeignValue::F32(2.0),
            ForeignValue::F32(3.0),
            ForeignValue::F32(3.0),
            ForeignValue::F32(6.0),
            ForeignValue::F32(-2.0),
            ForeignValue::F32(-3.0),
            ForeignValue::F32(-3.0),
            ForeignValue::F32(-6.0),
            // group 4
            ForeignValue::F32(2.0),
            ForeignValue::F32(2.0),
            ForeignValue::F32(3.0),
            ForeignValue::F32(6.0),
            ForeignValue::F32(-2.0),
            ForeignValue::F32(-2.0),
            ForeignValue::F32(-3.0),
            ForeignValue::F32(-6.0),
        ]
    );
}

#[test]
fn test_assemble_math_f32_part_b() {
    // numbers:
    //   - 0: 1.414
    //   - 1: 4.0
    //   - 2: 27.0
    //   - 3: 3.0
    //   - 4: 9.0
    //   - 5  -3.0
    //   - 6: -9.0
    //   - 7: 100.0
    //   - 8: 2.718281828               // std::f32::consts::E
    //   - 9: 0.523598776   (deg 30)    // std::f32::consts::FRAC_PI_6
    //
    // functions:
    //   group 0:
    //   - trunc   0        -> 1.0
    //   - fract   0        -> 0.41400003
    //   - sqrt    1        -> 2.0
    //   - cbrt    2        -> 3.0
    //
    //   group 1:
    //   - exp     3        -> 20.085_537 (e^3)
    //   - exp2    4        -> 512.0
    //   - ln      8        -> 0.99999994
    //   - log2    1        -> 2.0 (log_2 4)
    //   - log10   7        -> 2.0 (log_10 100)
    //
    //   group 2:
    //   - sin     9        -> 0.5
    //   - cos     9        -> 0.866_025_4
    //   - tan     9        -> 0.577_350_3
    //   - asin    imm(0.5)     -> deg 30
    //   - acos    imm(0.86..)  -> deg 30
    //   - atab    imm(0.57..)  -> deg 30
    //
    //   group 3:
    //   - pow      1 3      -> 64.0 (4^3)
    //   - log      4 3      -> 2.0 (log_3 9)
    //
    //   group 4:
    //   - copysign 4 3      -> 9.0
    //   - copysign 4 5      -> -9.0
    //   - copysign 5 4      -> 3.0
    //   - copysign 5 6      -> -3.0
    //
    //   group 5:
    //   - min      3 4      -> 3.0
    //   - min      4 5      -> -3.0
    //   - max      4 5      -> 9.0
    //   - max      5 6      -> -3.0
    //
    // (f32 f32 f32 f32  f32 f32 f32 f32  f32 f32) ->
    // (f32 f32 f32 f32  f32 f32 f32 f32 f32  f32 f32 f32 f32 f32 f32
    //  f32 f32
    //  f32 f32 f32 f32
    //  f32 f32 f32 f32)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
            (module $app
                (compiler_version "1.0")
                (function $test
                    (param $a0 f32)
                    (param $a1 f32)
                    (param $a2 f32)
                    (param $a3 f32)
                    (param $a4 f32)
                    (param $a5 f32)
                    (param $a6 f32)
                    (param $a7 f32)
                    (param $a8 f32)
                    (param $a9 f32)
                    (results
                        f32 f32 f32 f32
                        f32 f32 f32 f32 f32
                        f32 f32 f32 f32 f32 f32
                        f32 f32
                        f32 f32 f32 f32
                        f32 f32 f32 f32)
                    (code
                        (f32.trunc (local.load32_f32 $a0))
                        (f32.fract (local.load32_f32 $a0))
                        (f32.sqrt (local.load32_f32 $a1))
                        (f32.cbrt (local.load32_f32 $a2))

                        (f32.exp (local.load32_f32 $a3))
                        (f32.exp2 (local.load32_f32 $a4))
                        (f32.ln (local.load32_f32 $a8))
                        (f32.log2 (local.load32_f32 $a1))
                        (f32.log10 (local.load32_f32 $a7))

                        (f32.sin (local.load32_f32 $a9))
                        (f32.cos (local.load32_f32 $a9))
                        (f32.tan (local.load32_f32 $a9))
                        (f32.asin (f32.imm 0.5))
                        (f32.acos (f32.imm 0.866_025_4))
                        (f32.atan (f32.imm 0.577_350_3))

                        (f32.pow (local.load32_f32 $a1) (local.load32_f32 $a3))
                        (f32.log (local.load32_f32 $a4) (local.load32_f32 $a3))

                        (f32.copysign (local.load32_f32 $a4) (local.load32_f32 $a3))
                        (f32.copysign (local.load32_f32 $a4) (local.load32_f32 $a5))
                        (f32.copysign (local.load32_f32 $a5) (local.load32_f32 $a4))
                        (f32.copysign (local.load32_f32 $a5) (local.load32_f32 $a6))

                        (f32.min (local.load32_f32 $a3) (local.load32_f32 $a4))
                        (f32.min (local.load32_f32 $a4) (local.load32_f32 $a5))
                        (f32.max (local.load32_f32 $a4) (local.load32_f32 $a5))
                        (f32.max (local.load32_f32 $a5) (local.load32_f32 $a6))
                    )
                )
            )
            "#,
    );

    let program_source0 = InMemoryProgramSource::new(vec![module_binary]);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::F32(1.414),
            ForeignValue::F32(4.0),
            ForeignValue::F32(27.0),
            ForeignValue::F32(3.0),
            ForeignValue::F32(9.0),
            ForeignValue::F32(-3.0),
            ForeignValue::F32(-9.0),
            ForeignValue::F32(100.0),
            ForeignValue::F32(std::f32::consts::E),
            ForeignValue::F32(std::f32::consts::FRAC_PI_6),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::F32(1.0),
            ForeignValue::F32(0.41400003),
            ForeignValue::F32(2.0),
            ForeignValue::F32(3.0),
            // group 1
            ForeignValue::F32(20.085_537),
            ForeignValue::F32(512.0),
            ForeignValue::F32(0.99999994), // 1.0
            ForeignValue::F32(2.0),
            ForeignValue::F32(2.0),
            // group 2
            ForeignValue::F32(0.5),
            ForeignValue::F32(0.866_025_4),
            ForeignValue::F32(0.577_350_3),
            ForeignValue::F32(std::f32::consts::FRAC_PI_6),
            ForeignValue::F32(std::f32::consts::FRAC_PI_6),
            ForeignValue::F32(std::f32::consts::FRAC_PI_6),
            // group 3
            ForeignValue::F32(64.0),
            ForeignValue::F32(2.0),
            // group 4
            ForeignValue::F32(9.0),
            ForeignValue::F32(-9.0),
            ForeignValue::F32(3.0),
            ForeignValue::F32(-3.0),
            // group 5
            ForeignValue::F32(3.0),
            ForeignValue::F32(-3.0),
            ForeignValue::F32(9.0),
            ForeignValue::F32(-3.0),
        ]
    );
}

#[test]
fn test_assemble_math_f64_part_a() {
    // numbers:
    //   - 0: 1.414
    //   - 1: -1.732
    //   - 2: 2.4
    //   - 3: 2.5
    //   - 4: 2.6
    //   - 5: 5.5
    //   - 6: -2.4
    //   - 7: -2.5
    //   - 8: -2.6
    //   - 9: -5.5
    //
    // functions:
    //   - abs      0   -> 1.414
    //   - abs      1   -> 1.732
    //   - neg      0   -> -1.414
    //   - neg      1   -> 1.732
    //
    //   - ceil     2   -> 3.0
    //   - ceil     4   -> 3.0
    //   - ceil     5   -> -2.0
    //   - ceil     7   -> -2.0
    //
    //   - floor    2   -> 2.0
    //   - floor    4   -> 2.0
    //   - floor    5   -> -3.0
    //   - floor    7   -> -3.0
    //
    //   - round_half_away_from_zero    2   -> 2.0
    //   - round_half_away_from_zero    3   -> 3.0
    //   - round_half_away_from_zero    4   -> 3.0
    //   - round_half_away_from_zero    5   -> 6.0
    //   - round_half_away_from_zero    6   -> -2.0
    //   - round_half_away_from_zero    7   -> -3.0
    //   - round_half_away_from_zero    8   -> -3.0
    //   - round_half_away_from_zero    9   -> -6.0
    //
    //   - round_half_to_even    2   -> 2.0
    //   - round_half_to_even    3   -> 2.0
    //   - round_half_to_even    4   -> 3.0
    //   - round_half_to_even    5   -> 6.0
    //   - round_half_to_even    6   -> -2.0
    //   - round_half_to_even    7   -> -2.0
    //   - round_half_to_even    8   -> -3.0
    //   - round_half_to_even    9   -> -6.0
    //
    // (f64 f64 f64 f64  f64 f64 f64 f64) ->
    // (f64 f64 f64 f64  f64 f64 f64 f64  f64 f64 f64 f64
    //  f64 f64 f64 f64 f64 f64 f64 f64
    //  f64 f64 f64 f64 f64 f64 f64 f64)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
            (module $app
                (compiler_version "1.0")
                (function $test
                    (param $a0 f64)
                    (param $a1 f64)
                    (param $a2 f64)
                    (param $a3 f64)
                    (param $a4 f64)
                    (param $a5 f64)
                    (param $a6 f64)
                    (param $a7 f64)
                    (param $a8 f64)
                    (param $a9 f64)
                    (results
                        f64 f64 f64 f64
                        f64 f64 f64 f64
                        f64 f64 f64 f64
                        f64 f64 f64 f64 f64 f64 f64 f64
                        f64 f64 f64 f64 f64 f64 f64 f64)
                    (code
                        (f64.abs (local.load64_f64 $a0))
                        (f64.abs (local.load64_f64 $a1))
                        (f64.neg (local.load64_f64 $a0))
                        (f64.neg (local.load64_f64 $a1))

                        (f64.ceil (local.load64_f64 $a2))
                        (f64.ceil (local.load64_f64 $a4))
                        (f64.ceil (local.load64_f64 $a6))
                        (f64.ceil (local.load64_f64 $a8))

                        (f64.floor (local.load64_f64 $a2))
                        (f64.floor (local.load64_f64 $a4))
                        (f64.floor (local.load64_f64 $a6))
                        (f64.floor (local.load64_f64 $a8))

                        (f64.round_half_away_from_zero (local.load64_f64 $a2))
                        (f64.round_half_away_from_zero (local.load64_f64 $a3))
                        (f64.round_half_away_from_zero (local.load64_f64 $a4))
                        (f64.round_half_away_from_zero (local.load64_f64 $a5))
                        (f64.round_half_away_from_zero (local.load64_f64 $a6))
                        (f64.round_half_away_from_zero (local.load64_f64 $a7))
                        (f64.round_half_away_from_zero (local.load64_f64 $a8))
                        (f64.round_half_away_from_zero (local.load64_f64 $a9))

                        (f64.round_half_to_even (local.load64_f64 $a2))
                        (f64.round_half_to_even (local.load64_f64 $a3))
                        (f64.round_half_to_even (local.load64_f64 $a4))
                        (f64.round_half_to_even (local.load64_f64 $a5))
                        (f64.round_half_to_even (local.load64_f64 $a6))
                        (f64.round_half_to_even (local.load64_f64 $a7))
                        (f64.round_half_to_even (local.load64_f64 $a8))
                        (f64.round_half_to_even (local.load64_f64 $a9))
                    )
                )
            )
            "#,
    );

    let program_source0 = InMemoryProgramSource::new(vec![module_binary]);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::F64(1.414),
            ForeignValue::F64(-1.732),
            ForeignValue::F64(2.4),
            ForeignValue::F64(2.5),
            ForeignValue::F64(2.6),
            ForeignValue::F64(5.5),
            ForeignValue::F64(-2.4),
            ForeignValue::F64(-2.5),
            ForeignValue::F64(-2.6),
            ForeignValue::F64(-5.5),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::F64(1.414),
            ForeignValue::F64(1.732),
            ForeignValue::F64(-1.414),
            ForeignValue::F64(1.732),
            // group 1
            ForeignValue::F64(3.0),
            ForeignValue::F64(3.0),
            ForeignValue::F64(-2.0),
            ForeignValue::F64(-2.0),
            // group 2
            ForeignValue::F64(2.0),
            ForeignValue::F64(2.0),
            ForeignValue::F64(-3.0),
            ForeignValue::F64(-3.0),
            // group 3
            ForeignValue::F64(2.0),
            ForeignValue::F64(3.0),
            ForeignValue::F64(3.0),
            ForeignValue::F64(6.0),
            ForeignValue::F64(-2.0),
            ForeignValue::F64(-3.0),
            ForeignValue::F64(-3.0),
            ForeignValue::F64(-6.0),
            // group 4
            ForeignValue::F64(2.0),
            ForeignValue::F64(2.0),
            ForeignValue::F64(3.0),
            ForeignValue::F64(6.0),
            ForeignValue::F64(-2.0),
            ForeignValue::F64(-2.0),
            ForeignValue::F64(-3.0),
            ForeignValue::F64(-6.0),
        ]
    );
}

#[test]
fn test_assemble_math_f64_part_b() {
    // numbers:
    //   - 0: 1.414
    //   - 1: 4.0
    //   - 2: 27.0
    //   - 3: 3.0
    //   - 4: 9.0
    //   - 5: -3.0
    //   - 6: -9.0
    //   - 7: 100.0
    //   - 8: 2.718281828               // std::f64::consts::E
    //   - 9: 0.523598776   (deg 30)    // std::f64::consts::FRAC_PI_6
    //
    // functions:
    //   group 0:
    //   - trunc   0        -> 1.0
    //   - fract   0        -> 0.4139999999999999
    //   - sqrt    1        -> 2.0
    //   - cbrt    2        -> 3.0000000000000004
    //
    //   group 1:
    //   - exp     3        -> 20.085536923187668 (e^3)
    //   - exp2    4        -> 512.0
    //   - ln      8        -> 1.0
    //   - log2    1        -> 2.0 (log_2 4)
    //   - log10   7        -> 2.0 (log_10 100)
    //
    //   group 2:
    //   - sin     9        -> 0.5
    //   - cos     9        -> 0.866_025_403_784_438_6
    //   - tan     9        -> 0.577_350_269_189_625_8
    //   - asin    imm(0.5)     -> deg 30
    //   - acos    imm(0.86..)  -> deg 30
    //   - atab    imm(0.57..)  -> deg 30
    //
    //   group 3:
    //   - pow     1 3      -> 64.0 (4^3)
    //   - log     4 3      -> 2.0 (log_3 9)
    //
    //   group 4:
    //   - copysign 4 3      -> 9.0
    //   - copysign 4 5      -> -9.0
    //   - copysign 5 4      -> 3.0
    //   - copysign 5 6      -> -3.0
    //
    //   group 5:
    //   - min      3 4      -> 3.0
    //   - min      4 5      -> -3.0
    //   - max      4 5      -> 9.0
    //   - max      5 6      -> -3.0
    //
    // (f64 f64 f64 f64  f64 f64 f64 f64  f64 f64) ->
    // (f64 f64 f64 f64  f64 f64 f64 f64 f64  f64 f64 f64 f64 f64 f64
    //  f64 f64
    //  f64 f64 f64 f64
    //  f64 f64 f64 f64)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
            (module $app
                (compiler_version "1.0")
                (function $test
                    (param $a0 f64)
                    (param $a1 f64)
                    (param $a2 f64)
                    (param $a3 f64)
                    (param $a4 f64)
                    (param $a5 f64)
                    (param $a6 f64)
                    (param $a7 f64)
                    (param $a8 f64)
                    (param $a9 f64)
                    (results
                        f64 f64 f64 f64
                        f64 f64 f64 f64 f64
                        f64 f64 f64 f64 f64 f64
                        f64 f64
                        f64 f64 f64 f64
                        f64 f64 f64 f64)
                    (code
                        (f64.trunc (local.load64_f64 $a0))
                        (f64.fract (local.load64_f64 $a0))
                        (f64.sqrt (local.load64_f64 $a1))
                        (f64.cbrt (local.load64_f64 $a2))

                        (f64.exp (local.load64_f64 $a3))
                        (f64.exp2 (local.load64_f64 $a4))
                        (f64.ln (local.load64_f64 $a8))
                        (f64.log2 (local.load64_f64 $a1))
                        (f64.log10 (local.load64_f64 $a7))

                        (f64.sin (local.load64_f64 $a9))
                        (f64.cos (local.load64_f64 $a9))
                        (f64.tan (local.load64_f64 $a9))
                        (f64.asin (f64.imm 0.5))
                        (f64.acos (f64.imm 0.8660254037844386))
                        (f64.atan (f64.imm 0.5773502691896258))

                        (f64.pow (local.load64_f64 $a1) (local.load64_f64 $a3))
                        (f64.log (local.load64_f64 $a4) (local.load64_f64 $a3))

                        (f64.copysign (local.load64_f64 $a4) (local.load64_f64 $a3))
                        (f64.copysign (local.load64_f64 $a4) (local.load64_f64 $a5))
                        (f64.copysign (local.load64_f64 $a5) (local.load64_f64 $a4))
                        (f64.copysign (local.load64_f64 $a5) (local.load64_f64 $a6))

                        (f64.min (local.load64_f64 $a3) (local.load64_f64 $a4))
                        (f64.min (local.load64_f64 $a4) (local.load64_f64 $a5))
                        (f64.max (local.load64_f64 $a4) (local.load64_f64 $a5))
                        (f64.max (local.load64_f64 $a5) (local.load64_f64 $a6))
                    )
                )
            )
            "#,
    );

    let program_source0 = InMemoryProgramSource::new(vec![module_binary]);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::F64(1.414),
            ForeignValue::F64(4.0),
            ForeignValue::F64(27.0),
            ForeignValue::F64(3.0),
            ForeignValue::F64(9.0),
            ForeignValue::F64(-3.0),
            ForeignValue::F64(-9.0),
            ForeignValue::F64(100.0),
            ForeignValue::F64(std::f64::consts::E),
            ForeignValue::F64(std::f64::consts::FRAC_PI_6),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::F64(1.0),
            ForeignValue::F64(0.4139999999999999),
            ForeignValue::F64(2.0),
            ForeignValue::F64(3.0000000000000004),
            // group 1
            ForeignValue::F64(20.085536923187668),
            ForeignValue::F64(512.0),
            ForeignValue::F64(1.0),
            ForeignValue::F64(2.0),
            ForeignValue::F64(2.0),
            // group 2
            ForeignValue::F64(0.5),
            ForeignValue::F64(0.8660254037844386),
            ForeignValue::F64(0.5773502691896258),
            ForeignValue::F64(std::f64::consts::FRAC_PI_6),
            ForeignValue::F64(std::f64::consts::FRAC_PI_6),
            ForeignValue::F64(std::f64::consts::FRAC_PI_6),
            // group 3
            ForeignValue::F64(64.0),
            ForeignValue::F64(2.0),
            // group 4
            ForeignValue::F64(9.0),
            ForeignValue::F64(-9.0),
            ForeignValue::F64(3.0),
            ForeignValue::F64(-3.0),
            // group 5
            ForeignValue::F64(3.0),
            ForeignValue::F64(-3.0),
            ForeignValue::F64(9.0),
            ForeignValue::F64(-3.0),
        ]
    );
}
