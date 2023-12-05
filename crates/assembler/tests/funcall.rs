// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_assembler::utils::helper_generate_module_image_binary_from_str;
use ancvm_program::program_source::ProgramSource;
use ancvm_process::{
    in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
};
use ancvm_types::ForeignValue;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_function_call() {
    // fn $test (i32) -> (i32)
    //     (call $sum_square)
    // end
    //
    // fn $sum_square (n/1:i32) -> (i32)
    //     zero
    //     (local_load32 0 0)
    //     (block 3 3) (sum/0:i32, n/1:i32) -> (i32)
    //                                  ;; if n == 0
    //         (local_load32 0 1)
    //         i32_eqz
    //         (block_alt 4 4) () -> (i32)
    //             (local_load32 1 0)   ;; then sum
    //             (break 0)            ;; else
    //                                  ;; sum + n^2
    //             (local_load32 1 0)
    //             (local_load32 1 1)
    //             (call $square)
    //             i32_add
    //                                  ;; n - 1
    //             (local_load32 1 1)
    //             (i32_dec 1)
    //                                  ;; recur 1
    //             (recur 1)
    //         end
    //     end
    // end
    //
    // fn $square (i32) -> (i32)
    //     (local_load 32)
    //     (local_load 32)
    //     i32_mul
    // end

    // expect (5) -> 1 + 2^2 + 3^2 + 4^2 + 5^2 -> 1 + 4 + 9 + 16 + 25 -> 55

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (param $count i32) (result i32)
                (code
                    (call $sum_square (local.load32_i32 $count))
                )
            )

            (function $sum_square (param $count i32) (result i32)
                (code
                    zero                        ;; for arg 'sum'
                    (local.load32_i32 $count)   ;; for arg 'n'
                    (for (param $sum i32) (param $n i32) (result i32)
                        (do
                            (if (result i32)
                                (i32.eqz (local.load32_i32 $n))
                                (local.load32_i32 $sum)
                                (recur
                                    (i32.add
                                        (local.load32_i32 $sum)
                                        (call $square (local.load32_i32 $n))
                                    )
                                    (i32.dec 1
                                        (local.load32_i32 $n)
                                    )
                                )
                            )
                        )
                    )
                )
            )

            (function $square (param $n i32) (result i32)
                (code
                    (i32.mul
                        (local.load32_i32 $n)
                        (local.load32_i32 $n)
                    )
                )
            )
        )
        "#,
    );

    let program_source0 = InMemoryProgramSource::new(vec![module_binary]);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::U32(5)]);
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(55),]);
}

#[test]
fn test_assemble_function_call_dyncall() {
    // fn $test () -> (i32, i32, i32, i32, i32)
    //     (i32_imm 2)
    //     (dyncall)
    //     (i32_imm 4)
    //     (dyncall)
    //     (i32_imm 3)
    //     (dyncall)
    //     (i32_imm 1)
    //     (dyncall)
    //     (i32_imm 2)
    //     (dyncall)
    // end
    //
    // fn $eleven (;1;) () -> (i32)
    //     (i32_imm 11)
    // end
    //
    // fn $thirteen (;2;) () -> (i32)
    //     (i32_imm 13)
    // end
    //
    // fn $seventeen (;3;) () -> (i32)
    //     (i32_imm 17)
    // end
    //
    // fn $nineteen (;4;) () -> (i32)
    //     (i32_imm 19)
    // end

    // expect (13, 19, 17, 11, 13)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (results i32 i32 i32 i32 i32)
                (code
                    (dyncall (macro.get_function_public_index $thirteen))
                    (dyncall (macro.get_function_public_index $nineteen))
                    (dyncall (macro.get_function_public_index $seventeen))
                    (dyncall (macro.get_function_public_index $eleven))
                    (dyncall (macro.get_function_public_index $thirteen))
                )
            )

            (function $eleven (result i32)
                (code
                    (i32.imm 11)
                )
            )

            (function $thirteen (result i32)
                (code
                    (i32.imm 13)
                )
            )

            (function $seventeen (result i32)
                (code
                    (i32.imm 17)
                )
            )

            (function $nineteen (result i32)
                (code
                    (i32.imm 19)
                )
            )
        )
        "#,
    );

    let program_source0 = InMemoryProgramSource::new(vec![module_binary]);
    let program0 = program_source0.build_program().unwrap();
    let mut thread_context0 = program0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::U32(13),
            ForeignValue::U32(19),
            ForeignValue::U32(17),
            ForeignValue::U32(11),
            ForeignValue::U32(13),
        ]
    );
}
