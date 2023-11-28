// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::time::Instant;

use ancvm_assembler::utils::helper_generate_single_module_image_binary_from_assembly;
use ancvm_process::{
    in_memory_program_source::InMemoryProgramSource,
    multithread_program::run_program_in_multithread,
};
use ancvm_types::envcallcode::EnvCallCode;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_multithread_run_program_in_multithread() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    let module_binaries = helper_generate_single_module_image_binary_from_assembly(
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

    let module_binaries = helper_generate_single_module_image_binary_from_assembly(&format!(
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

    let module_binaries = helper_generate_single_module_image_binary_from_assembly(&format!(
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
                            ;; now the operand(s) on the top of stack is: (child thread id)
                        )
                        ;; now the operand(s) on the top of stack is: (child thread exit code, thread result)
                    )
                    ;; now the operand(s) on the top of stack is: (child thread exit code)
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

    // start data: 0x11, 0x13, 0x17, 0x19, 0x23, 0x29, 0x31, 0x37
    //
    //                     0   3  4   7   (offset in byte)
    //                    |=====  =====|
    //                     |      | copy 4 bytes
    //                     |      |
    //              /------|------/
    //              |      |
    //              v      v
    //              0   3  4   7  (offset in byte)
    // heap: 0x200 |=====  =====|
    //       0x23, 0x29, 0x31, 0x37, 0x11, 0x13, 0x17, 0x19
    //       ----------------------------------------------
    //       |
    //       \---> exit code 0x19171311_37312923

    let module_binaries = helper_generate_single_module_image_binary_from_assembly(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test (result i64)
                (code
                    ;; resize heap to 1 page, because the heap is required to read the thread_start_data.
                    (drop
                        (heap.resize (i32.imm 1))
                    )

                    ;; check the data length
                    (when
                        (i32.ne
                            (envcall {ENV_CALL_CODE_THREAD_START_DATA_LENGTH})
                            (i32.imm 8)
                        )
                        (debug 0)
                    )

                    ;; read 4 bytes data from offset 0 to heap (0x100+4)
                    (when
                        (i32.ne
                            (envcall {ENV_CALL_CODE_THREAD_START_DATA_READ}
                                (i32.imm 0)         ;; offset
                                (i32.imm 4)         ;; length
                                (i64.imm 0x104)     ;; dst address
                            )
                            (i32.imm 4)
                        )
                        (debug 1)
                    )

                    ;; read 8 bytes data from offset 4 to heap (0x100)
                    ;; actual read length should be 4
                    (when
                        (i32.ne
                            (envcall {ENV_CALL_CODE_THREAD_START_DATA_READ}
                                (i32.imm 4)         ;; offset
                                (i32.imm 8)         ;; length
                                (i64.imm 0x100)     ;; temporary dst address
                            )
                            (i32.imm 4)
                        )
                        (debug 2)
                    )

                    (heap.load64_i64 0 (i64.imm 0x100))
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
    assert_eq!(result0.unwrap(), 0x19171311_37312923);
}

#[test]
fn test_assemble_multithread_thread_running_status() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    let module_binaries = helper_generate_single_module_image_binary_from_assembly(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test (result i64)
                (local $tid i32)
                (local $last_status i32)
                (local $last_result i32)
                (code
                    ;; create child thread
                    (local.store32 $tid
                        (envcall {ENV_CALL_CODE_THREAD_CREATE}
                            (macro.get_func_pub_index $child)   ;; function pub index
                            (i32.imm 0)         ;; thread_start_data_address
                            (i32.imm 0)         ;; thread_start_data_length
                        )
                    )

                    ;; pause 500ms to ensure the child thread is running
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP} (i64.imm 500))

                    ;; get the runnting status
                    (local.store32 $last_status
                        (local.store32 $last_result
                            (envcall {ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                                (local.load32_i32 $tid)
                            )
                        )
                    )

                    ;; the $last_result should be 0=success
                    (when
                        (local.load32_i32 $last_result)
                        (debug 0)
                    )

                    ;; the $last_status should be 0=running
                    (when
                        (local.load32_i32 $last_status)
                        (debug 1)
                    )

                    ;; pause 1000ms to ensure the child thread is finish
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP} (i64.imm 1000))

                    ;; get the runnting status
                    (local.store32 $last_status
                        (local.store32 $last_result
                            (envcall {ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                                (local.load32_i32 $tid)
                            )
                        )
                    )

                    ;; the $last_result should be 0=success
                    (when
                        (local.load32_i32 $last_result)
                        (debug 2)
                    )

                    ;; the $last_status should be 1=finish
                    (when
                        (i32.ne
                            (local.load32_i32 $last_status)
                            (i32.imm 1)
                        )
                        (debug 3)
                    )

                    ;; try to get the thrad running status of a non-existent thread.
                    ;; get the runnting status
                    (local.store32 $last_status
                        (local.store32 $last_result
                            (envcall {ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                                (i32.imm 0x1113)
                            )
                        )
                    )

                    ;; the $last_result should be 1=failure
                    (when
                        (i32.ne
                            (local.load32_i32 $last_result)
                            (i32.imm 1)
                        )
                        (debug 4)
                    )

                    ;; wait and collect the child thread
                    (drop
                        (envcall {ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                            (local.load32_i32 $tid)
                        )
                    )
                )
            )

            (fn $child (result i64)
                (code
                    ;; sleep 1000ms
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP} (i64.imm 1000))

                    ;; set thread_exit_code as 0x17
                    (i64.imm 0x17)
                )
            )
        )
        "#,
        ENV_CALL_CODE_THREAD_SLEEP = (EnvCallCode::thread_sleep as u32),
        ENV_CALL_CODE_THREAD_RUNNING_STATUS = (EnvCallCode::thread_running_status as u32),
        ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT = (EnvCallCode::thread_wait_and_collect as u32),
        ENV_CALL_CODE_THREAD_CREATE = (EnvCallCode::thread_create as u32)
    ));

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let result0 = run_program_in_multithread(program_source0, vec![]);
    assert_eq!(result0.unwrap(), 0x17);
}

