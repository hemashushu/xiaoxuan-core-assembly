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
    process::process_function,
};
use dyncall_util::cstr_pointer_to_str;
use pretty_assertions::assert_eq;
use syscall_util::{errno::Errno, number::SysCallNum};

#[test]
fn test_assemble_function_call() {
    // fn test (num/0:i32) -> (i32)             ;; type 0
    //     call(sum_square)
    // end
    //
    // fn sum_square (count/0:i32) -> (i32)     ;; type 1
    //     imm_i32(0)
    //     local_load32(0, 0)
    //     block (sum/0:i32, n/1:i32) -> (i32)  ;; type 3
    //                                          ;; if n == 0
    //         local_load32(0, 1)
    //         eqz_i32
    //         block_alt () -> (i32)            ;; type 4
    //             local_load32(1, 0)           ;; then sum
    //         break_alt()                      ;; else
    //                                          ;; sum + n^2
    //             local_load32(1, 0)
    //             local_load32(1, 1)
    //             call(square)
    //             add_i32
    //                                          ;; n - 1
    //             local_load32(1, 1)
    //             sub_imm_i32(1)
    //                                          ;; recur 1
    //             recur(1)
    //         end
    //     end
    // end
    //
    // fn square (num/0:i32) -> (i32)         // type 2
    //     local_load_i32s(0, 0)
    //     local_load_i32s(0, 0)
    //     mul_i32()
    // end
    //
    // expect (5) -> 1 + 2^2 + 3^2 + 4^2 + 5^2 -> 1 + 4 + 9 + 16 + 25 -> 55

    let binary0 = helper_make_single_module_app(
        r#"
        fn test (count:i32) -> i32
        {
            call(sum_square, local_load_i32_s(count))
        }

        fn sum_square (count:i32) -> i32
        {
            block (sum:i32=imm_i32(0), n:i32=local_load_i32_s(count)) -> i32
            {
                if -> i32
                    eqz_i32(local_load_i32_s(n))
                    local_load_i32_s(sum)
                    recur(
                        add_i32(
                            local_load_i32_s(sum)
                            call(square, local_load_i32_s(n))
                        )
                        sub_imm_i32(
                            1
                            local_load_i32_s(n)
                        )
                    )
            }
        }

        fn square (n:i32) -> i32
        {
            mul_i32(
                local_load_i32_s(n)
                local_load_i32_s(n)
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
        &[ForeignValue::U32(5)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(55),]);
}

#[test]
fn test_assemble_function_dyncall() {
    // fn test () -> (i32, i32, i32, i32, i32)  ;; pub idx: 0
    //     get_function(thirteen)
    //     dyncall()
    //     get_function(nineteen)
    //     dyncall()
    //     get_function(seventeen)
    //     dyncall()
    //     get_function(eleven)
    //     dyncall()
    //     get_function(thirteen)
    //     dyncall()
    // end
    //
    // fn eleven () -> (i32)        ;; pub idx: 1
    //     imm_i32(11)
    // end
    //
    // fn thirteen () -> (i32)      ;; pub idx: 2
    //     imm_i32(13)
    // end
    //
    // fn seventeen () -> (i32)     ;; pub idx: 3
    //     imm_i32(17)
    // end
    //
    // fn nineteen () -> (i32)      ;; pub idx: 4
    //     imm_i32(19)
    // end
    //
    // expect (13, 19, 17, 11, 13)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test () -> (i32, i32, i32, i32, i32)
        {
            dyncall(get_function(thirteen))
            dyncall(get_function(nineteen))
            dyncall(get_function(seventeen))
            dyncall(get_function(eleven))
            dyncall(get_function(thirteen))
        }

        fn eleven () -> i32
            imm_i32(11)

        fn thirteen () -> i32
            imm_i32(13)

        fn seventeen () -> i32
            imm_i32(17)

        fn nineteen () -> i32
            imm_i32(19)
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
            ForeignValue::U32(13),
            ForeignValue::U32(19),
            ForeignValue::U32(17),
            ForeignValue::U32(11),
            ForeignValue::U32(13),
        ]
    );
}

#[test]
fn test_assemble_syscall_without_args() {
    // fn test () -> (result:i64 errno:i32)

    // syscall:
    // `pid_t getpid(void);`

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        fn test ()->(i64, i32)
            syscall ({SYS_CALL_NUMBER_0})
        "#,
        SYS_CALL_NUMBER_0 = (SysCallNum::getpid as u32)
    ));

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    let result_values0 = result0.unwrap();

    let pid = std::process::id();

    assert!(matches!(result_values0[0], ForeignValue::U64(value) if value == pid as u64));
    assert_eq!(result_values0[1], ForeignValue::U32(0));
}

