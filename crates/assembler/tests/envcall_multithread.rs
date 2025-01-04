// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::time::Instant;

use anc_assembler::utils::helper_make_single_module_app;
use anc_context::resource::Resource;
use anc_processor::{
    envcall_num::EnvCallNum, in_memory_resource::InMemoryResource,
    multithread_process::start_program,
};
use pretty_assertions::assert_eq;

#[test]
fn test_assemble_multithread_main_thread_id() {
    // the signature of 'thread start function' must be
    // () -> (i32)

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        fn test ()->i32
            envcall({ENV_CALL_CODE_THREAD_ID})
        "#,
        ENV_CALL_CODE_THREAD_ID = (EnvCallNum::thread_id as u32)
    ));

    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let result0 = start_program(&process_context0, "", vec![]);

    const MAIN_THREAD_ID: u32 = 0;
    assert_eq!(result0.unwrap(), MAIN_THREAD_ID);
}

#[test]
fn test_assemble_multithread_child_thread_id() {
    // the signature of 'thread start function' must be
    // () -> (i32)

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        fn test ()->i32
        [
            temp:i32  // for dropping operands
        ]
        {{
            // drops 'thread result', leaves 'child thread exit code'
            local_store_i32(temp
                // returns (child thread exit code, thread result)
                envcall({ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                    // returns the child thread id
                    envcall({ENV_CALL_CODE_THREAD_CREATE}
                        get_function(child)                 // get function public index
                        imm_i32(0)                          // thread_start_data_address
                        imm_i32(0)                          // thread_start_data_length
                    )
                )
            )
        }}

        fn child ()->i32
            envcall({ENV_CALL_CODE_THREAD_ID})
        "#,
        ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT = (EnvCallNum::thread_wait_and_collect as u32),
        ENV_CALL_CODE_THREAD_CREATE = (EnvCallNum::thread_create as u32),
        ENV_CALL_CODE_THREAD_ID = (EnvCallNum::thread_id as u32)
    ));

    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let result0 = start_program(&process_context0, "", vec![]);

    const FIRST_CHILD_THREAD_ID: u32 = 1;
    assert_eq!(result0.unwrap(), FIRST_CHILD_THREAD_ID);
}

#[test]
fn test_assemble_multithread_thread_create() {
    // the signature of 'thread start function' must be
    // () -> (i32)

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        fn test ()->i32
        [
            temp:i32  // for dropping operands
        ]
        {{
            // drops 'thread result', leaves 'child thread exit code'
            local_store_i32(temp
                // returns (child thread exit code, thread result)
                envcall({ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                    // returns the child thread id
                    envcall({ENV_CALL_CODE_THREAD_CREATE}
                        get_function(child)                 // get function public index
                        imm_i32(0)                          // thread_start_data_address
                        imm_i32(0)                          // thread_start_data_length
                    )
                )
            )
        }}

        fn child ()->i32
            imm_i64(0x13)  // set thread_exit_code as 0x13
        "#,
        ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT = (EnvCallNum::thread_wait_and_collect as u32),
        ENV_CALL_CODE_THREAD_CREATE = (EnvCallNum::thread_create as u32)
    ));

    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let result0 = start_program(&process_context0, "", vec![]);
    assert_eq!(result0.unwrap(), 0x13);
}

#[test]
fn test_assemble_multithread_thread_sleep() {
    // the signature of 'thread start function' must be
    // () -> (i32)

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        fn test ()->i32
        {{
            // `thread_sleep (milliseconds:u64)`
            envcall({ENV_CALL_CODE_THREAD_SLEEP}
                imm_i64(1000)
            )

            // set thread_exit_code
            imm_i64(0x13)
        }}
        "#,
        ENV_CALL_CODE_THREAD_SLEEP = (EnvCallNum::thread_sleep as u32)
    ));

    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();

    let instant_a = Instant::now();
    let result0 = start_program(&process_context0, "", vec![]);
    assert_eq!(result0.unwrap(), 0x13);

    let instant_b = Instant::now();
    let duration = instant_b.duration_since(instant_a);
    let ms = duration.as_millis() as u64;
    assert!(ms > 500);
}

