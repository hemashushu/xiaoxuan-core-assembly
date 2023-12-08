// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancasm_assembler::utils::helper_generate_module_image_binary_from_str;
use ancvm_program::program_source::ProgramSource;
use ancvm_process::{
    in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
};
use ancvm_types::ForeignValue;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_data_load_and_store_initialized() {
    //        read-only data section
    //        ======================
    //
    //       |low address    high addr|
    //       |                        |
    // index |0           1           |
    //  type |i32------| |i32---------|
    //
    //  data 11 13 17 19 c0 d0    e0 f0
    //       |           |        |  |
    //       |           |        |  |load8u (step 1)
    //       |           |        |load8u (step 2)
    //       |           |load16u (step 3)
    //       |load32 (step 4)
    //
    //        read-write data section
    //        =======================
    //
    //       |low address                                                              high address|
    //       |                                                                                     |
    // index |2(0)                               3(1)   4(2)   5(3)                      6(4)      |
    //  type |bytes-------------------|         |f32|  |f64|  |i64------------------|   |i32-------|
    //
    //  data 11 13 17 19 c0 d0    e0 f0         f32    f64    11 13 17 19 c0 d0 e0 f0    11 12 17 19
    //       |           |        |  |          |      |      ^                          ^
    //       |store32    |store16 |  |          |      |      |                          |
    //                            |  |          |      |      |                          |
    //                      store8|  |          |      |      |                          |
    //       |                       |store8    |      |      |store64                   |store32
    //       |                                  |      |      |                          |
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
    // () -> (i64,i32,i32,i32,i32,i32, f32,f64 ,i64,i32)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (runtime_version "1.0")
            (data $d0 (read_only i32 0x19171311))
            (data $d1 (read_only i32 0xf0e0d0c0))
            (data $d2 (read_write (bytes 8) d"00-11-22-33-44-55-66-77"))    ;; random data
            (data $d3 (read_write f32 3.1415927))
            (data $d4 (read_write f64 2.718281828459045))
            (data $d5 (read_write i64 0))
            (data $d6 (read_write i32 0))
            (function $test
                (results
                        i64 i32 i32 i32 i32 i32 ;; group 0
                        f32 f64                 ;; group 1
                        i64 i32                 ;; group 2
                        )
                (code
                    ;; load and store
                    (data.store32 $d2
                        (data.load32_i32 $d0))

                    (data.store16 $d2 4
                        (data.load32_i16_u $d1))

                    (data.store8 $d2 6
                        (data.load32_i8_u $d1 2))

                    (data.store8 $d2 7
                        (data.load32_i8_u $d1 3))

                    ;; load and store
                    (data.store64 $d5
                        (data.load64_i64 $d2))

                    (data.store32 $d6
                        (data.load64_i64 $d2))

                    ;; load datas
                    (data.load64_i64 $d2 )
                    (data.load32_i32 $d2 4)
                    (data.load32_i16_u $d2 6)
                    (data.load32_i16_s $d2 6)
                    (data.load32_i8_u $d2 7)
                    (data.load32_i8_s $d2 7)

                    (data.load32_f32 $d3)
                    (data.load64_f64 $d4)

                    (data.load64_i64 $d5)
                    (data.load32_i32 $d6)
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
            // ForeignValue::F32(std::f32::consts::PI),
            // ForeignValue::F64(std::f64::consts::E),
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
fn test_assemble_data_load_and_store_uninitialized() {
    //        read-only data section
    //        ======================
    //
    //       |low address    high addr|
    //       |                        |
    // index |0           1           |
    //  type |i32------| |i32---------|
    //
    //  data 11 13 17 19 c0 d0    e0 f0
    //       |           |        |  |
    //       |           |        |  |load8u (step 1)
    //       |           |        |load8u (step 2)
    //       |           |load16u (step 3)
    //       |load32 (step 4)
    //
    //        uninitialized data section
    //        ==========================
    //
    //       |low address                                                              high address|
    //       |                                                                                     |
    // index |2(0)                               3(1)   4(2)   5(3)                      6(4)      |
    //  type |bytes-------------------|         |f32|  |f64|  |i64------------------|   |i32-------|
    //
    //  data 11 13 17 19 c0 d0    e0 f0         f32    f64    11 13 17 19 c0 d0 e0 f0    11 12 17 19
    //       |           |        |  |          |      |      ^                          ^
    //       |store32    |store16 |  |          |sf32  |sf64  |                          |
    //                            |  |          |stepN'|stepN |                          |
    //                      store8|  |          |      |      |                          |
    //       |                       |store8    |      |      |store64                   |store32
    //       |                                  |      |      |                          |
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

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (runtime_version "1.0")
            (data $d0 (read_only i32 0x19171311))
            (data $d1 (read_only i32 0xf0e0d0c0))
            (data $d2 (uninit (bytes 8 4)))
            (data $d3 (uninit f32))
            (data $d4 (uninit f64))
            (data $d5 (uninit i64))
            (data $d6 (uninit i32))
            (function $test
                (param $a0 f32)
                (param $a1 f64)
                (results
                        i64 i32 i32 i32 i32 i32 ;; group 0
                        f32 f64                 ;; group 1
                        i64 i32                 ;; group 2
                        )
                (code
                    ;; load and store
                    (data.store32 $d2
                        (data.load32_i32 $d0))

                    (data.store16 $d2 4
                        (data.load32_i16_u $d1))

                    (data.store8 $d2 6
                        (data.load32_i8_u $d1 2))

                    (data.store8 $d2 7
                        (data.load32_i8_u $d1 3))

                    ;; load and store. args a0, a1-> data d3, d4
                    (data.store32 $d3
                        (local.load32_f32 $a0))

                    (data.store64 $d4
                        (local.load64_f64 $a1))

                    ;; load and store
                    (data.store64 $d5
                        (data.load64_i64 $d2))

                    (data.store32 $d6
                        (data.load64_i64 $d2))

                    ;; load datas
                    (data.load64_i64 $d2 )
                    (data.load32_i32 $d2 4)
                    (data.load32_i16_u $d2 6)
                    (data.load32_i16_s $d2 6)
                    (data.load32_i8_u $d2 7)
                    (data.load32_i8_s $d2 7)

                    (data.load32_f32 $d3)
                    (data.load64_f64 $d4)

                    (data.load64_i64 $d5)
                    (data.load32_i32 $d6)
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
fn test_assemble_data_long_load_and_store() {
    //        uninitialized data section
    //        ==========================
    //
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
    // () -> (i64,i32,i32,i32,i32,i32,  i64,i32,i32,i32)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (runtime_version "1.0")
            (data $d0 (uninit (bytes 8 4)))
            (data $d1 (uninit (bytes 8 4)))
            (function $test
                (results
                        i64 i32 i32 i32 i32 i32 ;; group 0
                        i64 i32 i32 i32         ;; group 1
                        )
                (code
                    ;; store imm
                    (data.long_store32 $d0
                        (i32.imm 0)
                        (i32.imm 0x19171311))

                    (data.long_store16 $d0
                        (i32.imm 4)
                        (i32.imm 0xd0c0))

                    (data.long_store8 $d0
                        (i32.imm 6)
                        (i32.imm 0xe0))

                    (data.long_store8 $d0
                        (i32.imm 7)
                        (i32.imm 0xf0))

                    ;; load and store

                    (data.long_store64 $d1
                        (i32.imm 0)
                        (data.long_load64_i64 $d0 (i32.imm 0)))

                    ;; load datas
                    (data.long_load64_i64 $d0 (i32.imm 0))
                    (data.long_load32_i32 $d0 (i32.imm 4))
                    (data.long_load32_i16_u $d0 (i32.imm 6))
                    (data.long_load32_i16_s $d0 (i32.imm 6))
                    (data.long_load32_i8_u $d0 (i32.imm 7))
                    (data.long_load32_i8_s $d0 (i32.imm 7))

                    ;; load datas
                    (data.long_load64_i64 $d1 (i32.imm 0))
                    (data.long_load32_i32 $d1 (i32.imm 0))
                    (data.long_load32_i16_u $d1 (i32.imm 0))
                    (data.long_load32_i8_u $d1 (i32.imm 0))
                )
            )
        )
        "#,
    );

    let program_source0 = InMemoryProgramSource::new(vec![module_binary]);
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
