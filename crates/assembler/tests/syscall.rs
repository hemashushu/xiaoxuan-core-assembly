// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_assembler::utils::helper_generate_module_image_binaries_from_single_module_assembly;
use ancvm_binary::bytecode_reader::print_bytecode_as_text;
use ancvm_program::program_source::ProgramSource;
use ancvm_process::{
    in_memory_program_source::InMemoryProgramSource, interpreter::process_function,
};
use ancvm_syscall_util::{errno::Errno, number::SysCallNum};
use ancvm_types::ForeignValue;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_syscall_without_args() {
    // fn $test () -> (result:i64 errno:i32)

    // syscall:
    // `pid_t getpid(void);`

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test (results i64 i32)
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

    let function_entry = program0.module_images[0]
        .get_function_section()
        .get_function_entry(0);

    let bytecode_text = print_bytecode_as_text(&function_entry.code);
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

    assert!(matches!(result_values0[0], ForeignValue::U64(value) if value == pid as u64));
    assert_eq!(result_values0[1], ForeignValue::U32(0));
}

#[test]
fn test_assemble_syscall_with_2_args() {
    // fn $test (buf_addr:i64, buf_len:i32) -> (result:i64 errno:i32)

    // syscall:
    // `char *getcwd(char buf[.size], size_t size);`

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test
                (param $buf_addr i64)
                (param $buf_len i32)
                (results i64 i32)
                (code
                    (syscall
                        {SYS_CALL_NUMBER_0}
                        (local.load64_i64 $buf_addr)
                        (local.load32_i32 $buf_len)
                    )
                )
            )
        )
        "#,
        SYS_CALL_NUMBER_0 = (SysCallNum::getcwd as u32)
    ));

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();

    let function_entry = program0.module_images[0]
        .get_function_section()
        .get_function_entry(0);

    let bytecode_text = print_bytecode_as_text(&function_entry.code);
    // println!("{}", bytecode_text);

    assert_eq!(
        bytecode_text,
        "\
0x0000  00 02 00 00  00 00 00 00    local.load64_i64  rev:0   off:0x00  idx:0
0x0008  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
0x0010  80 01 00 00  4f 00 00 00    i32.imm           0x0000004f
0x0018  80 01 00 00  02 00 00 00    i32.imm           0x00000002
0x0020  03 0b                       syscall
0x0022  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    const BUF_LENGTH: u32 = 1024;
    let buf = [0u8; BUF_LENGTH as usize];
    let buf_addr = buf.as_ptr() as u64;

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::U64(buf_addr),
            ForeignValue::U32(BUF_LENGTH),
        ],
    );

    let results0 = result0.unwrap();

    // note
    //
    // the syscall 'getcwd' in the libc returns the pointer to the buf, but the
    // raw syscall 'getcwd' returns the length of the path (includes the NULL terminated char)

    let null_pos = buf.iter().position(|u| *u == 0).unwrap();

    assert!(matches!(results0[0], ForeignValue::U64(value) if value == (null_pos + 1) as u64));
    assert_eq!(results0[1], ForeignValue::U32(0));

    let path0 = String::from_utf8_lossy(&buf[0..null_pos]);
    let cwd = std::env::current_dir().unwrap();
    let cwd0 = cwd.as_os_str().to_string_lossy();
    assert_eq!(path0, cwd0);
}

#[test]
fn test_assemble_syscall_error_no() {
    // fn $test (file_path_buf_addr:i64) -> (result:i64 errno:i32)

    // syscall:
    // `int open(const char *pathname, int flags)`

    let module_binaries = helper_generate_module_image_binaries_from_single_module_assembly(&format!(
        r#"
        (module $app
            (runtime_version "1.0")
            (function $test
                (param $file_path_buf_addr i64)
                (results i64 i32)
                (code
                    (syscall
                        {SYS_CALL_NUMBER_0}
                        (local.load64_i64 $file_path_buf_addr)
                        (i32.imm 0) ;; open flags
                    )
                )
            )
        )
        "#,
        SYS_CALL_NUMBER_0 = (SysCallNum::open as u32)
    ));

    let program_source0 = InMemoryProgramSource::new(module_binaries);
    let program0 = program_source0.build_program().unwrap();

    let function_entry = program0.module_images[0]
        .get_function_section()
        .get_function_entry(0);

    let bytecode_text = print_bytecode_as_text(&function_entry.code);
    // println!("{}", bytecode_text);

    assert_eq!(
        bytecode_text,
        "\
0x0000  00 02 00 00  00 00 00 00    local.load64_i64  rev:0   off:0x00  idx:0
0x0008  80 01 00 00  00 00 00 00    i32.imm           0x00000000
0x0010  80 01 00 00  02 00 00 00    i32.imm           0x00000002
0x0018  80 01 00 00  02 00 00 00    i32.imm           0x00000002
0x0020  03 0b                       syscall
0x0022  00 0a                       end"
    );

    let mut thread_context0 = program0.create_thread_context();

    let file_path0 = b"/this/file/should/not/exist\0";
    let file_path1 = b"/dev/zero\0";

    let file_path_addr0 = file_path0.as_ptr() as usize;
    let file_path_addr1 = file_path1.as_ptr() as usize;

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U64(file_path_addr0 as u64)],
    );
    let results0 = result0.unwrap();

    assert_eq!(
        results0,
        vec![
            ForeignValue::U64(0),
            ForeignValue::U32(Errno::ENOENT as u32)
        ]
    );

    let result1 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U64(file_path_addr1 as u64)],
    );
    let results1 = result1.unwrap();

    assert!(matches!(results1[0], ForeignValue::U64(value) if value > 0));
    assert_eq!(results1[1], ForeignValue::U32(0));
}
