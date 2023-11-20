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
use ancvm_types::{
    envcallcode::EnvCallCode, ForeignValue, RUNTIME_CODE_NAME, RUNTIME_MAJOR_VERSION,
    RUNTIME_MINOR_VERSION, RUNTIME_PATCH_VERSION,
};

use crate::utils::assemble_single_module;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_envcall_runtime_version() {
    // () -> (i64)

    let module_binaries = assemble_single_module(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $main (result i64)
                (code
                    (envcall {ENV_CALL_CODE_0})
                )
            )
        )
        "#,
        ENV_CALL_CODE_0 = (EnvCallCode::runtime_version as u32)
    ));

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();

    let func_entry = program0.module_images[0]
        .get_func_section()
        .get_func_entry(0);

    let bytecode_text = print_bytecode_as_text(&func_entry.code);
    assert_eq!(
        bytecode_text,
        "\
0x0000  02 0b 00 00  01 01 00 00    envcall           idx:257
0x0008  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);

    let expect_version_number = RUNTIME_PATCH_VERSION as u64
        | (RUNTIME_MINOR_VERSION as u64) << 16
        | (RUNTIME_MAJOR_VERSION as u64) << 32;

    assert_eq!(
        result0.unwrap(),
        vec![ForeignValue::UInt64(expect_version_number)]
    );
}

#[test]
fn test_assemble_envcall_runtime_code_name() {
    // () -> (i32, i64)
    //        ^    ^
    //        |    |name buffer (8 bytes)
    //        |name length

    let module_binaries = assemble_single_module(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $main (results i32 i64)
                (local $buf (bytes 8 8))
                (code
                    (envcall {ENV_CALL_CODE_0} (host.addr_local $buf))
                    (local.load64_i64 $buf)
                )
            )
        )
        "#,
        ENV_CALL_CODE_0 = (EnvCallCode::runtime_name as u32)
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
0x0000  03 0c 00 00  00 00 00 00    host.addr_local   rev:0   off:0x00  idx:0
0x0008  02 0b 00 00  00 01 00 00    envcall           idx:256
0x0010  00 02 00 00  00 00 00 00    local.load64_i64  rev:0   off:0x00  idx:0
0x0018  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    let fvs1 = result0.unwrap();
    let name_len = if let ForeignValue::UInt32(i) = fvs1[0] {
        i
    } else {
        0
    };
    let name_u64 = if let ForeignValue::UInt64(i) = fvs1[1] {
        i
    } else {
        0
    };

    let name_data = name_u64.to_le_bytes();
    assert_eq!(&RUNTIME_CODE_NAME[..], &name_data[0..name_len as usize]);
}
