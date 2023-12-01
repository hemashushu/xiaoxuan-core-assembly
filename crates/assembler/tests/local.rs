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
fn test_assemble_local_load_store() {
    // args index (also local var):     0       1
    // data type:                       f32     f64
    //
    //       |low address                                                              high address|
    // local |                                                                                     |
    // index |2                                  3      4      5                         6         |
    //  type |bytes-------------------|         |f32|  |f64|  |i64------------------|   |i32-------|
    //
    //  data 11 13 17 19 c0 d0    e0 f0         f32    f64    11 13 17 19 c0 d0 e0 f0    11 12 17 19
    //       |           |        |  |          |      |      ^                          ^
    //       |store32    |store16 |  |          |sf32  |sf64  |                          |
    //        step0       step1   |  |          |step5 |step4 |                          |
    //                      store8|  |          |      |      |                          |
    //       |              step2    |store8    |      |      |store64                   |store32
    //       |                        step3     |      |      |                          |
    //       \----->--load64-->---------------------------->--/-->-------------------->--/
    //
    //       11 13 17 19 c0 d0    e0 f0         f32    f64    11 13 17 19 c0 d0 e0 f0    11 12 17 19
    //       |           |        |  |load8u    |      |      |                          |
    //       |           |        |  |load8s  loadf32  |      |                          |
    //       |           |        |                  loadf64  |                          |
    //       |           |        |load16u                    |                          |
    //       |           |        |load16s                 load64                      load32
    //       |           |
    //       |load64     |load32
    //
    // (f32, f64) -> (i64,i32,i32,i32,i32,i32, f32,f64 ,i64,i32)

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test
                (param $a0 f32) (param $a1 f64)
                (results
                        i64 i32 i32 i32 i32 i32 ;; group 0
                        f32 f64                 ;; group 1
                        i64 i32                 ;; group 2
                        )
                (local $a2 (bytes 8 8))
                (local $a3 f32)
                (local $a4 f64)
                (local $a5 i64)
                (local $a6 i32)
                (code
                    ;; store i32, i16, i8, i8. imm -> a2
                    (local.store32 $a2   (i32.imm 0x19171311))
                    (local.store16 $a2 4 (i32.imm 0xd0c0))
                    (local.store8  $a2 6 (i32.imm 0xe0))
                    (local.store8  $a2 7 (i32.imm 0xf0))

                    ;; load and store f32, f64. args a0, a1 -> a3, a4
                    (local.store32 $a3 (local.load32_f32 $a0))
                    (local.store64 $a4 (local.load64_f64 $a1))

                    ;; load i64, store i64. a2 -> a5
                    (local.store64 $a5 (local.load64_i64 $a2))

                    ;; load i64, store i32. a2 -> a6
                    (local.store32 $a6 (local.load64_i64 $a2))

                    ;; load i64, i32, i16u, i16s, i8u, i8s. (a2 -> results)
                    (local.load64_i64   $a2 0)
                    (local.load32_i32   $a2 4)
                    (local.load32_i16_u $a2 6)
                    (local.load32_i16_s $a2 6)
                    (local.load32_i8_u  $a2 7)
                    (local.load32_i8_s  $a2 7)

                    ;; load f32, f64. (a3, a4 -> results)
                    (local.load32_f32 $a3)
                    (local.load64_f64 $a4)

                    ;; load i64, i32. (a5, a6 -> results)
                    (local.load64_i64 $a5)
                    (local.load32_i32 $a6)
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
            ForeignValue::F32(std::f32::consts::PI),
            ForeignValue::F64(std::f64::consts::E),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::U64(0xf0e0d0c0_19171311u64),
            ForeignValue::U32(0xf0e0d0c0u32),
            ForeignValue::U32(0xf0e0u32),
            ForeignValue::U32(0xfffff0e0u32), // extend from i16 to i32
            ForeignValue::U32(0xf0u32),
            ForeignValue::U32(0xfffffff0u32), // extend from i8 to i32
            // group 1
            ForeignValue::F32(std::f32::consts::PI),
            ForeignValue::F64(std::f64::consts::E),
            // group 2
            ForeignValue::U64(0xf0e0d0c0_19171311u64),
            ForeignValue::U32(0x19171311u32),
        ]
    );
}

