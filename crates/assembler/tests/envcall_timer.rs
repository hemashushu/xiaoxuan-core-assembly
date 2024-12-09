// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::time::Duration;

use anc_assembler::utils::helper_make_single_module_app;
use anc_context::resource::Resource;
use anc_processor::{
    envcall_num::EnvCallNum, handler::Handler, in_memory_resource::InMemoryResource,
    process::process_function,
};
use libc::{clock_gettime, timespec, CLOCK_MONOTONIC};

#[test]
fn test_assemble_envcall_time_now() {
    // () -> (i64, i32)

    let binary0 = helper_make_single_module_app(&format!(
        r#"
        fn test ()->(i64, i32)
            envcall({ENV_CALL_CODE_TIME_NOW})
        "#,
        ENV_CALL_CODE_TIME_NOW = (EnvCallNum::time_now as u32)
    ));

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    let results0 = result0.unwrap();

    let secs = results0[0].as_u64();
    let nanos = results0[1].as_u32();
    let dur_a = Duration::new(secs, nanos);

    let mut t: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe {
        clock_gettime(CLOCK_MONOTONIC, &mut t);
    }
    let dur_b = Duration::new(t.tv_sec as u64, t.tv_nsec as u32);

    let dur = dur_b - dur_a;
    assert!(dur.as_millis() < 1000);
}
