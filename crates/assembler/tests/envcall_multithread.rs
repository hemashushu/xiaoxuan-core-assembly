// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

mod utils;

use ancvm_runtime::{
    in_memory_program_source::InMemoryProgramSource,
    multithread_program::run_program_in_multithread,
};
use ancvm_types::envcallcode::EnvCallCode;

use crate::utils::assemble_single_module;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_multithread_run_program_in_multithread() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    let module_binaries = assemble_single_module(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test (results i64)
                (code
                    (i64.imm 0x11)
                )
            )
        )
        "#,
    );

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let result0 = run_program_in_multithread(program_source0, vec![]);

    const EXPECT_THREAD_EXIT_CODE: u64 = 0x11;
    assert_eq!(result0.unwrap(), EXPECT_THREAD_EXIT_CODE);
}

#[test]
fn test_assemble_multithread_thread_id() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    let module_binaries = assemble_single_module(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test (results i64)
                (code
                    (i64.extend_i32_u
                        (envcall {ENV_CALL_CODE_THREAD_ID})
                    )
                )
            )
        )
        "#,
        ENV_CALL_CODE_THREAD_ID = (EnvCallCode::thread_id as u32)
    ));

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let result0 = run_program_in_multithread(program_source0, vec![]);

    const FIRST_CHILD_THREAD_ID: u64 = 1;
    assert_eq!(result0.unwrap(), FIRST_CHILD_THREAD_ID);
}

#[test]
fn test_assemble_multithread_thread_create() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    let module_binaries = assemble_single_module(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test (result i64)
                (code
                    (drop
                        (envcall {ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                            (envcall {ENV_CALL_CODE_THREAD_CREATE}
                                (macro.get_func_pub_index $child)   ;; function pub index
                                (i32.imm 0)         ;; thread_start_data_address
                                (i32.imm 0)         ;; thread_start_data_length
                            )
                            ;; now the operand on the top of stack is the child thread id
                        )
                        ;; now the operand on the top of stack is the (child thread exit code, thread result)
                    )
                    ;; now the operand on the top of stack is the (child thread exit code)
                )
            )

            (fn $child (result i64)
                (code
                    (i64.imm 0x13)  ;; set thread_exit_code as 0x13
                )
            )
        )
        "#,
        ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT = (EnvCallCode::thread_wait_and_collect as u32),
        ENV_CALL_CODE_THREAD_CREATE = (EnvCallCode::thread_create as u32)
    ));

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let result0 = run_program_in_multithread(program_source0, vec![]);
    assert_eq!(result0.unwrap(), 0x13);
}

#[test]
fn test_assemble_multithread_thread_start_data() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    //                            0        8   (offset in byte)
    // start data:               |=========|
    //                                | copy 8 bytes
    //                                v
    //                         0 1 2 3 4 5 6 7  (offset in byte)
    // heap:            0x100 |===|===|===|===|
    //                         |           |
    // start data length --\   \-\     /---/
    //                     |     |     |
    //                     V     V     V
    // local var u64:    | u32 | u16 | u16 |
    //                   -------------------
    //                   low     |      high
    //                           \---> exit code 0x37_31_13_11_00_00_00_08

    let module_binaries = assemble_single_module(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test (result i64)
                (local $exit_code i64)
                (code
                    ;; resize heap to 1 page, because the heap is required to read the thread_start_data.
                    (drop
                        (heap.resize (i32.imm 1))
                    )
                    (local.store16 $exit_code 0
                        (envcall {ENV_CALL_CODE_THREAD_START_DATA_LENGTH})
                    )
                    (envcall {ENV_CALL_CODE_THREAD_START_DATA_READ}
                        (i32.imm 0)         ;; offset
                        (i32.imm 8)         ;; length
                        (i64.imm 0x100)     ;; dst address
                    )
                    (local.store16 $exit_code 4
                        (heap.load32_i16_u 0 (i64.imm 0x100))
                    )
                    (local.store16 $exit_code 6
                        (heap.load32_i16_u 6 (i64.imm 0x100))
                    )
                    (local.load64_i64 $exit_code)
                )
            )
        )
        "#,
        ENV_CALL_CODE_THREAD_START_DATA_LENGTH = (EnvCallCode::thread_start_data_length as u32),
        ENV_CALL_CODE_THREAD_START_DATA_READ = (EnvCallCode::thread_start_data_read as u32)
    ));

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let result0 = run_program_in_multithread(
        program_source0,
        vec![0x11, 0x13, 0x17, 0x19, 0x23, 0x29, 0x31, 0x37],
    );
    assert_eq!(result0.unwrap(), 0x37_31_13_11_00_00_00_08);
}
