// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancasm_assembler::utils::helper_generate_module_image_binary_from_str;
use ancvm_context::program_resource::ProgramResource;
use ancvm_processor::{
    in_memory_program_resource::InMemoryProgramResource, interpreter::process_function,
};
use ancvm_types::ForeignValue;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_heap_capacity() {
    // () -> (i64, i64, i64, i64, i64)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test
                (results i64 i64 i64 i64 i64)
                (code
                    ;; get the capacity
                    (heap.capacity)

                    ;; resize - increase
                    (heap.resize (i32.imm 2))

                    ;; resize - increase
                    (heap.resize (i32.imm 4))

                    ;; resize - decrease
                    (heap.resize (i32.imm 1))

                    ;; get the capcity
                    (heap.capacity)
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
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
fn test_assemble_heap_load_and_store() {
    //       |low address                                                              high address|
    //       |                                                                                     |
    // index |0x100                              0x200  0x300  0x400                     0x500     |
    //  type |bytes-------------------|         |f32|  |f64|  |i64------------------|   |i32-------|
    //
    //  data 11 13 17 19 c0 d0    e0 f0         f32    f64    11 13 17 19 c0 d0 e0 f0    11 13 17 19
    //       |           |        |  |          |      |      ^                          ^
    //       |store32    |store16 |  |          |sf32  |sf64  |                          |
    //        step0       step1   |  |          |step5 |step4 |                          |
    //                      store8|  |          |      |      |                          |
    //       |              step2    |store8    |      |      |store64                   |store32
    //       |                        step3     |      |      |                          |
    //       \----->--load64-->---------------------------->--/-->-------------------->--/
    //
    //       11 13 17 19 c0 d0    e0 f0         f32    f64    11 13 17 19 c0 d0 e0 f0    11 13 17 19
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
            (compiler_version "1.0")
            (function $test
                (param $a0 f32)
                (param $a1 f64)
                (results
                    i64 i32 i32 i32 i32 i32
                    f32 f64
                    i64 i32)
                (code
                    ;; init heap size
                    (drop
                        (heap.resize (i32.imm 1)))

                    ;; store imm
                    (heap.store32
                        (i64.imm 0x100)
                        (i32.imm 0x19171311)
                        0
                    )

                    (heap.store16
                        (i64.imm 0x100)
                        (i32.imm 0xd0c0)
                        4
                    )

                    (heap.store8
                        (i64.imm 0x100)
                        (i32.imm 0xe0)
                        6
                    )

                    (heap.store8
                        (i64.imm 0x100)
                        (i32.imm 0xf0)
                        7
                    )

                    ;; load from args, store to heap
                    (heap.store64
                        (i64.imm 0x300)
                        (local.load64_f64 $a1)
                        (; ommit the param 'offset' ;)
                    )

                    (heap.store32
                        (i64.imm 0x200)
                        (local.load32_f32 $a0)
                        (; ommit the param 'offset' ;)
                    )

                    ;; load and store
                    (heap.store64
                        (i64.imm 0x400)
                        (heap.load64_i64 (i64.imm 0x100))
                        0
                    )

                    (heap.store32
                        (i64.imm 0x500)
                        (heap.load64_i64 (i64.imm 0x100))
                        0
                    )

                    ;; load heaps, group 0
                    (heap.load64_i64
                        (i64.imm 0x100) 0)

                    (heap.load32_i32
                        (i64.imm 0x100) 4)

                    (heap.load32_i16_u
                        (i64.imm 0x100) 6)

                    (heap.load32_i16_s
                        (i64.imm 0x100) 6)

                    (heap.load32_i8_u
                        (i64.imm 0x100) 7)

                    (heap.load32_i8_s
                        (i64.imm 0x100) 7)

                    ;; load heaps, group 1
                    (heap.load32_f32
                        (i64.imm 0x200) 0)

                    (heap.load64_f64
                        (i64.imm 0x300) 0)

                    ;; load heaps, group 2
                    (heap.load64_i64
                        (i64.imm 0x400)
                        (; ommit the param 'offset' ;)
                    )

                    (heap.load32_i32
                        (i64.imm 0x500)
                        (; ommit the param 'offset' ;)
                    )
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

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
