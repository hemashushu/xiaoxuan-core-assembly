// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

mod utils;

use std::time::{Duration, Instant};

use ancvm_program::program_source::ProgramSource;
use ancvm_runtime::{
    in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
};
use ancvm_types::envcallcode::EnvCallCode;
use libc::{clock_gettime, timespec, CLOCK_MONOTONIC};

use crate::utils::assemble_single_module;

#[test]
fn test_assemble_envcall_time_now() {
    // () -> (i64)

    let module_binaries = assemble_single_module(&format!(
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

#[test]
fn test_assemble_envcall_time_sleep() {
    // () -> (i)

    let module_binaries = assemble_single_module(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $test
                (code
                    (envcall {ENV_CALL_CODE_TIME_SLEEP}
                        (i64.imm 1000)
                    )
                )
            )
        )
        "#,
        ENV_CALL_CODE_TIME_SLEEP = (EnvCallCode::time_sleep as u32)
    ));

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let before = Instant::now();
    let _ = process_function(&mut thread_context0, 0, 0, &[]);
    let after = Instant::now();

    let duration = after.duration_since(before);
    let ms = duration.as_millis() as u64;
    assert!(ms > 500);
}
