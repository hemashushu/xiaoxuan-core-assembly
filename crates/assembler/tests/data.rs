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
    // index |2                                  3      4      5                         6         |
    //  type |bytes-------------------|         |f32|  |f64|  |i64------------------|   |i32-------|
    //
    //  data 11 13 17 19 c0 d0    e0 f0         f32    f64    11 13 17 19 c0 d0 e0 f0    11 13 17 19
    //       |           |        |  |          |      |      ^                          ^
    //       |store32    |store16 |  |          |      |      |                          |
    //                            |  |          |      |      |                          |
    //                      store8|  |          |      |      |                          |
    //       |                       |store8    |      |      |store64                   |store32
    //       |                                  |      |      |                          |
    //       \----->--load64-->---------------------------->--/-->-------------------->--/
    //
    //       11 13 17 19 c0 d0    e0 f0         f32    f64    11 13 17 19 c0 d0 e0 f0    11 13 17 19
    //       |           |        |  |load8u    |      |      |                          |
    //       |           |        |  |load8s  loadf32  |      |                          |
    //       |           |        |                  loadf64  |                          |
    //       |           |        |load16u                    |                          |
    //       |           |        |load16s                 load64                      load32u
    //       |           |                                                             load32s
    //       |load64     |load32u
    //                   |load32s
    //
    // () -> (i64,i32,i32,i32,i32,i32,i32,  f32,f64,  i64,i32,i32)

    let binary0 = helper_make_single_module_app(
        r#"
        readonly data d0:i32=0x19171311
        readonly data d1:i32=0xf0e0d0c0
        data d2:byte[align=8]=h"00 11 22 33 44 55 66 77"
        data d3:f32=3.1415927_f32
        data d4:f64=2.718281828459045
        data d5:i64=0
        data d6:i32=0
        fn test()->
            (
            i64, i32, i32, i32, i32, i32, i32   // group 0
            f32, f64                            // group 1
            i64, i32, i32                       // group 2
            )
        {
            // load and store
            data_store_i32(d2, data_load_i32_u(d0))
            data_store_i16(d2, data_load_i16_u(d1), offset=4)
            data_store_i8(d2, data_load_i8_u(d1, offset=2), offset=6)
            data_store_i8(d2, data_load_i8_u(d1, offset=3), offset=7)

            // load and store
            data_store_i64(d5, data_load_i64(d2))
            data_store_i32(d6, data_load_i64(d2))

            // load data
            data_load_i64(d2)
            data_load_i32_u(d2, offset=4)
            data_load_i32_s(d2, offset=4)
            data_load_i16_u(d2, offset=6)
            data_load_i16_s(d2, offset=6)
            data_load_i8_u(d2, offset=7)
            data_load_i8_s(d2, offset=7)

            data_load_f32(d3)
            data_load_f64(d4)

            data_load_i64(d5)
            data_load_i32_u(d6)
            data_load_i32_s(d6)
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
            // ForeignValue::F32(std::f32::consts::PI),
            // ForeignValue::F64(std::f64::consts::E),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::U64(0xf0e0d0c0_19171311u64),
            ForeignValue::U32(0xf0e0d0c0_u32), // i32_u
            ForeignValue::U32(0xF0E0D0C0_u32), // i32_s
            ForeignValue::U32(0xf0e0u32),
            ForeignValue::U32(0xfffff0e0u32), // extend from i16 to i32
            ForeignValue::U32(0xf0u32),
            ForeignValue::U32(0xfffffff0u32), // extend from i8 to i32
            // group 1
            ForeignValue::F32(std::f32::consts::PI),
            ForeignValue::F64(std::f64::consts::E),
            // group 2
            ForeignValue::U64(0xf0e0d0c0_19171311u64),
            ForeignValue::U32(0x19171311_u32), // i32_u
            ForeignValue::U32(0x19171311_u32), // i32_s
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
    // index |2                                  3      4      5                         6         |
    //  type |bytes-------------------|         |f32|  |f64|  |i64------------------|   |i32-------|
    //
    //  data 11 13 17 19 c0 d0    e0 f0         f32    f64    11 13 17 19 c0 d0 e0 f0    11 13 17 19
    //       |           |        |  |          |      |      ^                          ^
    //       |store32    |store16 |  |          |sf32  |sf64  |                          |
    //                            |  |          |stepN'|stepN |                          |
    //                      store8|  |          |      |      |                          |
    //       |                       |store8    |      |      |store64                   |store32
    //       |                                  |      |      |                          |
    //       \----->--load64-->---------------------------->--/-->-------------------->--/
    //
    //       11 13 17 19 c0 d0    e0 f0         f32    f64    11 13 17 19 c0 d0 e0 f0    11 13 17 19
    //       |           |        |  |load8u    |      |      |                          |
    //       |           |        |  |load8s  loadf32  |      |                          |
    //       |           |        |                  loadf64  |                          |
    //       |           |        |load16u                    |                          |
    //       |           |        |load16s                 load64                      load32u
    //       |           |                                                             load32s
    //       |load64     |load32u
    //                   |load32s
    //
    // (f32, f64) -> (i64,i32,i32,i32,i32,i32,i32,  f32,f64,  i64,i32,i32)

    let binary0 = helper_make_single_module_app(
        r#"
        readonly data d0:i32=0x19171311
        readonly data d1:i32=0xf0e0d0c0
        uninit data d2:byte[8, align=4]
        uninit data d3:f32
        uninit data d4:f64
        uninit data d5:i64
        uninit data d6:i32

        fn test(a0:f32, a1:f64) ->
            (
            i64, i32, i32, i32, i32, i32, i32   // group 0
            f32, f64                            // group 1
            i64, i32, i32                       // group 2
            )
        {
            // load and store
            data_store_i32(d2, data_load_i32_u(d0))
            data_store_i16(d2, data_load_i16_u(d1), offset=4)
            data_store_i8(d2, data_load_i8_u(d1, offset=2), offset=6)
            data_store_i8(d2, data_load_i8_u(d1, offset=3), offset=7)

            // load and store. args a0, a1-> data d3, d4
            data_store_i32(d3, local_load_f32(a0))
            data_store_i64(d4, local_load_f64(a1))

            // load and store
            data_store_i64(d5, data_load_i64(d2))
            data_store_i32(d6, data_load_i64(d2))

            // load datas
            data_load_i64(d2)
            data_load_i32_u(d2, offset=4)
            data_load_i32_s(d2, offset=4)
            data_load_i16_u(d2, offset=6)
            data_load_i16_s(d2, offset=6)
            data_load_i8_u(d2, offset=7)
            data_load_i8_s(d2, offset=7)

            data_load_f32(d3)
            data_load_f64(d4)

            data_load_i64(d5)
            data_load_i32_u(d6)
            data_load_i32_s(d6)
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
            ForeignValue::F32(std::f32::consts::PI),
            ForeignValue::F64(std::f64::consts::E),
        ],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            // group 0
            ForeignValue::U64(0xf0e0d0c0_19171311u64),
            ForeignValue::U32(0xf0e0d0c0u32), // i32u
            ForeignValue::U32(0xf0e0d0c0u32), // i32s
            ForeignValue::U32(0xf0e0u32),
            ForeignValue::U32(0xfffff0e0u32), // extend from i16 to i32
            ForeignValue::U32(0xf0u32),
            ForeignValue::U32(0xfffffff0u32), // extend from i8 to i32
            // group 1
            ForeignValue::F32(std::f32::consts::PI),
            ForeignValue::F64(std::f64::consts::E),
            // group 2
            ForeignValue::U64(0xf0e0d0c0_19171311u64),
            ForeignValue::U32(0x19171311u32), // i32u
            ForeignValue::U32(0x19171311u32), // i32s
        ]
    );
}

