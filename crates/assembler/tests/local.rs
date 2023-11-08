// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

mod utils;

use ancvm_program::program_source::ProgramSource;
use ancvm_runtime::{in_memory_program_source::InMemoryProgramSource, interpreter::process_function};
use ancvm_types::ForeignValue;

use crate::utils::assemble_single_module;

#[test]
fn test_assemble_local_load_store() {
    // (f32, f64) -> (i64,i32,i32,i32,i32,i32, f32,f64 ,i64,i32)
    let module_binaries = assemble_single_module(
        r#"
        (module $app
            (runtime_version "1.0")
            (fn $main
                (param $v0 f32) (param $v1 f64)
                (results
                        i64 i32 i32 i32 i32 i32 ;; group 0
                        f32 f64                 ;; group 1
                        i64 i32                 ;; group 2
                        )
                (local $v2 (bytes 8 8))
                (code
                    zero
                )
            )
        )
        "#,
    );

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    // assert_eq!(result0.unwrap(), vec![ForeignValue::UInt32(0)]);
}