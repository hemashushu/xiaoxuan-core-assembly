// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::time::Instant;

use ancasm_assembler::utils::helper_generate_module_image_binary_from_str;
use ancvm_processor::{
    in_memory_program_resource::InMemoryProgramResource,
    process::start_program_in_multithread
};
use ancvm_types::envcallcode::EnvCallCode;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_multithread_run_program_in_multithread() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (results i64)
                (code
                    (i64.imm 0x11)
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let result0 = start_program_in_multithread(program_resource0, vec![]);

    const EXPECT_THREAD_EXIT_CODE: u64 = 0x11;
    assert_eq!(result0.unwrap(), EXPECT_THREAD_EXIT_CODE);
}

#[test]
fn test_assemble_multithread_thread_id() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    let module_binary = helper_generate_module_image_binary_from_str(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (results i64)
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

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let result0 = start_program_in_multithread(program_resource0, vec![]);

    const FIRST_CHILD_THREAD_ID: u64 = 1;
    assert_eq!(result0.unwrap(), FIRST_CHILD_THREAD_ID);
}

#[test]
fn test_assemble_multithread_thread_create() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    let module_binary = helper_generate_module_image_binary_from_str(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (result i64)
                (local $temp i32)   // for dropping operand
                (code
                    // (drop
                    (local.store32 $temp
                        (envcall {ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                            (envcall {ENV_CALL_CODE_THREAD_CREATE}
                                (macro.get_function_public_index $child)   // function pub index
                                (i32.imm 0)         // thread_start_data_address
                                (i32.imm 0)         // thread_start_data_length
                            )
                            // now the operand(s) on the top of stack is: (child thread id)
                        )
                        // now the operand(s) on the top of stack is: (child thread exit code, thread result)
                    )
                    // now the operand(s) on the top of stack is: (child thread exit code)
                )
            )

            (function $child (result i64)
                (code
                    (i64.imm 0x13)  // set thread_exit_code as 0x13
                )
            )
        )
        "#,
        ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT = (EnvCallCode::thread_wait_and_collect as u32),
        ENV_CALL_CODE_THREAD_CREATE = (EnvCallCode::thread_create as u32)
    ));

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let result0 = start_program_in_multithread(program_resource0, vec![]);
    assert_eq!(result0.unwrap(), 0x13);
}

#[test]
fn test_assemble_multithread_thread_local_storage() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    let module_binary = helper_generate_module_image_binary_from_str(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (data $buf (read_write i32 0))
            (function $test (result i64)
                (local $tid i32)
                (code
                    // resize heap to 1 page
                    // (drop
                        (heap.resize (i32.imm 1))
                    // )

                    // write value to data
                    (data.store32 $buf (i32.imm 0x11))

                    // write value to heap at address 0x100
                    (heap.store32
                        (i64.imm 0x100)     // address
                        (i32.imm 0x13)      // data
                    )

                    // create child thread
                    (local.store32 $tid
                        (envcall {ENV_CALL_CODE_THREAD_CREATE}
                            (macro.get_function_public_index $child)   // function pub index
                            (i32.imm 0)         // thread_start_data_address
                            (i32.imm 0)         // thread_start_data_length
                        )
                    )

                    // pause 500ms to ensure the child thread is running
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP} (i64.imm 500))

                    // wait and collect the child thread
                    // (drop
                        (envcall {ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                            (local.load32_i32 $tid)
                        )
                    // )

                    // check value in data
                    (when
                        (i32.ne
                            (data.load32_i32 $buf)
                            (i32.imm 0x11)
                        )
                        (debug 0)
                    )

                    // check value in heap
                    (when
                        (i32.ne
                            (heap.load32_i32
                                (i64.imm 0x100)
                            )
                            (i32.imm 0x13)
                        )
                        (debug 1)
                    )

                    // set thread_exit_code
                    (i64.imm 0)
                )
            )

            (function $child (result i64)
                (code
                    // resize heap to 1 page
                    // (drop
                        (heap.resize (i32.imm 1))
                    // )

                    // check value in data
                    (when
                        (i32.ne
                            (data.load32_i32 $buf)
                            (i32.imm 0)
                        )
                        (debug 0)
                    )

                    // the initial data of heap should be garbage
                    // so it doesn't need to be checked

                    // write value to data
                    (data.store32 $buf (i32.imm 0x23))

                    // write value to heap at address 0x100
                    (heap.store32
                        (i64.imm 0x100)     // address
                        (i32.imm 0x29)      // data
                    )

                    // set thread_exit_code
                    (i64.imm 0)
                )
            )
        )
        "#,
        ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT = (EnvCallCode::thread_wait_and_collect as u32),
        ENV_CALL_CODE_THREAD_SLEEP = (EnvCallCode::thread_sleep as u32),
        ENV_CALL_CODE_THREAD_CREATE = (EnvCallCode::thread_create as u32)
    ));

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let result0 = start_program_in_multithread(program_resource0, vec![]);
    assert_eq!(result0.unwrap(), 0);
}