#[test]
fn test_assemble_data_load_and_store_extend() {
    //        uninitialized data section
    //        ==========================
    //
    //       |low address                                 high address|
    //       |                                                        |
    // index |0                                  1                    |
    //  type |bytes-------------------|         |bytes----------------|
    //
    //  data 11 13 17 19 c0 d0    e0 f0         11 13 17 19 c0 d0 e0 f0
    //       |imm        |imm     |  |          ^
    //       |store32    |store16 |  |          |
    //        step0       step1   |  |          |
    //                         imm|  |imm       |
    //       |              store8|  |store8    |store64
    //       |               step2    step3     |
    //       \----->--load64-->-----------------/
    //
    //       11 13 17 19 c0 d0    e0 f0         11 13 17 19 c0 d0 e0 f0
    //       |           |        |  |load8u    |
    //       |           |        |  |load8s    |load64
    //       |           |        |             |load32u
    //       |           |        |load16u      |load16u
    //       |           |        |load16s      |load8u
    //       |           |
    //       |load64     |load32u
    //                   |load32s
    //
    // () -> (i64,i32,i32,i32,i32,i32,i32,  i64,i32,i32,i32)

    let binary0 = helper_make_single_module_app(
        r#"
        uninit data d0:byte[8, align=4]
        uninit data d1:byte[8, align=4]
        fn test() ->
            (
            i64, i32, i32, i32, i32, i32, i32 // group 0
            i64, i32, i32, i32         // group 1
            )
        {
            // store imm
            data_store_extend_i32(
                d0
                imm_i32(0)
                imm_i32(0x19171311))

            data_store_extend_i16(
                d0
                imm_i32(4)
                imm_i32(0xd0c0))

            data_store_extend_i8(
                d0
                imm_i32(6)
                imm_i32(0xe0))

            data_store_extend_i8(
                d0
                imm_i32(7)
                imm_i32(0xf0))

            // load and store
            data_store_extend_i64(
                d1
                imm_i32(0)
                data_load_extend_i64(d0, imm_i32(0)))

            // load data
            data_load_extend_i64(d0, imm_i32(0))
            data_load_extend_i32_u(d0, imm_i32(4))
            data_load_extend_i32_s(d0, imm_i32(4))
            data_load_extend_i16_u(d0, imm_i32(6))
            data_load_extend_i16_s(d0, imm_i32(6))
            data_load_extend_i8_u(d0, imm_i32(7))
            data_load_extend_i8_s(d0, imm_i32(7))

            // load data
            data_load_extend_i64(d1, imm_i32(0))
            data_load_extend_i32_u(d1, imm_i32(0))
            data_load_extend_i16_u(d1, imm_i32(0))
            data_load_extend_i8_u(d1, imm_i32(0))
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
            // group 0
            ForeignValue::U64(0xf0e0d0c0_19171311u64),
            ForeignValue::U32(0xf0e0d0c0u32), // i32u
            ForeignValue::U32(0xf0e0d0c0u32), // i32s
            ForeignValue::U32(0xf0e0u32),
            ForeignValue::U32(0xfffff0e0u32), // extend from i16 to i32
            ForeignValue::U32(0xf0u32),
            ForeignValue::U32(0xfffffff0u32), // extend from i8 to i32
            // group 1
            ForeignValue::U64(0xf0e0d0c0_19171311u64),
            ForeignValue::U32(0x19171311u32), // i32u
            ForeignValue::U32(0x00001311u32), // extend from i16 to i32
            ForeignValue::U32(0x00000011u32), // extend from i8 to i32
        ]
    );
}