#[test]
fn test_assemble_syscall_with_2_args() {
    // fn test (buf_addr:i64, buf_len:i32) -> (result:i64 errno:i32)
    //
    // syscall:
    // `char *getcwd(char buf[.size], size_t size);`

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        fn test(buf_addr:i64, buf_len:i32) -> (i64, i32)
            syscall(
                {SYS_CALL_NUMBER_0}
                local_load_i64(buf_addr)
                local_load_i32_s(buf_len)
            )
        "#,
        SYS_CALL_NUMBER_0 = (SysCallNum::getcwd as u32)
    ));

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    const BUF_LENGTH: u32 = 1024;
    let buf = [0u8; BUF_LENGTH as usize];
    let buf_addr = buf.as_ptr() as u64;

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U64(buf_addr), ForeignValue::U32(BUF_LENGTH)],
    );

    let results0 = result0.unwrap();

    // note
    //
    // the function 'getcwd' in the libc returns the pointer to the buf, but the
    // raw syscall 'getcwd' returns the length of the path (includes the NULL terminated char)

    let null_pos = buf.iter().position(|u| *u == 0).unwrap();

    assert!(matches!(results0[0], ForeignValue::U64(value) if value == (null_pos + 1) as u64));
    assert_eq!(results0[1], ForeignValue::U32(0));

    let path0 = String::from_utf8_lossy(&buf[0..null_pos]);
    let cwd = std::env::current_dir().unwrap();
    let cwd0 = cwd.as_os_str().to_string_lossy();
    assert_eq!(path0, cwd0);
}

#[test]
fn test_assemble_syscall_error_no() {
    // fn test (file_path_buf_addr:i64) -> (result:i64 errno:i32)
    //
    // syscall:
    // `int open(const char *pathname, int flags)`

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        fn test(file_path_buf_addr:i64) -> (i64, i32)
            syscall(
                {SYS_CALL_NUMBER_0}
                local_load_i64(file_path_buf_addr)
                imm_i32(0)         // open flags
            )
        "#,
        SYS_CALL_NUMBER_0 = (SysCallNum::open as u32)
    ));

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let file_path0 = b"/this/file/should/not/exist\0";
    let file_path1 = b"/dev/zero\0";

    let file_path_addr0 = file_path0.as_ptr() as usize;
    let file_path_addr1 = file_path1.as_ptr() as usize;

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U64(file_path_addr0 as u64)],
    );
    let results0 = result0.unwrap();

    assert_eq!(
        results0,
        vec![
            ForeignValue::U64(0),
            ForeignValue::U32(Errno::ENOENT as u32)
        ]
    );

    let result1 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U64(file_path_addr1 as u64)],
    );
    let results1 = result1.unwrap();

    assert!(matches!(results1[0], ForeignValue::U64(value) if value > 0));
    assert_eq!(results1[1], ForeignValue::U32(0));
}

#[test]
fn test_assemble_extcall_with_system_libc_getuid() {
    // () -> (i32)
    //
    // ref:
    // `man 3 getuid`
    // 'uid_t getuid(void);'

    let libc = ExternalLibraryEntry::new(
        "libc".to_owned(),
        Box::new(ExternalLibraryDependency::System("libc.so.6".to_owned())),
    );

    let binary0 = helper_make_single_module_app_with_external_library(
        r#"
        external fn libc::getuid()-> i32

        fn test ()-> i32
            extcall(getuid)
        "#,
        &[libc],
    );

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);

    assert!(result0.is_ok());

    // let results0 = result0.unwrap();
    // assert!(matches!(results0[0], ForeignValue::U32(uid) if uid > 0 ));
}

#[test]
fn test_assemble_extcall_with_system_libc_getenv() {
    // () -> (i64)
    //
    // ref:
    // `man 3 getenv`
    // 'char *getenv(const char *name);'

    let libc = ExternalLibraryEntry::new(
        "libc".to_owned(),
        Box::new(ExternalLibraryDependency::System("libc.so.6".to_owned())),
    );

    let binary0 = helper_make_single_module_app_with_external_library(
        r#"
        external fn libc::getenv (i64) -> i64

        readonly data pwd:byte[] = "PWD"

        fn test ()-> i64
            extcall(
                getenv
                host_addr_data(pwd))
        "#,
        &[libc],
    );

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    let results0 = result0.unwrap();

    assert!(matches!(results0[0], ForeignValue::U64(addr) if {
        let pwd0 = cstr_pointer_to_str(addr as *const i8);
        !pwd0.to_string().is_empty()
    }));
}

#[test]
fn test_assemble_extcall_with_user_lib() {
    // (i32,i32) -> (i32)
    //
    // 'libtest0.so.1'
    // 'int add(int, int)'

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
        external fn test0::add (i32, i32) -> i32

        fn test (a:i32, b:i32) -> i32
        extcall(
            add
            local_load_i32_s(a)
            local_load_i32_s(b)
        )
        "#,
        &[libtest0],
    );

    let mut pwd = std::env::current_dir().unwrap();
    // let pkg_name = env!("CARGO_PKG_NAME");
    let crate_folder_name = "assembler";
    if !pwd.ends_with(crate_folder_name) {
        // in the VSCode editor `Debug` environment, the `current_dir()` returns
        // the project's root folder.
        // while in both `$ cargo test` and VSCode editor `Run Test` environment,
        // the `current_dir()` returns the current crate path.
        // here canonicalize the unit test resources path.
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
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(24)]);

    let result1 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(211), ForeignValue::U32(223)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::U32(434)]);
}