#[test]
fn test_assemble_multithread_thread_terminate() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    let module_binaries = helper_generate_single_module_image_binary_from_assembly(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test (result i64)
                (local $tid i32)
                (local $last_status i32)
                (local $last_result i32)
                (code
                    ;; create child thread
                    (local.store32 $tid
                        (envcall {ENV_CALL_CODE_THREAD_CREATE}
                            (macro.get_func_pub_index $child)   ;; function pub index
                            (i32.imm 0)         ;; thread_start_data_address
                            (i32.imm 0)         ;; thread_start_data_length
                        )
                    )

                    ;; pause 500ms to ensure the child thread is running
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP} (i64.imm 500))

                    ;; get the runnting status
                    (local.store32 $last_status
                        (local.store32 $last_result
                            (envcall {ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                                (local.load32_i32 $tid)
                            )
                        )
                    )

                    ;; the $last_result should be 0=success
                    (when
                        (local.load32_i32 $last_result)
                        (debug 0)
                    )

                    ;; the $last_status should be 0=running
                    (when
                        (local.load32_i32 $last_status)
                        (debug 1)
                    )

                    ;; terminate the child thread
                    (envcall {ENV_CALL_CODE_THREAD_TERMINATE} (local.load32_i32 $tid))

                    ;; get the runnting status
                    (local.store32 $last_status
                        (local.store32 $last_result
                            (envcall {ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                                (local.load32_i32 $tid)
                            )
                        )
                    )

                    ;; the $last_result should be 1=failure (thread_not_found)
                    (when
                        (i32.ne
                            (local.load32_i32 $last_result)
                            (i32.imm 1)
                        )
                        (debug 2)
                    )

                    ;; try to collect the child thread
                    (local.store32 $last_result
                        (envcall {ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                            (local.load32_i32 $tid)
                        )
                    )

                    ;; the $last_result should be 1=failure (thread_not_found)
                    (when
                        (i32.ne
                            (local.load32_i32 $last_result)
                            (i32.imm 1)
                        )
                        (debug 3)
                    )

                    ;; set thread_exit_code as 0x23
                    (i64.imm 0x23)
                )
            )

            (fn $child (result i64)
                (code
                    ;; sleep 5000ms
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP} (i64.imm 5000))

                    ;; set thread_exit_code as 0x19
                    (i64.imm 0x19)
                )
            )
        )
        "#,
        ENV_CALL_CODE_THREAD_SLEEP = (EnvCallCode::thread_sleep as u32),
        ENV_CALL_CODE_THREAD_TERMINATE = (EnvCallCode::thread_terminate as u32),
        ENV_CALL_CODE_THREAD_RUNNING_STATUS = (EnvCallCode::thread_running_status as u32),
        ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT = (EnvCallCode::thread_wait_and_collect as u32),
        ENV_CALL_CODE_THREAD_CREATE = (EnvCallCode::thread_create as u32)
    ));

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let result0 = run_program_in_multithread(program_source0, vec![]);
    assert_eq!(result0.unwrap(), 0x23);
}