#[test]
fn test_assemble_multithread_thread_sleep() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    let module_binary = helper_generate_module_image_binary_from_str(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test
                (result i64)
                (code
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP}
                        (i64.imm 1000)
                    )
                    // exit code
                    (i64.imm 0x13)
                )
            )
        )
        "#,
        ENV_CALL_CODE_THREAD_SLEEP = (EnvCallCode::thread_sleep as u32)
    ));

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);

    let before = Instant::now();
    let result0 = start_program_in_multithread(program_resource0, vec![]);
    assert_eq!(result0.unwrap(), 0x13);
    let after = Instant::now();

    let duration = after.duration_since(before);
    let ms = duration.as_millis() as u64;
    assert!(ms > 500);
}

#[test]
fn test_assemble_multithread_thread_start_data() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    // start data: 0x11, 0x13, 0x17, 0x19, 0x23, 0x29, 0x31, 0x37
    //
    //               0   3  4   7   (offset in byte)
    //              |=====||=====|
    //  copy 4 bytes |      | copy 4 bytes
    //               |      |
    //               |      \----------\
    //               |                 |
    //               v                 v
    //               0   3             0   3
    //  local part0 |=====|    part 1 |=====|
    //
    //        i32 0x19171311     i32 0x37312923

    let module_binary = helper_generate_module_image_binary_from_str(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (result i64)
                (local $part0 i32)
                (local $part1 i32)
                (code
                    // check the data length
                    (when
                        (i32.ne
                            (envcall {ENV_CALL_CODE_THREAD_START_DATA_LENGTH})
                            (i32.imm 8)
                        )
                        (debug 0)
                    )

                    // read 4 bytes data from offset 0 to local variable $part0
                    (when
                        (i32.ne
                            (envcall {ENV_CALL_CODE_THREAD_START_DATA_READ}
                                (i32.imm 0)                 // offset
                                (i32.imm 4)                 // length
                                (host.addr_local $part0)    // dst address
                            )
                            (i32.imm 4)
                        )
                        (debug 1)
                    )

                    // check value
                    (when
                        (i32.ne
                            (local.load32_i32 $part0)
                            (i32.imm 0x19171311)
                        )
                        (debug 2)
                    )

                    // read 8 bytes data from offset 4 to local variable $part0
                    // actual read length should be 4
                    (when
                        (i32.ne
                            (envcall {ENV_CALL_CODE_THREAD_START_DATA_READ}
                                (i32.imm 4)                 // offset
                                (i32.imm 8)                 // length
                                (host.addr_local $part1)    // dst address
                            )
                            (i32.imm 4)
                        )
                        (debug 3)
                    )

                    // check value
                    (when
                        (i32.ne
                            (local.load32_i32 $part1)
                            (i32.imm 0x37312923)
                        )
                        (debug 4)
                    )

                    (i64.imm 0)
                )
            )
        )
        "#,
        ENV_CALL_CODE_THREAD_START_DATA_LENGTH = (EnvCallCode::thread_start_data_length as u32),
        ENV_CALL_CODE_THREAD_START_DATA_READ = (EnvCallCode::thread_start_data_read as u32)
    ));

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let result0 = start_program_in_multithread(
        program_resource0,
        vec![0x11, 0x13, 0x17, 0x19, 0x23, 0x29, 0x31, 0x37],
    );
    assert_eq!(result0.unwrap(), 0);
}

