// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use anc_assembler::utils::helper_make_single_module_app;
use anc_context::process_resource::ProcessResource;
use anc_isa::{ForeignValue, RUNTIME_EDITION};
use anc_processor::{
    envcall_num::EnvCallNum, handler::Handler, in_memory_process_resource::InMemoryProcessResource,
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
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);

    let version_patch = env!("CARGO_PKG_VERSION_PATCH").parse::<u16>().unwrap();
    let version_minor = env!("CARGO_PKG_VERSION_MINOR").parse::<u16>().unwrap();
    let version_major = env!("CARGO_PKG_VERSION_MAJOR").parse::<u16>().unwrap();

    let expect_version_number =
        version_patch as u64 | (version_minor as u64) << 16 | (version_major as u64) << 32;

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
            envcall({ENV_CALL_CODE_RUNTIME_EDITION}, host_addr_local(buf))
            local_load_i64(buf)
        }}
        "#,
        ENV_CALL_CODE_RUNTIME_EDITION = (EnvCallNum::runtime_edition as u32)
    ));

    let handler = Handler::new();
    let resource0 = InMemoryProcessResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    let fvs1 = result0.unwrap();
    let name_len = fvs1[0].as_u32();
    let name_u64 = fvs1[1].as_u64();

    let name_data = name_u64.to_le_bytes();
    assert_eq!(RUNTIME_EDITION, &name_data);
    assert_eq!(
        RUNTIME_EDITION.iter().position(|c| *c == 0).unwrap(),
        name_len as usize
    );
}
