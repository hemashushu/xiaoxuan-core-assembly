// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// mod utils;

// use ancvm_program::program_source::ProgramSource;
// use ancvm_runtime::{
//     in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
// };
// use ancvm_types::ForeignValue;
//
// use crate::utils::assemble_single_module;
//
// use pretty_assertions::assert_eq;

// #[test]
// fn test_assemble_control_flow_when() {
//
//     let module_binaries = assemble_single_module(
//         r#"
//         (module $app
//             (runtime_version "1.0")
//             (func $main
//                 (param $a0 f32)
//                 (param $a1 f32)
//                 (param $a2 f32)
//                 (param $a3 f32)
//                 (results
//                     i32 i32 i32
//                     i32 i32 i32
//                     i32 i32 i32 i32
//                     i32 i32 i32 i32)
//                 (code
//                     (i32.and (local.load32_i32 $a0) (local.load32_i32 $a1))
//                     (i32.or (local.load32_i32 $a0) (local.load32_i32 $a1))
//                     (i32.xor (local.load32_i32 $a0) (local.load32_i32 $a1))
//
//                     (i32.shift_left (local.load32_i32 $a2) (i32.imm 4))
//                     (i32.shift_right_s (local.load32_i32 $a3) (i32.imm 16))
//                     (i32.shift_right_u (local.load32_i32 $a3) (i32.imm 16))
//
//                     (i32.shift_left (local.load32_i32 $a2) (i32.imm 24))
//                     (i32.rotate_left (local.load32_i32 $a2) (i32.imm 24))
//                     (i32.shift_right_u (local.load32_i32 $a2) (i32.imm 28))
//                     (i32.rotate_right (local.load32_i32 $a2) (i32.imm 28))
//
//                     (i32.not (local.load32_i32 $a0))
//                     (i32.leading_zeros (local.load32_i32 $a2))
//                     (i32.trailing_zeros (local.load32_i32 $a2))
//                     (i32.count_ones (local.load32_i32 $a2))
//                 )
//             )
//         )
//         "#,
//     );
//
//     let program_source0 = InMemoryProgramSource::new(module_binaries);
//     let program0 = program_source0.build_program().unwrap();
//     let mut thread_context0 = program0.create_thread_context();
//
//     let result0 = process_function(
//         &mut thread_context0,
//         0,
//         0,
//         &[
//             ForeignValue::UInt32(0xff0000ff),
//             ForeignValue::UInt32(0xf0f000ff),
//             ForeignValue::UInt32(0x00f00000),
//             ForeignValue::UInt32(0x80000000),
//         ],
//     );
//     assert_eq!(
//         result0.unwrap(),
//         vec![
//             // group 0
//             ForeignValue::UInt32(0xf00000ff),
//             ForeignValue::UInt32(0xfff000ff),
//             ForeignValue::UInt32(0x0ff00000),
//             // group 1
//             ForeignValue::UInt32(0x0f000000),
//             ForeignValue::UInt32(0xffff8000),
//             ForeignValue::UInt32(0x00008000),
//             // group 2
//             ForeignValue::UInt32(0x00000000),
//             ForeignValue::UInt32(0x0000f000),
//             ForeignValue::UInt32(0x00000000),
//             ForeignValue::UInt32(0x0f000000),
//             // group 3
//             ForeignValue::UInt32(0x00ffff00),
//             ForeignValue::UInt32(8),
//             ForeignValue::UInt32(20),
//             ForeignValue::UInt32(4),
//         ]
//     );
// }
