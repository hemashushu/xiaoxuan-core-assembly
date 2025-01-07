// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::collections::HashMap;

use anc_assembler::utils::{
    helper_make_single_module_app, helper_make_single_module_app_with_external_library,
};
use anc_context::{process_config::ProcessConfig, process_resource::ProcessResource};
use anc_image::entry::ExternalLibraryEntry;
use anc_isa::{DependencyCondition, DependencyLocal, ExternalLibraryDependency, ForeignValue};
use anc_processor::{
    handler::Handler, in_memory_process_resource::InMemoryProcessResource,
    process::process_function, HandleErrorType, HandlerError,
};
use pretty_assertions::assert_eq;

fn read_memory_i64(fv: ForeignValue) -> u64 {
    // #[cfg(target_pointer_width = "64")]
    if let ForeignValue::U64(addr) = fv {
        let ptr = addr as *const u64;
        unsafe { std::ptr::read(ptr) }
    } else {
        panic!("The data type of the foreign value does not match.")
    }
    // #[cfg(target_pointer_width = "32")]
    // if let ForeignValue::U32(addr) = fv {
    //     let ptr = addr as *const u64;
    //     unsafe { std::ptr::read(ptr) }
    // } else {
    //     panic!("The data type of the foreign value does not match.")
    // }
}

fn read_memory_i32(fv: ForeignValue) -> u32 {
    // #[cfg(target_pointer_width = "64")]
    if let ForeignValue::U64(addr) = fv {
        let ptr = addr as *const u32;
        unsafe { std::ptr::read(ptr) }
    } else {
        panic!("The data type of the foreign value does not match.")
    }
    // #[cfg(target_pointer_width = "32")]
    // if let ForeignValue::U32(addr) = fv {
    //     let ptr = addr as *const u32;
    //     unsafe { std::ptr::read(ptr) }
    // } else {
    //     panic!("The data type of the foreign value does not match.")
    // }
}

fn read_memory_i16(fv: ForeignValue) -> u16 {
    // #[cfg(target_pointer_width = "64")]
    if let ForeignValue::U64(addr) = fv {
        let ptr = addr as *const u16;
        unsafe { std::ptr::read(ptr) }
    } else {
        panic!("The data type of the foreign value does not match.")
    }
    // #[cfg(target_pointer_width = "32")]
    // if let ForeignValue::U32(addr) = fv {
    //     let ptr = addr as *const u16;
    //     unsafe { std::ptr::read(ptr) }
    // } else {
    //     panic!("The data type of the foreign value does not match.")
    // }
}

fn read_memory_i8(fv: ForeignValue) -> u8 {
    // #[cfg(target_pointer_width = "64")]
    if let ForeignValue::U64(addr) = fv {
        let ptr = addr as *const u8;
        unsafe { std::ptr::read(ptr) }
    } else {
        panic!("The data type of the foreign value does not match.")
    }
    // #[cfg(target_pointer_width = "32")]
    // if let ForeignValue::U32(addr) = fv {
    //     let ptr = addr as *const u8;
    //     unsafe { std::ptr::read(ptr) }
    // } else {
    //     panic!("The data type of the foreign value does not match.")
    // }
}

#[test]
fn test_assemble_host_panic() {
    // () -> ()
    let binary0 = helper_make_single_module_app(
        r#"
        fn test()
            panic(0x101)
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();

    let mut thread_context0 = process_context0.create_thread_context();
    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);

    assert!(matches!(
        result0,
        Err(HandlerError {
            error_type: HandleErrorType::Panic(0x101)
        })
    ));
}

/*
#[test]
fn test_assemble_host_debug() {
    // () -> ()

    let binary0 = helper_make_single_module_app(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test
                (code
                    nop
                    (debug 0x101)
                )
            )
        )
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();

    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);

    assert!(matches!(
        result0,
        Err(InterpreterError {
            error_type: InterpreterErrorType::Debug(0x101)
        })
    ));
}

#[test]
fn test_assemble_host_unreachable() {
    // () -> ()

    let binary0 = helper_make_single_module_app(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test
                (code
                    nop
                    (unreachable 0x103)
                )
            )
        )
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();

    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);

    assert!(matches!(
        result0,
        Err(InterpreterError {
            error_type: InterpreterErrorType::Unreachable(0x103)
        })
    ));
}
*/

