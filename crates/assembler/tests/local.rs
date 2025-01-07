// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use anc_assembler::utils::helper_make_single_module_app;
use anc_context::process_resource::ProcessResource;
use anc_isa::ForeignValue;
use anc_processor::{
    handler::Handler, in_memory_process_resource::InMemoryProcessResource, process::process_function,
};
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
    //  data 11 13 17 19 c0 d0    e0 f0         f32    f64    11 13 17 19 c0 d0 e0 f0    11 13 17 19
    //       |imm        |imm     |  |          |      |      ^                          ^
    //       |store32    |store16 |  |          |sf32  |sf64  |                          |
    //        step0       step1   |  |          |step5 |step4 |                          |
    //                         imm|  |imm       |      |      |                          |
    //       |              store8|  |store8    |      |      |store64                   |store32
    //       |               step2     step3     |      |      |                          |
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
        fn test(a0:f32, a1:f64) ->
            (
            i64, i32, i32, i32, i32, i32, i32   // group 0
            f32, f64                            // group 1
            i64, i32, i32                       // group 2
            )
            [a2:byte[8, align=8], a3:f32, a4:f64, a5:i64, a6:i32]
        {
            // store i32, i16, i8, i8. imm -> a2
            local_store_i32(a2, imm_i32(0x19171311))
            local_store_i16(a2, imm_i32(0xd0c0), offset=4)
            local_store_i8(a2, imm_i32(0xe0), offset=6)
            local_store_i8(a2, imm_i32(0xf0), offset=7)

            // load and store f32, f64. args a0, a1 -> a3, a4
            local_store_f32(a3, local_load_f32(a0))
            local_store_f64(a4, local_load_f64(a1))

            // load i64, store i64. a2 -> a5
            local_store_i64(a5, local_load_i64(a2))

            // load i64, store i32. a2 -> a6
            local_store_i32(a6, local_load_i64(a2))

            // load i64, i32, i16u, i16s, i8u, i8s. (a2 -> results)
            local_load_i64(a2, offset=0)
            local_load_i32_u(a2, offset=4)
            local_load_i32_s(a2, offset=4)
            local_load_i16_u(a2, offset=6)
            local_load_i16_s(a2, offset=6)
            local_load_i8_u(a2, offset=7)
            local_load_i8_s(a2, offset=7)

            // load f32, f64. (a3, a4 -> results)
            local_load_f32(a3)
            local_load_f64(a4)

            // load i64, i32. (a5, a6 -> results)
            local_load_i64(a5)
            local_load_i32_u(a6)
            local_load_i32_s(a6)
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
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
fn test_assemble_local_load_and_store_extend() {
    //       |low address                                 high address|
    //       |                                                        |
    // index |0                                  1                    |
    //  type |bytes-------------------|         |bytes----------------|
    //
    //  data 11 13 17 19 c0 d0    e0 f0         11 13 17 19 c0 d0 e0 f0
    //       |           |        |  |          ^
    //       |store32    |store16 |  |          |
    //        step0       step1   |  |          |
    //                         imm|  |imm       |
    //       |              store8|  |store8    |store64
    //       |              step2     step3     |
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
        fn test () ->
            (
            i64, i32, i32, i32, i32, i32, i32   // group 0
            i64, i32, i32, i32                  // group 1
            )
            [a0:byte[8, align=8], a1:byte[8, align=8]]
        {
            // store i32, i16, i8, i8. imm -> a0
            local_store_extend_i32(a0, imm_i32(0), imm_i32(0x19171311))
            local_store_extend_i16(a0, imm_i32(4), imm_i32(0xd0c0))
            local_store_extend_i8(a0, imm_i32(6), imm_i32(0xe0))
            local_store_extend_i8(a0, imm_i32(7), imm_i32(0xf0))

            // load i64, store i64. a0 -> a1
            local_store_extend_i64(
                a1
                imm_i32(0)
                local_load_extend_i64(a0, imm_i32(0)))

            // load i64, i32, i16u, i16s, i8u, i8s. (a0 -> results)
            local_load_extend_i64(a0, imm_i32(0))
            local_load_extend_i32_u(a0, imm_i32(4))
            local_load_extend_i32_s(a0, imm_i32(4))
            local_load_extend_i16_u(a0, imm_i32(6))
            local_load_extend_i16_s(a0, imm_i32(6))
            local_load_extend_i8_u(a0, imm_i32(7))
            local_load_extend_i8_s(a0, imm_i32(7))

            // load i64, i32, i16u, i8u. (a1 -> results)
            local_load_extend_i64(a1, imm_i32(0))
            local_load_extend_i32_u(a1, imm_i32(0))
            local_load_extend_i16_u(a1, imm_i32(0))
            local_load_extend_i8_u(a1, imm_i32(0))
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
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
