// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::time::Duration;

use ancvm_assembler::utils::helper_generate_module_image_binaries_from_single_module_assembly;
use ancvm_process::{
    in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
};
use ancvm_program::program_source::ProgramSource;
use ancvm_types::envcallcode::EnvCallCode;
use libc::{clock_gettime, timespec, CLOCK_MONOTONIC};

#[test]
fn test_assemble_envcall_time_now() {
    // () -> (i64)

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test (results i64 i32)
                (code
                    (envcall {ENV_CALL_CODE_TIME_NOW})
                )
            )
        )
        "#,
        ENV_CALL_CODE_TIME_NOW = (EnvCallCode::time_now as u32)
    ));

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    let results0 = result0.unwrap();

    let secs = results0[0].as_u64();
    let nanos = results0[1].as_u32();
    let dur_before = Duration::new(secs, nanos);

    let mut t: timespec = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe {
        clock_gettime(CLOCK_MONOTONIC, &mut t);
    }
    let dur_after = Duration::new(t.tv_sec as u64, t.tv_nsec as u32);

    let dur = dur_after - dur_before;
    assert!(dur.as_millis() < 1000);
}