#[test]
fn test_assemble_multithread_thread_local_data_and_memory() {
    // the signature of 'thread start function' must be
    // () -> (i32)

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        data dat0:i32=0

        fn test ()->i32
            [tid:i32]
        {{
            // resize heap to 1 page
            memory_resize(imm_i32(1))

            // write the first value to data
            data_store_i32(dat0,
                imm_i32(0x11))      // the value

            // write the first value to memory at address 0x100
            memory_store_i32(
                imm_i64(0x100)     // address
                imm_i32(0x13)      // the value
            )

            // create child thread
            local_store_i32(tid
                envcall({ENV_CALL_CODE_THREAD_CREATE}
                    get_function(child) // function pub index
                    imm_i32(0)          // thread_start_data_address
                    imm_i32(0)          // thread_start_data_length
                )
            )

            // pause for 500ms to make sure the child thread has started.
            envcall({ENV_CALL_CODE_THREAD_SLEEP}, imm_i64(500))

            // wait and collect the child thread
            // returns (child thread exit code, thread result)
            envcall({ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                local_load_i32_s(tid)
            )

            // the child thread modifies the data and memory, but
            // this does not affect the data and memory of the main thread
            // because all data and memory are "thread-local".

            // check the value of data,
            // it should remain unchanged.
            when ne_i32(
                    data_load_i32_s(dat0)
                    imm_i32(0x11)
                )
                panic(0)

            // check the value of memory,
            // it should remain unchanged.
            when ne_i32(
                    memory_load_i32_s(
                        imm_i64(0x100)
                    )
                    imm_i32(0x13)
                )
                panic(1)

            // set thread_exit_code
            imm_i64(0)
        }}

        fn child ()->i32
        {{
            // resize heap to 1 page
            memory_resize(imm_i32(1))

            // write value to data
            // the data in the main thread wouldn't changed because the data is thread-local,
            data_store_i32(dat0
                imm_i32(0x23))      // new value

            // write value to memory at address 0x100
            // the memory in the main thread wouldn't changed because the memory is thread-local,
            memory_store_i32(
                imm_i64(0x100)      // address
                imm_i32(0x29)       // new value
            )

            // set thread_exit_code
            imm_i64(0)
        }}
        "#,
        ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT = (EnvCallNum::thread_wait_and_collect as u32),
        ENV_CALL_CODE_THREAD_SLEEP = (EnvCallNum::thread_sleep as u32),
        ENV_CALL_CODE_THREAD_CREATE = (EnvCallNum::thread_create as u32)
    ));

    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let result0 = start_program(&process_context0, "", vec![]);
    assert_eq!(result0.unwrap(), 0);
}

#[test]
fn test_assemble_multithread_thread_start_data() {
    // the signature of 'thread start function' must be
    // () -> (i32)

    // the "start data" is:
    // [0x11_i8, 0x13, 0x17, 0x19, 0x23, 0x29, 0x31, 0x37]
    //
    //                       0   3  4   7   (offset in byte)
    //                      |=====||=====|
    //  read the first 4 bytes |      | read remaining 4 bytes
    //                         |      |
    //                         |      |
    //                         v      v
    //                         0   3
    //         local var temp |=====|
    //
    //         the local var value:
    //         1. 0x19171311
    //         2. 0x37312923

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        fn test ()->i32
            [temp:i32]
        {{
            // check the data length
            when ne_i32(
                    envcall({ENV_CALL_CODE_THREAD_START_DATA_LENGTH})
                    imm_i32(8)
                )
                panic(0)

            // read the first 4 bytes data to local variable "temp"
            when ne_i32(
                    envcall({ENV_CALL_CODE_THREAD_START_DATA_READ}
                        imm_i32(0)              // offset
                        imm_i32(4)              // length
                        host_addr_local(temp)   // dst address
                    )
                    imm_i32(4)                  // actual read size
                )
                panic(1)

            // check value
            when ne_i32(
                    local_load_i32_s(temp)
                    imm_i32(0x19171311)
                )
                panic(2)

            // read the remaining 4 bytes data to local variable "temp"
            when ne_i32(
                    envcall({ENV_CALL_CODE_THREAD_START_DATA_READ}
                        imm_i32(4)              // offset
                        imm_i32(8)              // length
                        host_addr_local(temp)   // dst address
                    )
                    imm_i32(4)                  // actual read size
                )
                panic(3)

            // check value
            when ne_i32(
                    local_load_i32_s(temp)
                    imm_i32(0x37312923)
                )
                panic(4)

            // set thread_exit_code
            imm_i64(0)
        }}
        "#,
        ENV_CALL_CODE_THREAD_START_DATA_LENGTH = (EnvCallNum::thread_start_data_length as u32),
        ENV_CALL_CODE_THREAD_START_DATA_READ = (EnvCallNum::thread_start_data_read as u32)
    ));

    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let result0 = start_program(
        &process_context0,
        "",
        vec![0x11, 0x13, 0x17, 0x19, 0x23, 0x29, 0x31, 0x37],
    );
    assert_eq!(result0.unwrap(), 0);
}