#[test]
fn test_assemble_multithread_thread_running_status() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    let module_binary = helper_generate_module_image_binary_from_str(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (result i64)
                (local $tid i32)
                (local $last_status i32)
                (local $last_result i32)
                (local $temp i32)   // for dropping operand
                (code
                    // create child thread
                    (local.store32 $tid
                        (envcall {ENV_CALL_CODE_THREAD_CREATE}
                            (macro.get_function_public_index $child)   // function pub index
                            (i32.imm 0)         // thread_start_data_address
                            (i32.imm 0)         // thread_start_data_length
                        )
                    )

                    // pause 500ms to ensure the child thread is running
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP} (i64.imm 500))

                    // get the runnting status
                    (local.store32 $last_status
                        (local.store32 $last_result
                            (envcall {ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                                (local.load32_i32 $tid)
                            )
                        )
                    )

                    // the $last_result should be 0=success
                    (when
                        (local.load32_i32 $last_result)
                        (debug 0)
                    )

                    // the $last_status should be 0=running
                    (when
                        (local.load32_i32 $last_status)
                        (debug 1)
                    )

                    // pause 1000ms to ensure the child thread is finish
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP} (i64.imm 1000))

                    // get the runnting status
                    (local.store32 $last_status
                        (local.store32 $last_result
                            (envcall {ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                                (local.load32_i32 $tid)
                            )
                        )
                    )

                    // the $last_result should be 0=success
                    (when
                        (local.load32_i32 $last_result)
                        (debug 2)
                    )

                    // the $last_status should be 1=finish
                    (when
                        (i32.ne
                            (local.load32_i32 $last_status)
                            (i32.imm 1)
                        )
                        (debug 3)
                    )

                    // try to get the thrad running status of a non-existent thread.
                    // get the runnting status
                    (local.store32 $last_status
                        (local.store32 $last_result
                            (envcall {ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                                (i32.imm 0x1113)
                            )
                        )
                    )

                    // the $last_result should be 1=failure
                    (when
                        (i32.ne
                            (local.load32_i32 $last_result)
                            (i32.imm 1)
                        )
                        (debug 4)
                    )

                    // wait and collect the child thread
                    // (drop
                    (local.store32 $temp
                        (envcall {ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                            (local.load32_i32 $tid)
                        )
                    )
                )
            )

            (function $child (result i64)
                (code
                    // sleep 1000ms
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP} (i64.imm 1000))

                    // set thread_exit_code as 0x17
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

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let result0 = start_program_in_multithread(program_resource0, vec![]);
    assert_eq!(result0.unwrap(), 0x17);
}