#[test]
fn test_assemble_local_long_load_and_store() {
    //       |low address                                 high address|
    //       |                                                        |
    // index |0                                  1                    |
    //  type |bytes-------------------|         |bytes----------------|
    //
    //  data 11 13 17 19 c0 d0    e0 f0         11 13 17 19 c0 d0 e0 f0
    //       |           |        |  |          ^
    //       |store32    |store16 |  |          |
    //        step0       step1   |  |          |
    //                      store8|  |          |
    //       |              step2    |store8    |store64
    //       |                        step3     |
    //       \----->--load64-->-----------------/
    //
    //       11 13 17 19 c0 d0    e0 f0         11 13 17 19 c0 d0 e0 f0
    //       |           |        |  |load8u    |
    //       |           |        |  |load8s    |load64
    //       |           |        |             |load32
    //       |           |        |load16u      |load16u
    //       |           |        |load16s      |load8u
    //       |           |
    //       |load64     |load32
    //
    // () -> (i64,i32,i32,i32,i32,i32, i64,i32,i32,i32)
    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(
        r#"
    (module $app
        (runtime_version "1.0")
        (function $test
            (results
                    i64 i32 i32 i32 i32 i32 ;; group 0
                    i64 i32 i32 i32         ;; group 1
                    )
            (local $a0 (bytes 8 8))
            (local $a1 (bytes 8 8))
            (code
                ;; store i32, i16, i8, i8. imm -> a0
                (local.long_store32 $a0 (i32.imm 0) (i32.imm 0x19171311))
                (local.long_store16 $a0 (i32.imm 4) (i32.imm 0xd0c0))
                (local.long_store8  $a0 (i32.imm 6) (i32.imm 0xe0))
                (local.long_store8  $a0 (i32.imm 7) (i32.imm 0xf0))

                ;; load i64, store i64. a0 -> a1
                (local.long_store64 $a1
                    (i32.imm 0)
                    (local.long_load64_i64 $a0 (i32.imm 0))
                )

                ;; load i64, i32, i16u, i16s, i8u, i8s. (a0 -> results)
                (local.long_load64_i64   $a0 (i32.imm 0))
                (local.long_load32_i32   $a0 (i32.imm 4))
                (local.long_load32_i16_u $a0 (i32.imm 6))
                (local.long_load32_i16_s $a0 (i32.imm 6))
                (local.long_load32_i8_u  $a0 (i32.imm 7))
                (local.long_load32_i8_s  $a0 (i32.imm 7))

                ;; load i64, i32, i16u, i8u. (a1 -> results)
                (local.long_load64_i64   $a1 (i32.imm 0))
                (local.long_load32_i32   $a1 (i32.imm 0))
                (local.long_load32_i16_u $a1 (i32.imm 0))
                (local.long_load32_i8_u  $a1 (i32.imm 0))
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
            // group 0
            ForeignValue::U64(0xf0e0d0c0_19171311u64),
            ForeignValue::U32(0xf0e0d0c0u32),
            ForeignValue::U32(0xf0e0u32),
            ForeignValue::U32(0xfffff0e0u32), // extend from i16 to i32
            ForeignValue::U32(0xf0u32),
            ForeignValue::U32(0xfffffff0u32), // extend from i8 to i32
            // group 1
            ForeignValue::U64(0xf0e0d0c0_19171311u64),
            ForeignValue::U32(0x19171311u32),
            ForeignValue::U32(0x00001311u32), // extend from i16 to i32
            ForeignValue::U32(0x00000011u32), // extend from i8 to i32
        ]
    );
}
