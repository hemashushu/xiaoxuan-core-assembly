// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

mod utils;

use std::env;

use ancvm_extfunc_util::cstr_pointer_to_str;
use ancvm_program::{program_settings::ProgramSettings, program_source::ProgramSource};
use ancvm_runtime::{
    in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
};
use ancvm_types::ForeignValue;

use crate::utils::assemble_single_module;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_extcall_with_system_libc_getuid() {
    // () -> (i32)

    // `man 3 getuid`
    // 'uid_t getuid(void);'

    let module_binaries = assemble_single_module(
        r#"
        (module $app
            (runtime_version "1.0")
            (extern (library system "libc.so.6")
                (fn $getuid "getuid" (result i32))
            )
            (fn $main (result i32)
                (code
                    (extcall $getuid)
                )
            )
        )
        "#,
    );

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    let results0 = result0.unwrap();

    assert!(matches!(results0[0], ForeignValue::UInt32(uid) if uid > 0 ));
}

#[test]
fn test_assemble_extcall_with_system_libc_getenv() {
    // () -> (i64)

    // `man 3 getenv`
    // 'char *getenv(const char *name);'

    let module_binaries = assemble_single_module(
        r#"
        (module $app
            (runtime_version "1.0")
            (extern (library system "libc.so.6")
                (fn $getenv "getenv" (param i64) (result i64))
            )
            (data $pwd (read_only cstring "PWD"))
            (fn $main (result i64)
                (code
                    (extcall $getenv
                        (host.addr_data $pwd)
                    )
                )
            )
        )
        "#,
    );

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    let results0 = result0.unwrap();

    assert!(matches!(results0[0], ForeignValue::UInt64(addr) if {
        let pwd0 = cstr_pointer_to_str(addr as *const i8);
        !pwd0.to_string().is_empty()
    }));
}

#[test]
fn test_assemble_extcall_with_user_lib() {
    // (i32,i32) -> (i32)

    // 'lib-test-0.so.1'
    // 'int add(int, int)'

    let module_binaries = assemble_single_module(
        r#"
        (module $app
            (runtime_version "1.0")
            (extern (library user "lib-test-0.so.1")
                (fn $add "add" (params i32 i32) (result i32))
            )
            (fn $main (param $a i32) (param $b i32) (result i32)
                (code
                    (extcall $add
                        (local.load32_i32 $a)
                        (local.load32_i32 $b)
                    )
                )
            )
        )
        "#,
    );

    let mut pwd = env::current_dir().unwrap();
    pwd.push("tests");
    let program_source_path = pwd.to_str().unwrap();

    let program_source0 = InMemoryProgramSource::with_settings(
        module_binaries,
        &ProgramSettings::new(program_source_path, true, "", ""),
    );

    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::UInt32(11), ForeignValue::UInt32(13)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(24)]);

    let result1 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::UInt32(211), ForeignValue::UInt32(223)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::UInt32(434)]);
}