#[test]
fn test_assemble_multithread_thread_terminate() {
    // the signature of 'thread start function' must be
    // () -> (i64)

    let module_binary = helper_generate_module_image_binary_from_str(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (result i64)
                (local $tid i32)
                (local $last_status i32)
                (local $last_result i32)
                (code
                    // create child thread
                    (local.store32 $tid
                        (envcall {ENV_CALL_CODE_THREAD_CREATE}
                            (macro.get_function_public_index $child)   // function pub index
                            (i32.imm 0)         // thread_start_data_address
                            (i32.imm 0)         // thread_start_data_length
                        )
                    )

                    // pause 500ms to ensure the child thread is running
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP} (i64.imm 500))

                    // get the runnting status
                    (local.store32 $last_status
                        (local.store32 $last_result
                            (envcall {ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                                (local.load32_i32 $tid)
                            )
                        )
                    )

                    // the $last_result should be 0=success
                    (when
                        (local.load32_i32 $last_result)
                        (debug 0)
                    )

                    // the $last_status should be 0=running
                    (when
                        (local.load32_i32 $last_status)
                        (debug 1)
                    )

                    // terminate the child thread
                    (envcall {ENV_CALL_CODE_THREAD_TERMINATE} (local.load32_i32 $tid))

                    // get the runnting status
                    (local.store32 $last_status
                        (local.store32 $last_result
                            (envcall {ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                                (local.load32_i32 $tid)
                            )
                        )
                    )

                    // the $last_result should be 1=failure (thread_not_found)
                    (when
                        (i32.ne
                            (local.load32_i32 $last_result)
                            (i32.imm 1)
                        )
                        (debug 2)
                    )

                    // try to collect the child thread
                    (local.store32 $last_result
                        (envcall {ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                            (local.load32_i32 $tid)
                        )
                    )

                    // the $last_result should be 1=failure (thread_not_found)
                    (when
                        (i32.ne
                            (local.load32_i32 $last_result)
                            (i32.imm 1)
                        )
                        (debug 3)
                    )

                    // set thread_exit_code as 0x23
                    (i64.imm 0x23)
                )
            )

            (function $child (result i64)
                (code
                    // sleep 5000ms
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP} (i64.imm 5000))

                    // set thread_exit_code as 0x19
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

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let result0 = start_program_in_multithread(program_resource0, vec![]);
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

    let module_binary = helper_generate_module_image_binary_from_str(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (data $buf (read_write i32 0))
            (function $test (result i64)
                (local $tid i32)
                (local $last_length i32)
                (local $last_result i32)
                (local $last_status i32)
                (local $temp i32)   // for dropping operand
                (code
                    // create new thread
                    (local.store32 $tid
                        (envcall {ENV_CALL_CODE_THREAD_CREATE}
                            (macro.get_function_public_index $child)   // function pub index
                            (i32.imm 0)         // thread_start_data_address
                            (i32.imm 0)         // thread_start_data_length
                        )
                    )

                    // write the data to be sent
                    (data.store32 $buf
                        (i32.imm 0x11)      // data
                    )

                    // send data to child thread
                    (local.store32 $last_result
                        (envcall {ENV_CALL_CODE_THREAD_SEND_MSG}
                            (local.load32_i32 $tid)     // child thread id
                            (host.addr_data $buf)       // data src address
                            (i32.imm 4)                 // data length
                        )
                    )

                    // the last_result should be 0=success
                    (when
                        (local.load32_i32 $last_result)
                        (debug 0)
                    )

                    // receive data from child thread
                    (local.store32 $last_length
                        (local.store32 $last_result
                            (envcall {ENV_CALL_CODE_THREAD_RECEIVE_MSG}
                                (local.load32_i32 $tid)
                            )
                        )
                    )

                    // last_result should be 0=success
                    (when
                        (local.load32_i32 $last_result)
                        (debug 1)
                    )

                    // last_length should be 4
                    (when
                        (i32.ne
                            (local.load32_i32 $last_length)
                            (i32.imm 4)
                        )
                        (debug 2)
                    )

                    // test function 'thread_msg_length'
                    (local.store32 $last_length
                        (envcall {ENV_CALL_CODE_THREAD_MSG_LENGTH})
                    )

                    // last_length should be 4
                    (when
                        (i32.ne
                            (local.load32_i32 $last_length)
                            (i32.imm 4)
                        )
                        (debug 3)
                    )

                    // test function 'thread_msg_read'
                    // try to read 8 bytes, actually read 4 bytes
                    (local.store32 $last_length
                        (envcall {ENV_CALL_CODE_THREAD_MSG_READ}
                            (i32.imm 0)             // offset
                            (i32.imm 8)             // length
                            (host.addr_data $buf)   // dst address
                        )
                    )

                    // last_length should be 4
                    (when
                        (i32.ne
                            (local.load32_i32 $last_length)
                            (i32.imm 4)
                        )
                        (debug 4)
                    )

                    // check the received data
                    (when
                        (i32.ne
                            (data.load32_i32 $buf)
                            (i32.imm 0x13)
                        )
                        (debug 5)
                    )

                    // the status of child thread is changing to 'finish', wait 500ms
                    (envcall {ENV_CALL_CODE_THREAD_SLEEP} (i64.imm 500))

                    (local.store32 $last_status
                        (local.store32 $last_result
                            (envcall {ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                                (local.load32_i32 $tid)
                            )
                        )
                    )

                    // last_result should be 0=success
                    (when
                        (local.load32_i32 $last_result)
                        (debug 6)
                    )

                    // last_status should be 1=finish
                    (when
                        (i32.ne
                            (local.load32_i32 $last_status)
                            (i32.imm 1)
                        )
                        (debug 7)
                    )

                    // collect the child thread,
                    // and use the exit code of child thread as the
                    // main thread exit code
                    // (drop
                    (local.store32 $temp
                        (envcall {ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                            (local.load32_i32 $tid)
                        )
                    )
                )
            )

            (function $child (result i64)
                (local $last_length i32)
                (local $last_result i32)
                (code
                    // receive data from parent
                    (local.store32 $last_length
                        (envcall {ENV_CALL_CODE_THREAD_RECEIVE_MSG_FROM_PARENT})
                    )

                    // last_length should be 4
                    (when
                        (i32.ne
                            (local.load32_i32 $last_length)
                            (i32.imm 4)
                        )
                        (debug 0)
                    )

                    // test function 'thread_msg_length'
                    (local.store32 $last_length
                        (envcall {ENV_CALL_CODE_THREAD_MSG_LENGTH})
                    )

                    // last_length should be 4
                    (when
                        (i32.ne
                            (local.load32_i32 $last_length)
                            (i32.imm 4)
                        )
                        (debug 1)
                    )

                    // test function 'thread_msg_read'
                    // try to read 8 bytes, actually read 4 bytes
                    (local.store32 $last_length
                        (envcall {ENV_CALL_CODE_THREAD_MSG_READ}
                            (i32.imm 0)             // offset
                            (i32.imm 8)             // length
                            (host.addr_data $buf)   // dst address
                        )
                    )

                    // last_length should be 4
                    (when
                        (i32.ne
                            (local.load32_i32 $last_length)
                            (i32.imm 4)
                        )
                        (debug 2)
                    )

                    // check the received data
                    (when
                        (i32.ne
                            (data.load32_i32 $buf)
                            (i32.imm 0x11)
                        )
                        (debug 3)
                    )

                    // set the data to be sent
                    (data.store32 $buf
                        (i32.imm 0x13)      // data
                    )

                    // send data to parent
                    (local.store32 $last_result
                        (envcall {ENV_CALL_CODE_THREAD_SEND_MSG_TO_PARENT}
                            (host.addr_data $buf)
                            (i32.imm 4)
                        )
                    )

                    // last_result should be 0-success
                    (when
                        (local.load32_i32 $last_result)
                        (debug 4)
                    )

                    // exit code
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

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let result0 = start_program_in_multithread(program_resource0, vec![]);
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

    let module_binary = helper_generate_module_image_binary_from_str(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (data $buf (read_write i32 0))
            (function $test (result i64)
                (local $tid0 i32)
                (local $tid1 i32)
                (local $temp i32)   // for dropping operand
                (code
                    // create child thread 0 (t0)
                    (local.store32 $tid0
                        (envcall {ENV_CALL_CODE_THREAD_CREATE}
                            (macro.get_function_public_index $child0)   // function pub index
                            (i32.imm 0)         // thread_start_data_address
                            (i32.imm 0)         // thread_start_data_length
                        )
                    )

                    // create child thread 1 (t1)
                    (local.store32 $tid1
                        (envcall {ENV_CALL_CODE_THREAD_CREATE}
                            (macro.get_function_public_index $child1)   // function pub index
                            (i32.imm 0)         // thread_start_data_address
                            (i32.imm 0)         // thread_start_data_length
                        )
                    )

                    // receive message from t0
                    (envcall {ENV_CALL_CODE_THREAD_RECEIVE_MSG}
                        (local.load32_i32 $tid0)
                    )

                    // read message to $buf
                    (envcall {ENV_CALL_CODE_THREAD_MSG_READ}
                        (i32.imm 0)             // offset
                        (i32.imm 4)             // length
                        (host.addr_data $buf)   // dst address
                    )

                    // send message to t1
                    (envcall {ENV_CALL_CODE_THREAD_SEND_MSG}
                        (local.load32_i32 $tid1)    // child thread id
                        (host.addr_data $buf)       // data src address
                        (i32.imm 4)                 // data length
                    )

                    (when
                        (i64.ne
                            // collect t0
                            // (drop
                            (local.store32 $temp
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
                            // collect t1
                            // (drop
                            (local.store32 $temp
                                (envcall {ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                                    (local.load32_i32 $tid1)
                                )
                            )
                            (i64.imm 0x17)
                        )
                        (debug 0)
                    )

                    // exit code
                    (i64.imm 0x19)
                )
            )

            (function $child0 (result i64)
                (code
                    // set the data to be sent
                    (data.store32 $buf
                        (i32.imm 0x11)      // data
                    )

                    // send data to parent
                    (envcall {ENV_CALL_CODE_THREAD_SEND_MSG_TO_PARENT}
                        (host.addr_data $buf)
                        (i32.imm 4)
                    )

                    // exit code 0
                    (i64.imm 0x13)
                )
            )

            (function $child1 (result i64)
                (code
                    // receive data from parent
                    (envcall {ENV_CALL_CODE_THREAD_RECEIVE_MSG_FROM_PARENT})

                    (envcall {ENV_CALL_CODE_THREAD_MSG_READ}
                        (i32.imm 0)             // offset
                        (i32.imm 4)             // length
                        (host.addr_data $buf)   // dst address
                    )

                    // check the received data
                    (when
                        (i32.ne
                            (data.load32_i32 $buf)
                            (i32.imm 0x11)
                        )
                        (debug 0)
                    )

                    // exit code 0
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

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let result0 = start_program_in_multithread(program_resource0, vec![]);
    assert_eq!(result0.unwrap(), 0x19);
}
