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
fn test_assemble_memory_capacity() {
    // () -> (i64, i64, i64, i64, i64)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test()->(i64, i64, i64, i64, i64)
        {
            // get the capacity
            memory_capacity()

            // resize - increase
            memory_resize(imm_i32(2))

            // resize - increase
            memory_resize(imm_i32(4))

            // resize - decrease
            memory_resize(imm_i32(1))

            // get the capcity
            memory_capacity()
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
            ForeignValue::U64(0),
            ForeignValue::U64(2),
            ForeignValue::U64(4),
            ForeignValue::U64(1),
            ForeignValue::U64(1),
        ]
    );
}

#[test]
fn test_assemble_memory_load_and_store() {
    //       |low address                                                              high address|
    //       |                                                                                     |
    // index |0x100                              0x200  0x300  0x400                     0x500     |
    //  type |bytes-------------------|         |f32|  |f64|  |i64------------------|   |i32-------|
    //
    //  data 11 13 17 19 c0 d0    e0 f0         f32    f64    11 13 17 19 c0 d0 e0 f0    11 13 17 19
    //       |imm        |imm     |  |          |      |      ^                          ^
    //       |store32    |store16 |  |          |sf32  |sf64  |                          |
    //        step0       step1   |  |          |step5 |step4 |                          |
    //                         imm|  |imm       |      |      |                          |
    //       |              store8|  |store8    |      |      |store64                   |store32
    //       |               step2    step3     |      |      |                          |
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
        fn test (a0:f32, a1:f64)->
            (
            i64, i32, i32, i32, i32, i32, i32
            f32, f64
            i64, i32, i32
            )
        {
            // init heap size
            memory_resize(imm_i32(1))

            // store imm
            memory_store_i32(
                imm_i64(0x100)
                imm_i32(0x19171311)
                offset=0
            )

            memory_store_i16(
                imm_i64(0x100)
                imm_i32(0xd0c0)
                offset=4
            )

            memory_store_i8(
                imm_i64(0x100)
                imm_i32(0xe0)
                offset=6
            )

            memory_store_i8(
                imm_i64(0x100)
                imm_i32(0xf0)
                offset=7
            )

            // load from args, store to heap
            memory_store_f64(
                imm_i64(0x300)
                local_load_f64(a1)
                /* ommit the param 'offset' */
            )

            memory_store_f32(
                imm_i64(0x200)
                local_load_f32(a0)
                /* ommit the param 'offset' */
            )

            // load and store
            memory_store_i64(
                imm_i64(0x400)
                memory_load_i64(imm_i64(0x100))
                offset=0
            )

            memory_store_i32(
                imm_i64(0x500)
                memory_load_i64(imm_i64(0x100))
                offset=0
            )

            // load heaps, group 0
            memory_load_i64(
                imm_i64(0x100), 0)

            memory_load_i32_u(
                imm_i64(0x100), offset=4)

            memory_load_i32_s(
                imm_i64(0x100), offset=4)

            memory_load_i16_u(
                imm_i64(0x100), offset=6)

            memory_load_i16_s(
                imm_i64(0x100), offset=6)

            memory_load_i8_u(
                imm_i64(0x100), offset=7)

            memory_load_i8_s(
                imm_i64(0x100), offset=7)

            // load heaps, group 1
            memory_load_f32(
                imm_i64(0x200), offset=0)

            memory_load_f64(
                imm_i64(0x300), offset=0)

            // load heaps, group 2
            memory_load_i64(
                imm_i64(0x400)
                /* ommit the param 'offset' */
            )

            memory_load_i32_u(
                imm_i64(0x500)
                /* ommit the param 'offset' */
            )

            memory_load_i32_s(
                imm_i64(0x500)
                /* ommit the param 'offset' */
            )
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
