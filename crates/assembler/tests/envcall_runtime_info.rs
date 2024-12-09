// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use anc_assembler::utils::helper_make_single_module_app;
use anc_context::resource::Resource;
use anc_isa::{
    ForeignValue, RUNTIME_CODE_NAME, RUNTIME_MAJOR_VERSION, RUNTIME_MINOR_VERSION,
    RUNTIME_PATCH_VERSION,
};
use anc_processor::{
    envcall_num::EnvCallNum, handler::Handler, in_memory_resource::InMemoryResource,
    process::process_function,
};
use pretty_assertions::assert_eq;

#[test]
fn test_assemble_envcall_runtime_version() {
    // () -> (i64)

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        fn test ()->i64
            envcall({ENV_CALL_CODE_RUNTIME_VERSION})
        "#,
        ENV_CALL_CODE_RUNTIME_VERSION = (EnvCallNum::runtime_version as u32)
    ));

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);

    let expect_version_number = RUNTIME_PATCH_VERSION as u64
        | (RUNTIME_MINOR_VERSION as u64) << 16
        | (RUNTIME_MAJOR_VERSION as u64) << 32;

    assert_eq!(
        result0.unwrap(),
        vec![ForeignValue::U64(expect_version_number)]
    );
}

#[test]
fn test_assemble_envcall_runtime_code_name() {
    // () -> (i32, i64)
    //        ^    ^
    //        |    |name buffer (8 bytes)
    //        |name length

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        fn test () -> (i32, i64)
            [buf:byte[8, align=8]]
        {{
            envcall({ENV_CALL_CODE_RUNTIME_NAME}, host_addr_local(buf))
            local_load_i64(buf)
        }}
        "#,
        ENV_CALL_CODE_RUNTIME_NAME = (EnvCallNum::runtime_name as u32)
    ));

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    let fvs1 = result0.unwrap();
    let name_len = fvs1[0].as_u32();
    let name_u64 = fvs1[1].as_u64();

    let name_data = name_u64.to_le_bytes();
    assert_eq!(&RUNTIME_CODE_NAME[..], &name_data[0..name_len as usize]);
}