#[test]
fn test_assemble_host_address_of_data_and_local_variables() {
    //        read-only data section
    //        ======================
    //
    //       |low address    high addr|
    //       |                        |
    // index |0              1        |
    //  type |i32------|    |i32------|
    //
    //  data 11 00 00 00    13 00 00 00
    //
    //        read write data section
    //        =======================
    //
    //       |low address             high address|
    //       |                                    |
    // index |2(0)                       3(1)     |
    //  type |i64------------------|    |i32------|
    //
    //  data 17 00 00 00 00 00 00 00    19 00 00 00
    //
    //        uninitialized data section
    //        ==========================
    //
    //       |low address             high address|
    //       |                                    |
    // index |4(0)           5(1)                 |
    //  type |i32------|    |i64------------------|
    //
    //  data 23 00 00 00    29 00 00 00 00 00 00 00
    //
    //        local variable area
    //        ===================
    //
    //       |low address                                       high addr|
    //       |                                                           |
    // index |0       1                           2                      |
    //  type |bytes| |i32------|   |padding--|   |i32------|   |padding--|
    //
    //  data 0.....0 31 00 00 00   00 00 00 00   37 00 00 00   00 00 00 00
    //       ^
    //       | 64 bytes, the space for storing function results.
    //       | because the results will overwrite the stack, so it need to
    //       | leave enough space for results, then the data of local variables
    //       | can be still read after function is finish.
    //
    // () -> (i64,i64,i64,i64,i64,i64, i64,i64)
    //        -----------------------  -------
    //        | addr of data           | addr of local variables
    //
    // read the values of data and local variables through the host address.

    let binary0 = helper_make_single_module_app(
        r#"
        readonly data d0:i32 = 0x11
        readonly data d1:i32 = 0x13
        data d2:i64 = 0xee    // init data
        data d3:i32 = 0xff    // init data
        uninit data d4:i32
        uninit data d5:i64

        fn test () ->
            (i64, i64, i64, i64, i64, i64, i64, i64)
            [reserved:byte[64,align=8], n1:i32, n2:i32]
        {
            // store values to data
            data_store_i64(d2
                imm_i64(0x17))
            data_store_i32(d3
                imm_i32(0x19))
            data_store_i32(d4
                imm_i32(0x23))
            data_store_i64(d5
                imm_i64(0x29))

            // store values to local vars
            local_store_i32(n1
                imm_i32(0x31))
            local_store_i32(n2
                imm_i32(0x37))

            // get host address of data
            host_addr_data(d0, offset=0)
            host_addr_data(d1, offset=0)
            host_addr_data(d2)
            host_addr_data(d3)
            host_addr_data(d4)
            host_addr_data(d5)

            // get host address of local vars
            host_addr_local(n1, offset=0)
            host_addr_local(n2)
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();

    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    let fvs = result0.unwrap();

    assert_eq!(read_memory_i32(fvs[0]), 0x11);
    assert_eq!(read_memory_i32(fvs[1]), 0x13);
    assert_eq!(read_memory_i64(fvs[2]), 0x17);
    assert_eq!(read_memory_i32(fvs[3]), 0x19);
    assert_eq!(read_memory_i32(fvs[4]), 0x23);
    assert_eq!(read_memory_i64(fvs[5]), 0x29);

    // note:
    // depending on the implementation of the stack (the stack frame and local variables),
    // the following 'assert_eq' may fail,
    // because the local variables (as well as their host addresses) will no longer valid
    // when a function exits.

    assert_eq!(read_memory_i32(fvs[6]), 0x31);
    assert_eq!(read_memory_i32(fvs[7]), 0x37);
}

#[test]
fn test_assemble_host_address_of_data_and_local_variables_extend() {
    //        read-only data section
    //        ======================
    //
    //       |low address  high addr|
    //       |                      |
    // index |0            1        |
    //  type |bytes----|  |byte-----|
    //
    //  data 02 03 05 07  11 13 17 19
    //       |     |            |  |
    //       |0    |1           |2 |3
    //
    //        local variable area
    //        ===================
    //
    //       |low address         high addr|
    //       |                             |
    // index |0       1                    |
    //  type |bytes| |bytes----------------|
    //
    //  data 0.....0 23 29 31 37 41 43 47 53
    //       ^       |        |        |  |
    //       |       |4       |5       |6 |7
    //       |
    //       | 64 bytes, the space for storing function results.
    //       | because the results will overwrite the stack, so it need to
    //       | leave enough space for results, then the data of local variables
    //       | can be still read after function is finish.
    //
    // () -> (i64,i64,i64,i64, i64,i64, i64,i64)
    //        ---------------- ----------------
    //        | addr of data   | addr of local variables
    //
    // read the values of data and local variables through the host address.

    let binary0 = helper_make_single_module_app(
        r#"
        readonly data d0:byte[align=8] = h"02 03 05 07"
        readonly data d1:byte[align=8] = h"11 13 17 19"

        fn test () ->
            (i64, i64, i64, i64, i64, i64, i64, i64)
            [reserved:byte[64,align=8], n1:byte[8,align=8]]
        {
            // store values to local vars
            local_store_i64(n1
                imm_i64(0x5347434137312923_i64))

            // get host address of data
            host_addr_data_extend(d0, imm_i32(0))
            host_addr_data_extend(d0, imm_i32(2))
            host_addr_data_extend(d1, imm_i32(2))
            host_addr_data_extend(d1, imm_i32(3))

            // get host address of local vars
            host_addr_local_extend(n1, imm_i32(0))
            host_addr_local_extend(n1, imm_i32(3))
            host_addr_local_extend(n1, imm_i32(6))
            host_addr_local_extend(n1, imm_i32(7))
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();

    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    let fvs = result0.unwrap();

    assert_eq!(read_memory_i8(fvs[0]), 0x02);
    assert_eq!(read_memory_i8(fvs[1]), 0x05);
    assert_eq!(read_memory_i8(fvs[2]), 0x17);
    assert_eq!(read_memory_i8(fvs[3]), 0x19);

    // note:
    // depending on the implementation of the stack (the stack frame and local variables),
    // the following 'assert_eq' may fail,
    // because the local variables (as well as their host addresses) will no longer valid
    // when a function exits.

    assert_eq!(read_memory_i8(fvs[4]), 0x23);
    assert_eq!(read_memory_i8(fvs[5]), 0x37);
    assert_eq!(read_memory_i8(fvs[6]), 0x47);
    assert_eq!(read_memory_i8(fvs[7]), 0x53);
}

#[test]
fn test_assemble_host_address_memory() {
    //        heap
    //       |low address                high addr|
    //       |                                    |
    //  addr |0x100         0x200                 |
    //  type |i32-------|   |i64------------------|
    //
    //  data  02 03 05 07   11 13 17 19 23 29 31 37
    //        ^     ^       ^           ^        ^
    //        |0    |1      |2          |3       |4
    //
    // () -> (i64,i64,i64,i64,i64)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test() ->
            (i64, i64, i64, i64, i64)
        {
            // init the heap size
            memory_resize(imm_i32(1))

            // store values to heap
            memory_store_i32(
                imm_i64(0x100)
                imm_i32(0x07050302)
            )

            memory_store_i64(
                imm_i64(0x200)
                imm_i64(0x37312923_19171311_i64)
            )

            // get host address of heap
            host_addr_memory(imm_i64(0x100), offset=0)
            host_addr_memory(imm_i64(0x100), offset=2)

            host_addr_memory(imm_i64(0x200), offset=0)
            host_addr_memory(imm_i64(0x200), offset=4)
            host_addr_memory(imm_i64(0x200), offset=7)
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();

    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    let fvs = result0.unwrap();

    assert_eq!(read_memory_i32(fvs[0]), 0x07050302);
    assert_eq!(read_memory_i16(fvs[1]), 0x0705);
    assert_eq!(read_memory_i64(fvs[2]), 0x3731292319171311);
    assert_eq!(read_memory_i32(fvs[3]), 0x37312923);
    assert_eq!(read_memory_i8(fvs[4]), 0x37);
}

#[test]
fn test_assemble_host_memory_and_vm_memory_copy() {
    // fn(src_ptr, dst_ptr) -> ()

    // copy src_ptr -> VM heap 0x100 with 8 bytes
    // copy VM heap 0x100 -> dst_ptr with 8 bytes
    //
    //               0x100                        dst_ptr
    //            vm |01234567| --> copy --> host |01234567|
    //                ^
    //       /--copy--/
    //       |
    // host |01234567|
    //      src_ptr

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(src_ptr:i64,dst_ptr:i64)
        {
            // init the heap size
            memory_resize(imm_i32(1))

            // host_copy_from_memory(dst_pointer:i64, src_addr:i64, count:i64) -> ()
            // host_copy_to_memory(dst_addr:i64, src_pointer:i64, count:i64) -> ()

            host_copy_to_memory(
                imm_i64(0x100)
                local_load_i64(src_ptr)
                imm_i64(8)
            )

            host_copy_from_memory(
                local_load_i64(dst_ptr)
                imm_i64(0x100)
                imm_i64(8)
            )
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let src_buf: &[u8; 8] = b"hello.vm";
    let dst_buf: [u8; 8] = [0; 8];

    let src_ptr = src_buf.as_ptr();
    let dst_ptr = dst_buf.as_ptr();

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::U64(src_ptr as usize as u64),
            ForeignValue::U64(dst_ptr as usize as u64),
        ],
    );
    result0.unwrap();

    assert_eq!(&dst_buf, b"hello.vm");
}

#[test]
fn test_assemble_host_external_memory_copy() {
    // fn(src_ptr, dst_ptr) -> ()

    // copy src_ptr -> dst_ptr
    //
    // host src_ptr  local var     host dst_ptr
    // |01234567| -> |45670123| -> |45670123|

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(src_ptr:i64, dst_ptr:i64)
        {
            // host_external_memory_copy(dst_pointer:i64, src_pointer:i64, count:i64) -> ()
            host_external_memory_copy(
                local_load_i64(dst_ptr)
                local_load_i64(src_ptr)
                imm_i64(8)
            )
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let src_buf: &[u8; 8] = b"whatever";
    let dst_buf: [u8; 8] = [0; 8];

    let src_ptr = src_buf.as_ptr();
    let dst_ptr = dst_buf.as_ptr();

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::U64(src_ptr as usize as u64),
            ForeignValue::U64(dst_ptr as usize as u64),
        ],
    );
    result0.unwrap();

    assert_eq!(&dst_buf, b"whatever");
}

#[test]
fn test_assemble_host_addr_function_and_callback_function() {
    // the external function (a C function) in "libtest0.so.1":
    //
    // int do_something(int (*callback_function)(int), int a, int b)
    // {
    //     int s = (callback_function)(a);
    //     return s + b;
    // }
    //
    // VM functions
    //
    // ;; entry function
    // fn function0 (a:i32, b:i32)->i32 {
    //     do_something(function1, a, b)
    // }
    //
    // ;; used as callback function for external function 'do_something'
    // fn function1 (a:i32) -> i32 {
    //     a*2
    // }
    //
    // calling path:
    // (11,13) ->
    //   function0 (VM) ->
    //     do_something (external function) ->
    //       function1 (call from external) ->
    //     return to do_something ->
    //   return to function0 ->
    // return (11*2+13)

    let libtest0 = ExternalLibraryEntry::new(
        "test0".to_owned(),
        Box::new(ExternalLibraryDependency::Local(Box::new(
            DependencyLocal {
                path: "lib/libtest0.so.1".to_owned(), // it should be a path of file "*.so.VERSION" relative to the application
                condition: DependencyCondition::True,
                parameters: HashMap::default(),
            },
        ))),
    );

    let binary0 = helper_make_single_module_app_with_external_library(
        r#"
        external fn test0::do_something(i64,i32,i32)->i32

        fn func0(a:i32, b:i32)->i32
        {
            extcall(do_something,
                host_addr_function(func1)
                local_load_i32_s(a)
                local_load_i32_s(b)
            )
        }

        fn func1(n:i32)->i32
        {
            mul_i32(
                local_load_i32_s(n)
                imm_i32(2)
            )
        }
        "#,
        &[libtest0],
    );

    let mut pwd = std::env::current_dir().unwrap();
    // let pkg_name = env!("CARGO_PKG_NAME");
    let crate_folder_name = "assembler";
    if !pwd.ends_with(crate_folder_name) {
        // in the VSCode `Debug` environment, the `current_dir()`
        // the project root folder.
        // while in both `$ cargo test` and VSCode `Run Test` environment
        // the `current_dir()` return the current crate path.
        pwd.push("crates");
        pwd.push(crate_folder_name);
    }
    pwd.push("tests");
    let application_path = pwd.to_str().unwrap();

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::with_config(
        vec![binary0],
        &ProcessConfig::new(
            application_path,
            false,
            vec![],
            HashMap::<String, String>::new(),
        ),
    );
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(11), ForeignValue::U32(13)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(11 * 2 + 13)]);

    let result1 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(211), ForeignValue::U32(223)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::U32(211 * 2 + 223)]);
}