#[test]
fn test_assemble_multithread_thread_running_status() {
    // the signature of 'thread start function' must be
    // () -> (i32)

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        fn test ()->i32
        [
            tid:i32
            last_status:i32, last_result:i32
            temp:i32   // for dropping operands
        ]
        {{
            // create child thread
            local_store_i32(tid
                envcall({ENV_CALL_CODE_THREAD_CREATE}
                    get_function(child)     // function pub index
                    imm_i32(0)              // thread_start_data_address
                    imm_i32(0)              // thread_start_data_length
                )
            )

            // pause for 500ms to make sure the child thread has started
            envcall({ENV_CALL_CODE_THREAD_SLEEP}, imm_i64(500))

            // get the running status and result
            local_store_i32(last_status
                local_store_i32(last_result
                    // returns:
                    // - running_status: 0=running, 1=finish
                    // - thread_result: 0=success, 1=failure (thread_not_found)
                    envcall({ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                        local_load_i32_s(tid)
                    )
                )
            )

            // check the "last_result", it should be 0 (=success)
            when
                local_load_i32_s(last_result)
                panic(0)

            // check the "last_status", it should be 0 (=running)
            when
                local_load_i32_s(last_status)
                panic(1)

            // pause for 1500ms to make sure the child thread is finished.
            // the child thread will last 1000ms.
            envcall({ENV_CALL_CODE_THREAD_SLEEP}, imm_i64(1500))

            // get the running status and result
            local_store_i32(last_status
                local_store_i32(last_result
                    envcall({ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                        local_load_i32_s(tid)
                    )
                )
            )

            // check the "last_result", it should be 0 (=success)
            when
                local_load_i32_s(last_result)
                panic(2)

            // check the "last_status", it should be 1 (=finish)
            when ne_i32(
                    local_load_i32_s(last_status)
                    imm_i32(1)
                )
                panic(3)

            // try to get the thrad running status of a non-existent thread.
            local_store_i32(last_status
                local_store_i32(last_result
                    envcall({ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                        imm_i32(0x1113)
                    )
                )
            )

            // check the "last_result", it should be 1 (=failure)
            when
                ne_i32(
                    local_load_i32_s(last_result)
                    imm_i32(1)
                )
                panic(4)

            // wait and collect the child thread
            // returns (child thread exit code, thread result)
            // drops the "thread result" and leaves the "child thread exit code"
            local_store_i32(temp
                envcall({ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                    local_load_i32_s(tid)
                )
            )
        }}

        fn child ()->i32
        {{
            // sleep for 1000ms
            envcall({ENV_CALL_CODE_THREAD_SLEEP}, imm_i64(1000))

            // set thread_exit_code as 0x17
            imm_i64(0x17)
        }}
        "#,
        ENV_CALL_CODE_THREAD_SLEEP = (EnvCallNum::thread_sleep as u32),
        ENV_CALL_CODE_THREAD_RUNNING_STATUS = (EnvCallNum::thread_running_status as u32),
        ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT = (EnvCallNum::thread_wait_and_collect as u32),
        ENV_CALL_CODE_THREAD_CREATE = (EnvCallNum::thread_create as u32)
    ));

    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let result0 = start_program(&process_context0, "", vec![]);
    assert_eq!(result0.unwrap(), 0x17);
}

#[test]
fn test_assemble_multithread_thread_terminate() {
    // the signature of 'thread start function' must be
    // () -> (i32)

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        fn test ()->i32
            [
            tid:i32
            last_status:i32
            last_result:i32
            ]
        {{
            // create child thread
            local_store_i32(tid
                envcall({ENV_CALL_CODE_THREAD_CREATE}
                    get_function(child)     // function pub index
                    imm_i32(0)              // thread_start_data_address
                    imm_i32(0)              // thread_start_data_length
                )
            )

            // pause for 500ms to make sure the child thread has started
            envcall({ENV_CALL_CODE_THREAD_SLEEP}, imm_i64(500))

            // get the running status
            local_store_i32(last_status
                local_store_i32(last_result
                    envcall({ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                        local_load_i32_s(tid)
                    )
                )
            )

            // check the "last_result", it should be 0 (=success)
            when
                local_load_i32_s(last_result)
                panic(0)

            // check the "last_status", it should be 0 (=running)
            when
                local_load_i32_s(last_status)
                panic(1)

            // terminate the child thread.
            // the child thread will last 5000ms if not terminated.
            envcall({ENV_CALL_CODE_THREAD_TERMINATE}, local_load_i32_s(tid))

            // get the runnting status
            local_store_i32(last_status
                local_store_i32(last_result
                    envcall({ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                        local_load_i32_s(tid)
                    )
                )
            )

            // check the "last_result", it should be 1 (=failure, thread_not_found)
            when
                ne_i32(
                    local_load_i32_s(last_result)
                    imm_i32(1)
                )
                panic(2)

            // try to collect the child thread
            local_store_i32(last_result
                envcall({ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                    local_load_i32_s(tid)
                )
            )

            // check the "last_result", it should be 1 (=failure, thread_not_found)
            when
                ne_i32(
                    local_load_i32_s(last_result)
                    imm_i32(1)
                )
                panic(3)

            // set thread_exit_code as 0x23
            imm_i64(0x23)
        }}

        fn child ()->i32
        {{
            // sleep for 5000ms
            envcall({ENV_CALL_CODE_THREAD_SLEEP}, imm_i64(5000))

            // set thread_exit_code as 0x19
            imm_i64(0x19)
        }}
        "#,
        ENV_CALL_CODE_THREAD_SLEEP = (EnvCallNum::thread_sleep as u32),
        ENV_CALL_CODE_THREAD_TERMINATE = (EnvCallNum::thread_terminate as u32),
        ENV_CALL_CODE_THREAD_RUNNING_STATUS = (EnvCallNum::thread_running_status as u32),
        ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT = (EnvCallNum::thread_wait_and_collect as u32),
        ENV_CALL_CODE_THREAD_CREATE = (EnvCallNum::thread_create as u32)
    ));

    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let result0 = start_program(&process_context0, "", vec![]);
    assert_eq!(result0.unwrap(), 0x23);
}

#[test]
fn test_assemble_multithread_thread_message_send_and_receive() {
    // the signature of 'thread start function' must be
    // () -> (i32)
    //
    // main thread       child thread
    //          send
    // 0x11    ----->    0x11
    //          send
    // 0x13    <-----    0x13
    //
    //          exit
    // 0x17    <-----    0x17

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        data dat0:i32 = 0
        fn test ()->i32
        [
            tid:i32
            last_length:i32
            last_result:i32
            last_status:i32
            temp:i32   // for dropping operands
        ]
        {{
            // create new thread
            local_store_i32(tid
                envcall({ENV_CALL_CODE_THREAD_CREATE}
                    get_function(child)     // function pub index
                    imm_i32(0)              // thread_start_data_address
                    imm_i32(0)              // thread_start_data_length
                )
            )

            // write the value to be sent
            data_store_i32(dat0
                imm_i32(0x11)               // the value
            )

            // send data to child thread
            local_store_i32(last_result
                envcall({ENV_CALL_CODE_THREAD_SEND_MSG}
                    local_load_i32_s(tid)   // child thread id
                    host_addr_data(dat0)    // data src address
                    imm_i32(4)              // data length
                )
            )

            // the last_result should be 0=success
            when
                local_load_i32_s(last_result)
                panic(0)

            // receive data from child thread
            local_store_i32(last_length
                local_store_i32(last_result
                    envcall({ENV_CALL_CODE_THREAD_RECEIVE_MSG}
                        local_load_i32_s(tid)
                    )
                )
            )

            // the last_result should be 0=success
            when
                local_load_i32_s(last_result)
                panic(1)

            // the last_length should be 4
            when
                ne_i32(
                    local_load_i32_s(last_length)
                    imm_i32(4)
                )
                panic(2)

            // test function 'thread_msg_length'
            local_store_i32(last_length
                envcall({ENV_CALL_CODE_THREAD_MSG_LENGTH})
            )

            // the last_length should be 4
            when
                ne_i32(
                    local_load_i32_s(last_length)
                    imm_i32(4)
                )
                panic(3)

            // test function 'thread_msg_read'
            // try to read 8 bytes, actually read 4 bytes
            local_store_i32(last_length
                envcall({ENV_CALL_CODE_THREAD_MSG_READ}
                    imm_i32(0)             // offset
                    imm_i32(8)             // length
                    host_addr_data(dat0)   // dst address
                )
            )

            // the last_length should be 4
            when
                ne_i32(
                    local_load_i32_s(last_length)
                    imm_i32(4)
                )
                panic(4)

            // check the received data
            when
                ne_i32(
                    data_load_i32_s(dat0)
                    imm_i32(0x13)
                )
                panic(5)

            // wait for 1500ms to make sure the child thread is finished.
            envcall({ENV_CALL_CODE_THREAD_SLEEP}, imm_i64(1500))

            local_store_i32(last_status
                local_store_i32(last_result
                    envcall({ENV_CALL_CODE_THREAD_RUNNING_STATUS}
                        local_load_i32_s(tid)
                    )
                )
            )

            // the last_result should be 0=success
            when
                local_load_i32_s(last_result)
                panic(6)

            // the last_status should be 1=finish
            when
                ne_i32(
                    local_load_i32_s(last_status)
                    imm_i32(1)
                )
                panic(7)

            // collect the child thread
            // returns (child thread exit code, thread result)
            // drops the "thread result" and leaves the "child thread exit code"
            local_store_i32(temp
                envcall({ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                    local_load_i32_s(tid)
                )
            )
        }}

        fn child ()->i32
        [
            last_length:i32
            last_result:i32
        ]
        {{
            // receive data from parent
            local_store_i32(last_length
                envcall({ENV_CALL_CODE_THREAD_RECEIVE_MSG_FROM_PARENT})
            )

            // the last_length should be 4
            when
                ne_i32(
                    local_load_i32_s(last_length)
                    imm_i32(4)
                )
                panic(0)

            // test function 'thread_msg_length'
            local_store_i32(last_length
                envcall({ENV_CALL_CODE_THREAD_MSG_LENGTH})
            )

            // the last_length should be 4
            when
                ne_i32(
                    local_load_i32_s(last_length)
                    imm_i32(4)
                )
                panic(1)

            // test function 'thread_msg_read'
            // try to read 8 bytes, actually read 4 bytes
            local_store_i32(last_length
                envcall({ENV_CALL_CODE_THREAD_MSG_READ}
                    imm_i32(0)             // offset
                    imm_i32(8)             // length
                    host_addr_data(dat0)   // dst address
                )
            )

            // the last_length should be 4
            when
                ne_i32(
                    local_load_i32_s(last_length)
                    imm_i32(4)
                )
                panic(2)

            // check the received data
            when
                ne_i32(
                    data_load_i32_s(dat0)
                    imm_i32(0x11)
                )
                panic(3)

            // set the value to be sent
            data_store_i32(dat0
                imm_i32(0x13)      // the value
            )

            // send data to parent
            local_store_i32(last_result
                envcall({ENV_CALL_CODE_THREAD_SEND_MSG_TO_PARENT}
                    host_addr_data(dat0)
                    imm_i32(4)
                )
            )

            // the last_result should be 0-success
            when
                local_load_i32_s(last_result)
                panic(4)

            // set thread_exit_code
            imm_i64(0x17)
        }}
        "#,
        ENV_CALL_CODE_THREAD_CREATE = (EnvCallNum::thread_create as u32),
        ENV_CALL_CODE_THREAD_SEND_MSG = (EnvCallNum::thread_send_msg as u32),
        ENV_CALL_CODE_THREAD_RECEIVE_MSG = (EnvCallNum::thread_receive_msg as u32),
        ENV_CALL_CODE_THREAD_MSG_LENGTH = (EnvCallNum::thread_msg_length as u32),
        ENV_CALL_CODE_THREAD_MSG_READ = (EnvCallNum::thread_msg_read as u32),
        ENV_CALL_CODE_THREAD_SEND_MSG_TO_PARENT = (EnvCallNum::thread_send_msg_to_parent as u32),
        ENV_CALL_CODE_THREAD_RECEIVE_MSG_FROM_PARENT =
            (EnvCallNum::thread_receive_msg_from_parent as u32),
        ENV_CALL_CODE_THREAD_RUNNING_STATUS = (EnvCallNum::thread_running_status as u32),
        ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT = (EnvCallNum::thread_wait_and_collect as u32),
        ENV_CALL_CODE_THREAD_SLEEP = (EnvCallNum::thread_sleep as u32),
    ));

    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let result0 = start_program(&process_context0, "", vec![]);
    assert_eq!(result0.unwrap(), 0x17);
}

#[test]
fn test_assemble_multithread_thread_message_forward() {
    // the signature of 'thread start function' must be
    // () -> (i32)

    // main thread      child thread 0      child thread 1
    //
    // 0x11   <------------0x11
    //   |
    //   \----------------------------------> 0x11
    //
    // check thread 0 exit code
    // check thread 1 exit code
    //
    // 0x19
    //   |
    //   \-- exit code

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        data dat0:i32 = 0
        fn test ()->i32
        [
            tid0:i32
            tid1:i32
            temp:i32   // for dropping operands
        ]
        {{
            // create child thread 0 (t0)
            local_store_i32(tid0
                envcall({ENV_CALL_CODE_THREAD_CREATE}
                    get_function(child0)    // function pub index
                    imm_i32(0)              // thread_start_data_address
                    imm_i32(0)              // thread_start_data_length
                )
            )

            // create child thread 1 (t1)
            local_store_i32(tid1
                envcall({ENV_CALL_CODE_THREAD_CREATE}
                    get_function(child1)    // function pub index
                    imm_i32(0)              // thread_start_data_address
                    imm_i32(0)              // thread_start_data_length
                )
            )

            // receive message from thread 0
            envcall({ENV_CALL_CODE_THREAD_RECEIVE_MSG}
                local_load_i32_s(tid0)
            )

            // read message to dat0
            envcall({ENV_CALL_CODE_THREAD_MSG_READ}
                imm_i32(0)                  // offset
                imm_i32(4)                  // length
                host_addr_data(dat0)        // dst address
            )

            // send message to thread 1
            envcall({ENV_CALL_CODE_THREAD_SEND_MSG}
                local_load_i32_s(tid1)      // child thread id
                host_addr_data(dat0)        // data src address
                imm_i32(4)                  // data length
            )

            // check thread 0 exit code
            when
                ne_i64(
                    // wait and collect the child thread 0
                    // returns (child thread exit code, thread result)
                    // drops the "thread result" and leaves the "child thread exit code"
                    local_store_i32(temp
                        envcall({ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                            local_load_i32_s(tid0)
                        )
                    )
                    imm_i64(0x13)
                )
                panic(0)


            when
                ne_i64(
                    // wait and collect the child thread 1
                    // returns (child thread exit code, thread result)
                    // drops the "thread result" and leaves the "child thread exit code"
                    local_store_i32(temp
                        envcall({ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT}
                            local_load_i32_s(tid1)
                        )
                    )
                    imm_i64(0x17)
                )
                panic(1)

            // set thread_exit_code
            imm_i64(0x19)
        }}

        fn child0 ()->i32
        {{
            // set the value to be sent
            data_store_i32(dat0
                imm_i32(0x11)      // the value
            )

            // send data to parent
            envcall({ENV_CALL_CODE_THREAD_SEND_MSG_TO_PARENT}
                host_addr_data(dat0)
                imm_i32(4)
            )

            // set thread_exit_code 0
            imm_i64(0x13)
        }}

        fn child1 ()->i32
        {{
            // receive data from parent
            envcall({ENV_CALL_CODE_THREAD_RECEIVE_MSG_FROM_PARENT})

            // read data
            envcall({ENV_CALL_CODE_THREAD_MSG_READ}
                imm_i32(0)             // offset
                imm_i32(4)             // length
                host_addr_data(dat0)   // dst address
            )

            // check the received data
            when
                ne_i32(
                    data_load_i32_s(dat0)
                    imm_i32(0x11)
                )
                panic(0)

            // set thread_exit_code 0
            imm_i64(0x17)
        }}
        "#,
        ENV_CALL_CODE_THREAD_CREATE = (EnvCallNum::thread_create as u32),
        ENV_CALL_CODE_THREAD_SEND_MSG = (EnvCallNum::thread_send_msg as u32),
        ENV_CALL_CODE_THREAD_RECEIVE_MSG = (EnvCallNum::thread_receive_msg as u32),
        ENV_CALL_CODE_THREAD_MSG_READ = (EnvCallNum::thread_msg_read as u32),
        ENV_CALL_CODE_THREAD_SEND_MSG_TO_PARENT = (EnvCallNum::thread_send_msg_to_parent as u32),
        ENV_CALL_CODE_THREAD_RECEIVE_MSG_FROM_PARENT =
            (EnvCallNum::thread_receive_msg_from_parent as u32),
        ENV_CALL_CODE_THREAD_WAIT_AND_COLLECT = (EnvCallNum::thread_wait_and_collect as u32),
    ));

    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let result0 = start_program(&process_context0, "", vec![]);
    assert_eq!(result0.unwrap(), 0x19);
}
