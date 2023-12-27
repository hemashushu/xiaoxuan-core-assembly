// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancasm_assembler::utils::helper_generate_module_image_binary_from_str;
use ancvm_extfunc_util::cstr_pointer_to_str;
use ancvm_process::{
    in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
};
use ancvm_program::{program_settings::ProgramSettings, program_source::ProgramSource};
use ancvm_types::ForeignValue;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_extcall_with_system_libc_getuid() {
    // () -> (i32)

    // `man 3 getuid`
    // 'uid_t getuid(void);'

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (runtime_version "1.0")
            (external (library system "libc.so.6")
                (function $getuid "getuid" (result i32))
            )
            (function $test (result i32)
                (code
                    (extcall $getuid)
                )
            )
        )
        "#,
    );

    let program_source0 = InMemoryProgramSource::new(vec![module_binary]);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    let results0 = result0.unwrap();

    assert!(matches!(results0[0], ForeignValue::U32(uid) if uid > 0 ));
}

#[test]
fn test_assemble_extcall_with_system_libc_getenv() {
    // () -> (i64)

    // `man 3 getenv`
    // 'char *getenv(const char *name);'

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (runtime_version "1.0")
            (external (library system "libc.so.6")
                (function $getenv "getenv" (param i64) (result i64))
            )
            (data $pwd (read_only cstring "PWD"))
            (function $test (result i64)
                (code
                    (extcall $getenv
                        (host.addr_data $pwd)
                    )
                )
            )
        )
        "#,
    );

    let program_source0 = InMemoryProgramSource::new(vec![module_binary]);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    let results0 = result0.unwrap();

    assert!(matches!(results0[0], ForeignValue::U64(addr) if {
        let pwd0 = cstr_pointer_to_str(addr as *const i8);
        !pwd0.to_string().is_empty()
    }));
}

#[test]
fn test_assemble_extcall_with_user_lib() {
    // (i32,i32) -> (i32)

    // 'libtest0.so.1'
    // 'int add(int, int)'

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (runtime_version "1.0")
            (external (library user "libtest0.so.1")
                (function $add "add" (params i32 i32) (result i32))
            )
            (function $test (param $a i32) (param $b i32) (result i32)
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

    let mut pwd = std::env::current_dir().unwrap();
    if !pwd.ends_with("assembler") {
        // in the VSCode editor `Debug` environment, the `current_dir()` returns
        // the project's root folder.
        // while in both `$ cargo test` and VSCode editor `Run Test` environment,
        // the `current_dir()` returns the current crate path.
        // here canonicalize the test resources path.
        pwd.push("crates");
        pwd.push("assembler");
    }
    pwd.push("tests");

    let program_source_path = pwd.to_str().unwrap();

    let program_source0 = InMemoryProgramSource::with_settings(
        vec![module_binary],
        &ProgramSettings::new(program_source_path, true, "", ""),
    );

    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(11), ForeignValue::U32(13)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(24)]);

    let result1 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(211), ForeignValue::U32(223)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::U32(434)]);
}
