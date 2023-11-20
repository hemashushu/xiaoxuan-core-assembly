// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

mod utils;

use ancvm_binary::bytecode_reader::print_bytecode_as_text;
use ancvm_program::program_source::ProgramSource;
use ancvm_runtime::{
    in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
};
use ancvm_syscall_util::{errno::Errno, number::SysCallNum};
use ancvm_types::{
    ForeignValue, RUNTIME_MAJOR_VERSION, RUNTIME_MINOR_VERSION, RUNTIME_PATCH_VERSION,
};

use crate::utils::assemble_single_module;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_syscall_without_args() {
    // fn $main () -> (result:i64 errno:i32)

    // syscall:
    // `pid_t getpid(void);`

    let module_binaries = assemble_single_module(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $main (results i64 i32)
                (code
                    (syscall {SYS_CALL_NUMBER_0})
                )
            )
        )
        "#,
        SYS_CALL_NUMBER_0 = (SysCallNum::getpid as u32)
    ));

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();

    let func_entry = program0.module_images[0]
        .get_func_section()
        .get_func_entry(0);

    let bytecode_text = print_bytecode_as_text(&func_entry.code);
    // println!("{}", bytecode_text);

    assert_eq!(
        bytecode_text,
        "\
0x0000  80 01 00 00  27 00 00 00    i32.imm           0x00000027
0x0008  80 01 00 00  00 00 00 00    i32.imm           0x00000000
0x0010  03 0b                       syscall
0x0012  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    let result_values0 = result0.unwrap();

    let pid = std::process::id();

    assert!(matches!(result_values0[0], ForeignValue::UInt64(value) if value == pid as u64));
    assert_eq!(result_values0[1], ForeignValue::UInt32(0));
}