#[test]
fn test_assemble_multithread_thread_message_send_and_receive() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    // main thread        child thread
    //              send
    // 0x11        -----> 0x11
    //              send
    // 0x13        <----- 0x13
    //
    //              exit
    // 0x17        <----- 0x17

    let module_binaries = helper_generate_single_module_image_binary_from_assembly(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test (result i64)
                (local $tid i32)
                (local $last_length i32)
                (local $last_result i32)
                (local $last_status i32)
                (code
                    ;; resize heap to 1 page, because the heap is required to send/receive the message.
                    (drop
                        (heap.resize (i32.imm 1))
                    )

                    ;; create new thread
                    (local.store32 $tid
                        (envcall {ENV_CALL_CODE_THREAD_CREATE}
                            (macro.get_func_pub_index $child)   ;; function pub index
                            (i32.imm 0)         ;; thread_start_data_address
                            (i32.imm 0)         ;; thread_start_data_length
                        )
                    )

                    ;; write data 0x11 (to be sent) to heap at address 0x100
                    (heap.store32 0
                        (i64.imm 0x100)     ;; address
                        (i32.imm 0x11)      ;; data
                    )

                    ;; send data to child thread
                    (local.store32 $last_result
                        (envcall {ENV_CALL_CODE_THREAD_SEND_MSG}
                            (local.load32_i32 $tid)     ;; child thread id
                            (i64.imm 0x100)             ;; data src address
                            (i32.imm 4)                 ;; data length
                        )
                    )

                    ;; the last_result should be 0=success
                    (when
                        (local.load32_i32 $last_result)
                        (debug 0)
                    )

                    ;; receive data from child thread
                    (local.store32 $last_length
                        (local.store32 $last_result
                            (envcall {ENV_CALL_CODE_THREAD_RECEIVE_MSG}
                                (local.load32_i32 $tid)
                            )
                        )
                    )

                    ;; last_result should be 0=success
                    (when
                        (local.load32_i32 $last_result)
                        (debug 1)
                    )

                    ;; last_length should be 4
                    (when
                        (i32.ne
                            (local.load32_i32 $last_length)
                            (i32.imm 4)
                        )
                        (debug 2)
                    )

                    ;; test function 'thread_msg_length'
                    (local.store32 $last_length
                        (envcall {ENV_CALL_CODE_THREAD_MSG_LENGTH})
                    )

                    ;; last_length should be 4
                    (when
                        (i32.ne
                            (local.load32_i32 $last_length)
                            (i32.imm 4)
                        )
                        (debug 3)
                    )

                    ;; test function 'thread_msg_read'
                    ;; try to read 8 bytes, actually read 4 bytes
                    (local.store32 $last_length
                        (envcall {ENV_CALL_CODE_THREAD_MSG_READ}
                            (i32.imm 0)         ;; offset
                            (i32.imm 8)         ;; length
                            (i64.imm 0x200)     ;; dst address
                        )
                    )

                    ;; last_length should be 4
                    (when
                        (i32.ne
                            (local.load32_i32 $last_length)
                            (i32.imm 4)
                        )
                        (debug 4)
                    )

                    ;; check the received data
                    (when
                        (i32.ne
                            (heap.load32_i32 0 (i64.imm 0x200))
                            (i32.imm 0x13)
                        )
                        (debug 5)
                    )

                    ;; the status of child thread is changing to 'finish', wait 500ms
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP} (i64.imm 500))

                    (local.store32 $last_status
                        (local.store32 $last_result
                            (envcall {ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                                (local.load32_i32 $tid)
                            )
                        )
                    )

                    ;; last_result should be 0=success
                    (when
                        (local.load32_i32 $last_result)
                        (debug 6)
                    )

                    ;; last_status should be 1=finish
                    (when
                        (i32.ne
                            (local.load32_i32 $last_status)
                            (i32.imm 1)
                        )
                        (debug 7)
                    )

                    ;; collect the child thread,
                    ;; and use the exit code of child thread as the
                    ;; main thread exit code
                    (drop
                        (envcall {ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                            (local.load32_i32 $tid)
                        )
                    )
                )
            )

            (fn $child (result i64)
                (local $last_length i32)
                (local $last_result i32)
                (code
                    ;; resize heap to 1 page, because the heap is required to send/receive the message.
                    (drop
                        (heap.resize (i32.imm 1))
                    )

                    ;; receive data from parent
                    (local.store32 $last_length
                        (envcall {ENV_CALL_CODE_THREAD_RECEIVE_MSG_FROM_PARENT})
                    )

                    ;; last_length should be 4
                    (when
                        (i32.ne
                            (local.load32_i32 $last_length)
                            (i32.imm 4)
                        )
                        (debug 0)
                    )

                    ;; test function 'thread_msg_length'
                    (local.store32 $last_length
                        (envcall {ENV_CALL_CODE_THREAD_MSG_LENGTH})
                    )

                    ;; last_length should be 4
                    (when
                        (i32.ne
                            (local.load32_i32 $last_length)
                            (i32.imm 4)
                        )
                        (debug 1)
                    )

                    ;; test function 'thread_msg_read'
                    ;; try to read 8 bytes, actually read 4 bytes
                    (local.store32 $last_length
                        (envcall {ENV_CALL_CODE_THREAD_MSG_READ}
                            (i32.imm 0)         ;; offset
                            (i32.imm 8)         ;; length
                            (i64.imm 0x100)     ;; dst address
                        )
                    )

                    ;; last_length should be 4
                    (when
                        (i32.ne
                            (local.load32_i32 $last_length)
                            (i32.imm 4)
                        )
                        (debug 2)
                    )

                    ;; check the received data
                    (when
                        (i32.ne
                            (heap.load32_i32 0 (i64.imm 0x100))
                            (i32.imm 0x11)
                        )
                        (debug 3)
                    )

                    ;; set the data to be sent
                    (heap.store32 0
                        (i64.imm 0x200)     ;; address
                        (i32.imm 0x13)      ;; data
                    )

                    ;; send data to parent
                    (local.store32 $last_result
                        (envcall {ENV_CALL_CODE_THREAD_SEND_MSG_TO_PARENT}
                            (i64.imm 0x200)
                            (i32.imm 4)
                        )
                    )

                    ;; last_result should be 0-success
                    (when
                        (local.load32_i32 $last_result)
                        (debug 4)
                    )

                    ;; exit code
                    (i64.imm 0x17)
                )
            )
        )
        "#,
        ENV_CALL_CODE_THREAD_CREATE = (EnvCallCode::thread_create as u32),
        ENV_CALL_CODE_THREAD_SEND_MSG = (EnvCallCode::thread_send_msg as u32),
        ENV_CALL_CODE_THREAD_RECEIVE_MSG = (EnvCallCode::thread_receive_msg as u32),
        ENV_CALL_CODE_THREAD_MSG_LENGTH = (EnvCallCode::thread_msg_length as u32),
        ENV_CALL_CODE_THREAD_MSG_READ = (EnvCallCode::thread_msg_read as u32),
        ENV_CALL_CODE_THREAD_SEND_MSG_TO_PARENT = (EnvCallCode::thread_send_msg_to_parent as u32),
        ENV_CALL_CODE_THREAD_RECEIVE_MSG_FROM_PARENT =
            (EnvCallCode::thread_receive_msg_from_parent as u32),
        ENV_CALL_CODE_THREAD_RUNNING_STATUS = (EnvCallCode::thread_running_status as u32),
        ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT = (EnvCallCode::thread_wait_and_collect as u32),
        ENV_CALL_CODE_THREAD_SLEEP = (EnvCallCode::thread_sleep as u32),
    ));

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let result0 = run_program_in_multithread(program_source0, vec![]);
    assert_eq!(result0.unwrap(), 0x17);
}

#[test]
fn test_assemble_multithread_thread_message_forward() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    // main thread      child thread 0      child thread 1
    //
    // 0x11   <------------0x11
    //   |
    //   \----------------------------------> 0x11
    //
    // 0x19
    //   |
    //   \-- exit code

    let module_binaries = helper_generate_single_module_image_binary_from_assembly(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test (result i64)
                (local $tid0 i32)
                (local $tid1 i32)
                (code
                    ;; resize heap to 1 page, because the heap is required to send/receive the message.
                    (drop
                        (heap.resize (i32.imm 1))
                    )

                    ;; create child thread 0 (t0)
                    (local.store32 $tid0
                        (envcall {ENV_CALL_CODE_THREAD_CREATE}
                            (macro.get_func_pub_index $child0)   ;; function pub index
                            (i32.imm 0)         ;; thread_start_data_address
                            (i32.imm 0)         ;; thread_start_data_length
                        )
                    )

                    ;; create child thread 1 (t1)
                    (local.store32 $tid1
                        (envcall {ENV_CALL_CODE_THREAD_CREATE}
                            (macro.get_func_pub_index $child1)   ;; function pub index
                            (i32.imm 0)         ;; thread_start_data_address
                            (i32.imm 0)         ;; thread_start_data_length
                        )
                    )

                    ;; receive message from t0
                    (envcall {ENV_CALL_CODE_THREAD_RECEIVE_MSG}
                        (local.load32_i32 $tid0)
                    )

                    ;; read message to heap
                    (envcall {ENV_CALL_CODE_THREAD_MSG_READ}
                        (i32.imm 0)         ;; offset
                        (i32.imm 4)         ;; length
                        (i64.imm 0x100)     ;; dst address
                    )

                    ;; send message to t1
                    (envcall {ENV_CALL_CODE_THREAD_SEND_MSG}
                        (local.load32_i32 $tid1)    ;; child thread id
                        (i64.imm 0x100)             ;; data src address
                        (i32.imm 4)                 ;; data length
                    )

                    (when
                        (i64.ne
                            ;; collect t0
                            (drop
                                (envcall {ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                                    (local.load32_i32 $tid0)
                                )
                            )
                            (i64.imm 0x13)
                        )
                        (debug 0)
                    )


                    (when
                        (i64.ne
                            ;; collect t1
                            (drop
                                (envcall {ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                                    (local.load32_i32 $tid1)
                                )
                            )
                            (i64.imm 0x17)
                        )
                        (debug 0)
                    )

                    ;; exit code
                    (i64.imm 0x19)
                )
            )

            (fn $child0 (result i64)
                (code
                    ;; resize heap to 1 page, because the heap is required to send/receive the message.
                    (drop
                        (heap.resize (i32.imm 1))
                    )

                    ;; set the data to be sent
                    (heap.store32 0
                        (i64.imm 0x100)     ;; address
                        (i32.imm 0x11)      ;; data
                    )

                    ;; send data to parent
                    (envcall {ENV_CALL_CODE_THREAD_SEND_MSG_TO_PARENT}
                        (i64.imm 0x100)
                        (i32.imm 4)
                    )

                    ;; exit code 0
                    (i64.imm 0x13)
                )
            )

            (fn $child1 (result i64)
                (code
                    ;; resize heap to 1 page, because the heap is required to send/receive the message.
                    (drop
                        (heap.resize (i32.imm 1))
                    )

                    ;; receive data from parent
                    (envcall {ENV_CALL_CODE_THREAD_RECEIVE_MSG_FROM_PARENT})

                    (envcall {ENV_CALL_CODE_THREAD_MSG_READ}
                        (i32.imm 0)         ;; offset
                        (i32.imm 4)         ;; length
                        (i64.imm 0x100)     ;; dst address
                    )

                    ;; check the received data
                    (when
                        (i32.ne
                            (heap.load32_i32 0 (i64.imm 0x100))
                            (i32.imm 0x11)
                        )
                        (debug 0)
                    )

                    ;; exit code 0
                    (i64.imm 0x17)
                )
            )
        )
        "#,
        ENV_CALL_CODE_THREAD_CREATE = (EnvCallCode::thread_create as u32),
        ENV_CALL_CODE_THREAD_SEND_MSG = (EnvCallCode::thread_send_msg as u32),
        ENV_CALL_CODE_THREAD_RECEIVE_MSG = (EnvCallCode::thread_receive_msg as u32),
        ENV_CALL_CODE_THREAD_MSG_READ = (EnvCallCode::thread_msg_read as u32),
        ENV_CALL_CODE_THREAD_SEND_MSG_TO_PARENT = (EnvCallCode::thread_send_msg_to_parent as u32),
        ENV_CALL_CODE_THREAD_RECEIVE_MSG_FROM_PARENT =
            (EnvCallCode::thread_receive_msg_from_parent as u32),
        ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT = (EnvCallCode::thread_wait_and_collect as u32),
    ));

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let result0 = run_program_in_multithread(program_source0, vec![]);
    assert_eq!(result0.unwrap(), 0x19);
}

#[test]
fn test_assemble_multithread_thread_sleep() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    let module_binaries = helper_generate_single_module_image_binary_from_assembly(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
                (result i64)
                (code
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP}
                        (i64.imm 1000)
                    )
                    ;; exit code
                    (i64.imm 0x13)
                )
            )
        )
        "#,
        ENV_CALL_CODE_THREAD_SLEEP = (EnvCallCode::thread_sleep as u32)
    ));

    let program_source0 = InMemoryProgramSource::new(module_binaries);

    let before = Instant::now();
    let result0 = run_program_in_multithread(program_source0, vec![]);
    assert_eq!(result0.unwrap(), 0x13);
    let after = Instant::now();

    let duration = after.duration_since(before);
    let ms = duration.as_millis() as u64;
    assert!(ms > 500);
}
