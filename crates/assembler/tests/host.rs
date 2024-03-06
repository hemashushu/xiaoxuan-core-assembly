// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancasm_assembler::utils::helper_generate_module_image_binary_from_str;
use ancvm_binary::bytecode_reader::format_bytecode_as_text;
use ancvm_context::{program_resource::ProgramResource, program_settings::ProgramSettings};
use ancvm_processor::{
    in_memory_program_resource::InMemoryProgramResource, interpreter::process_function,
    InterpreterError, InterpreterErrorType,
};

use ancvm_types::ForeignValue;

use pretty_assertions::assert_eq;

#[test]
fn test_assemble_host_panic() {
    // () -> ()

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test
                (code
                    nop
                    panic
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();

    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);

    assert!(matches!(
        result0,
        Err(InterpreterError {
            error_type: InterpreterErrorType::Panic
        })
    ));
}

#[test]
fn test_assemble_host_debug() {
    // () -> ()

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test
                (code
                    nop
                    (debug 0x101)
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();

    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);

    assert!(matches!(
        result0,
        Err(InterpreterError {
            error_type: InterpreterErrorType::Debug(0x101)
        })
    ));
}

#[test]
fn test_assemble_host_unreachable() {
    // () -> ()

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test
                (code
                    nop
                    (unreachable 0x103)
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();

    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);

    assert!(matches!(
        result0,
        Err(InterpreterError {
            error_type: InterpreterErrorType::Unreachable(0x103)
        })
    ));
}

#[test]
fn test_assemble_host_address_of_data_and_local_vars() {
    //        read-only data section
    //        ======================
    //
    //       |low address    high addr|
    //       |                        |
    // index |0              1        |
    //  type |i32------|    |i32------|
    //
    //  data 11 00 00 00    13 00 00 00
    //
    //        read write data section
    //        =======================
    //
    //       |low address             high address|
    //       |                                    |
    // index |2(0)                       3(1)     |
    //  type |i64------------------|    |i32------|
    //
    //  data 17 00 00 00 00 00 00 00    19 00 00 00
    //
    //        uninitialized data section
    //        ==========================
    //
    //       |low address             high address|
    //       |                                    |
    // index |4(0)           5(1)                 |
    //  type |i32------|    |i64------------------|
    //
    //  data 23 00 00 00    29 00 00 00 00 00 00 00
    //
    //        local variable area
    //        ===================
    //
    //       |low address                                       high addr|
    //       |                                                           |
    // index |0       1                           2                      |
    //  type |bytes| |i32------|   |padding--|   |i32------|   |padding--|
    //
    //  data 0.....0 31 00 00 00   00 00 00 00   37 00 00 00   00 00 00 00
    //       ^
    //       | 64 bytes, the space for storing function results.
    //       | because the results will overwrite the stack, so it need to
    //       | leave enough space for results, then the data of local variables
    //       | can be still read after function is finish.
    //
    // () -> (i64,i64,i64,i64,i64,i64, i64,i64)
    //        -----------------------  -------
    //        | addr of data           | addr of local vars
    //
    // read the values of data and local vars through the host address.

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (data $d0 (read_only i32 0x11))
            (data $d1 (read_only i32 0x13))
            (data $d2 (read_write i64 0xee))    // init data
            (data $d3 (read_write i32 0xff))    // init data
            (data $d4 (uninit i32))
            (data $d5 (uninit i64))
            (function $test
                (results i64 i64 i64 i64 i64 i64 i64 i64)
                (local $reserved bytes 64 8)
                (local $n1 i32)
                (local $n2 i32)
                (code
                    // store values to data

                    (data.store64 $d2
                        (i64.imm 0x17))
                    (data.store32 $d3
                        (i32.imm 0x19))
                    (data.store32 $d4
                        (i32.imm 0x23))
                    (data.store64 $d5
                        (i64.imm 0x29))

                    // store values to local vars

                    (local.store32 $n1
                        (i32.imm 0x31))
                    (local.store32 $n2
                        (i32.imm 0x37))

                    // get host address of data

                    (host.addr_data $d0)
                    (host.addr_data $d1)
                    (host.addr_data $d2)
                    (host.addr_data $d3)
                    (host.addr_data $d4)
                    (host.addr_data $d5)

                    (host.addr_local $n1)
                    (host.addr_local $n2)
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();

    let function_entry = process_context0.module_images[0]
        .get_function_section()
        .get_function_entry(0);

    let bytecode_text = format_bytecode_as_text(&function_entry.code);

    assert_eq!(
        bytecode_text,
        "\
0x0000  81 01 00 00  17 00 00 00    i64.imm           low:0x00000017  high:0x00000000
        00 00 00 00
0x000c  08 03 00 00  02 00 00 00    data.store64      off:0x00  idx:2
0x0014  80 01 00 00  19 00 00 00    i32.imm           0x00000019
0x001c  09 03 00 00  03 00 00 00    data.store32      off:0x00  idx:3
0x0024  80 01 00 00  23 00 00 00    i32.imm           0x00000023
0x002c  09 03 00 00  04 00 00 00    data.store32      off:0x00  idx:4
0x0034  81 01 00 00  29 00 00 00    i64.imm           low:0x00000029  high:0x00000000
        00 00 00 00
0x0040  08 03 00 00  05 00 00 00    data.store64      off:0x00  idx:5
0x0048  80 01 00 00  31 00 00 00    i32.imm           0x00000031
0x0050  09 02 00 00  00 00 01 00    local.store32     rev:0   off:0x00  idx:1
0x0058  80 01 00 00  37 00 00 00    i32.imm           0x00000037
0x0060  09 02 00 00  00 00 02 00    local.store32     rev:0   off:0x00  idx:2
0x0068  05 0c 00 00  00 00 00 00    host.addr_data    off:0x00  idx:0
0x0070  05 0c 00 00  01 00 00 00    host.addr_data    off:0x00  idx:1
0x0078  05 0c 00 00  02 00 00 00    host.addr_data    off:0x00  idx:2
0x0080  05 0c 00 00  03 00 00 00    host.addr_data    off:0x00  idx:3
0x0088  05 0c 00 00  04 00 00 00    host.addr_data    off:0x00  idx:4
0x0090  05 0c 00 00  05 00 00 00    host.addr_data    off:0x00  idx:5
0x0098  03 0c 00 00  00 00 01 00    host.addr_local   rev:0   off:0x00  idx:1
0x00a0  03 0c 00 00  00 00 02 00    host.addr_local   rev:0   off:0x00  idx:2
0x00a8  00 0a                       end"
    );

    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    let fvs = result0.unwrap();

    assert_eq!(read_memory_i32(fvs[0]), 0x11);
    assert_eq!(read_memory_i32(fvs[1]), 0x13);
    assert_eq!(read_memory_i64(fvs[2]), 0x17);
    assert_eq!(read_memory_i32(fvs[3]), 0x19);
    assert_eq!(read_memory_i32(fvs[4]), 0x23);
    assert_eq!(read_memory_i64(fvs[5]), 0x29);

    // note:
    // depending on the implementation of the stack (the stack frame and local variables),
    // the following 'assert_eq' may fail,
    // because the local variables (as well as their host addresses) will no longer valid
    // when a function exits.

    assert_eq!(read_memory_i32(fvs[6]), 0x31);
    assert_eq!(read_memory_i32(fvs[7]), 0x37);
}

#[test]
fn test_assemble_host_address_offset_of_data_and_local_vars() {
    //        read-only data section
    //        ======================
    //
    //       |low address  high addr|
    //       |                      |
    // index |0            1        |
    //  type |bytes----|  |byte-----|
    //
    //  data 02 03 05 07  11 13 17 19
    //       |     |            |  |
    //       |0    |1           |2 |3
    //
    //        local variable area
    //        ===================
    //
    //       |low address         high addr|
    //       |                             |
    // index |0       1                    |
    //  type |bytes| |bytes----------------|
    //
    //  data 0.....0 23 29 31 37 41 43 47 53
    //       ^       |        |        |  |
    //       |       |4       |5       |6 |7
    //       |
    //       | 64 bytes, the space for storing function results.
    //       | because the results will overwrite the stack, so it need to
    //       | leave enough space for results, then the data of local variables
    //       | can be still read after function is finish.
    //
    // () -> (i64,i64,i64,i64, i64,i64, i64,i64)
    //        ---------------- ----------------
    //        | addr of data   | addr of local vars
    //
    // read the values of data and local vars through the host address.

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (data $d0 (read_only bytes h"02-03-05-07" 8))
            (data $d1 (read_only bytes h"11-13-17-19" 8))
            (function $test
                (results i64 i64 i64 i64 i64 i64 i64 i64)
                (local $reserved bytes 64 8)
                (local $n1 bytes 8 8)
                (code
                    // store values to local vars

                    (local.store64 $n1
                        (i64.imm 0x5347434137312923))

                    // get host address of data

                    (host.addr_data_offset $d0 (i32.imm 0))
                    (host.addr_data_offset $d0 (i32.imm 2))
                    (host.addr_data_offset $d1 (i32.imm 2))
                    (host.addr_data_offset $d1 (i32.imm 3))

                    (host.addr_local_offset $n1 (i32.imm 0))
                    (host.addr_local_offset $n1 (i32.imm 3))
                    (host.addr_local_offset $n1 (i32.imm 6))
                    (host.addr_local_offset $n1 (i32.imm 7))
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();

    let function_entry = process_context0.module_images[0]
        .get_function_section()
        .get_function_entry(0);

    let bytecode_text = format_bytecode_as_text(&function_entry.code);
    // println!("{}", bytecode_text);

    assert_eq!(
        bytecode_text,
        "\
0x0000  81 01 00 00  23 29 31 37    i64.imm           low:0x37312923  high:0x53474341
        41 43 47 53
0x000c  08 02 00 00  00 00 01 00    local.store64     rev:0   off:0x00  idx:1
0x0014  80 01 00 00  00 00 00 00    i32.imm           0x00000000
0x001c  06 0c 00 00  00 00 00 00    host.addr_data_offset  idx:0
0x0024  80 01 00 00  02 00 00 00    i32.imm           0x00000002
0x002c  06 0c 00 00  00 00 00 00    host.addr_data_offset  idx:0
0x0034  80 01 00 00  02 00 00 00    i32.imm           0x00000002
0x003c  06 0c 00 00  01 00 00 00    host.addr_data_offset  idx:1
0x0044  80 01 00 00  03 00 00 00    i32.imm           0x00000003
0x004c  06 0c 00 00  01 00 00 00    host.addr_data_offset  idx:1
0x0054  80 01 00 00  00 00 00 00    i32.imm           0x00000000
0x005c  04 0c 00 00  01 00 00 00    host.addr_local_offset  rev:0   idx:1
0x0064  80 01 00 00  03 00 00 00    i32.imm           0x00000003
0x006c  04 0c 00 00  01 00 00 00    host.addr_local_offset  rev:0   idx:1
0x0074  80 01 00 00  06 00 00 00    i32.imm           0x00000006
0x007c  04 0c 00 00  01 00 00 00    host.addr_local_offset  rev:0   idx:1
0x0084  80 01 00 00  07 00 00 00    i32.imm           0x00000007
0x008c  04 0c 00 00  01 00 00 00    host.addr_local_offset  rev:0   idx:1
0x0094  00 0a                       end"
    );

    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    let fvs = result0.unwrap();

    assert_eq!(read_memory_i8(fvs[0]), 0x02);
    assert_eq!(read_memory_i8(fvs[1]), 0x05);
    assert_eq!(read_memory_i8(fvs[2]), 0x17);
    assert_eq!(read_memory_i8(fvs[3]), 0x19);

    // note:
    // depending on the implementation of the stack (the stack frame and local variables),
    // the following 'assert_eq' may fail,
    // because the local variables (as well as their host addresses) will no longer valid
    // when a function exits.

    assert_eq!(read_memory_i8(fvs[4]), 0x23);
    assert_eq!(read_memory_i8(fvs[5]), 0x37);
    assert_eq!(read_memory_i8(fvs[6]), 0x47);
    assert_eq!(read_memory_i8(fvs[7]), 0x53);
}

#[test]
fn test_assemble_host_address_heap() {
    //        heap
    //       |low address                high addr|
    //       |                                    |
    //  addr |0x100         0x200                 |
    //  type |i32-------|   |i64------------------|
    //
    //  data  02 03 05 07   11 13 17 19 23 29 31 37
    //        ^     ^       ^           ^        ^
    //        |0    |1      |2          |3       |4
    //
    // () -> (i64,i64,i64,i64,i64)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test
                (results i64 i64 i64 i64 i64)
                (code
                    // init the heap size
                    // (drop
                        (heap.resize (i32.imm 1))
                    // )

                    // store values to heap
                    (heap.store32
                        (i64.imm 0x100)
                        (i32.imm 0x07050302)
                    )

                    (heap.store64
                        (i64.imm 0x200)
                        (i64.imm 0x37312923_19171311)
                    )

                    // get host address of heap

                    (host.addr_heap (i64.imm 0x100) 0)
                    (host.addr_heap (i64.imm 0x100) 2)

                    (host.addr_heap (i64.imm 0x200) 0)
                    (host.addr_heap (i64.imm 0x200) 4)
                    (host.addr_heap (i64.imm 0x200) 7)
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();

    let function_entry = process_context0.module_images[0]
        .get_function_section()
        .get_function_entry(0);

    let bytecode_text = format_bytecode_as_text(&function_entry.code);

    assert_eq!(
        bytecode_text,
        "\
0x0000  80 01 00 00  01 00 00 00    i32.imm           0x00000001
0x0008  83 04                       heap.resize
0x000a  00 01                       nop
0x000c  81 01 00 00  00 01 00 00    i64.imm           low:0x00000100  high:0x00000000
        00 00 00 00
0x0018  80 01 00 00  02 03 05 07    i32.imm           0x07050302
0x0020  09 04 00 00                 heap.store32      off:0x00
0x0024  81 01 00 00  00 02 00 00    i64.imm           low:0x00000200  high:0x00000000
        00 00 00 00
0x0030  81 01 00 00  11 13 17 19    i64.imm           low:0x19171311  high:0x37312923
        23 29 31 37
0x003c  08 04 00 00                 heap.store64      off:0x00
0x0040  81 01 00 00  00 01 00 00    i64.imm           low:0x00000100  high:0x00000000
        00 00 00 00
0x004c  07 0c 00 00                 host.addr_heap    off:0x00
0x0050  81 01 00 00  00 01 00 00    i64.imm           low:0x00000100  high:0x00000000
        00 00 00 00
0x005c  07 0c 02 00                 host.addr_heap    off:0x02
0x0060  81 01 00 00  00 02 00 00    i64.imm           low:0x00000200  high:0x00000000
        00 00 00 00
0x006c  07 0c 00 00                 host.addr_heap    off:0x00
0x0070  81 01 00 00  00 02 00 00    i64.imm           low:0x00000200  high:0x00000000
        00 00 00 00
0x007c  07 0c 04 00                 host.addr_heap    off:0x04
0x0080  81 01 00 00  00 02 00 00    i64.imm           low:0x00000200  high:0x00000000
        00 00 00 00
0x008c  07 0c 07 00                 host.addr_heap    off:0x07
0x0090  00 0a                       end"
    );

    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&mut thread_context0, 0, 0, &[]);
    let fvs = result0.unwrap();

    assert_eq!(read_memory_i32(fvs[0]), 0x07050302);
    assert_eq!(read_memory_i16(fvs[1]), 0x0705);
    assert_eq!(read_memory_i64(fvs[2]), 0x3731292319171311);
    assert_eq!(read_memory_i32(fvs[3]), 0x37312923);
    assert_eq!(read_memory_i8(fvs[4]), 0x37);
}

#[test]
fn test_assemble_host_heap_copy() {
    // fn(src_ptr, dst_ptr) -> ()

    // copy src_ptr -> VM heap 0x100 with 8 bytes
    // copy VM heap 0x100 -> dst_ptr with 8 bytes
    //
    //               0x100                        dst_ptr
    //            vm |01234567| --> copy --> host |01234567|
    //                ^
    //       /--copy--/
    //       |
    // host |01234567|
    //      src_ptr

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test
                (param $src_ptr i64)
                (param $dst_ptr i64)
                (code
                    // init the heap size
                    // (drop
                        (heap.resize (i32.imm 1))
                    // )

                    // (host.copy_memory_to_heap dst_offset src_ptr length)
                    // (host.copy_heap_to_memory dst_ptr src_offset length)

                    (host.copy_memory_to_heap
                        (i64.imm 0x100)
                        (local.load64_i64 $src_ptr)
                        (i64.imm 8)
                    )

                    (host.copy_heap_to_memory
                        (local.load64_i64 $dst_ptr)
                        (i64.imm 0x100)
                        (i64.imm 8)
                    )
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let src_buf: &[u8; 8] = b"hello.vm";
    let dst_buf: [u8; 8] = [0; 8];

    let src_ptr = src_buf.as_ptr();
    let dst_ptr = dst_buf.as_ptr();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::U64(src_ptr as usize as u64),
            ForeignValue::U64(dst_ptr as usize as u64),
        ],
    );
    result0.unwrap();

    assert_eq!(&dst_buf, b"hello.vm");
}

#[test]
fn test_assemble_host_memory_copy() {
    // fn(src_ptr, dst_ptr) -> ()

    // copy src_ptr -> dst_ptr
    //
    // host src_ptr  local var     host dst_ptr
    // |01234567| -> |45670123| -> |45670123|

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (function $test
                (param $src_ptr i64)
                (param $dst_ptr i64)
                (local $buf i64)
                (code
                    // (host.memory_copy dst_ptr src_ptr length)
                    (host.memory_copy
                        (host.addr_local $buf 4)
                        (local.load64_i64 $src_ptr)
                        (i64.imm 4)
                    )

                    (host.memory_copy
                        (host.addr_local $buf 0)
                        (i64.inc (local.load64_i64 $src_ptr) 4)
                        (i64.imm 4)
                    )

                    (host.memory_copy
                        (local.load64_i64 $dst_ptr)
                        (host.addr_local $buf 0)
                        (i64.imm 8)
                    )
                )
            )
        )
        "#,
    );

    let program_resource0 = InMemoryProgramResource::new(vec![module_binary]);
    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let src_buf: &[u8; 8] = b"whatever";
    let dst_buf: [u8; 8] = [0; 8];

    let src_ptr = src_buf.as_ptr();
    let dst_ptr = dst_buf.as_ptr();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[
            ForeignValue::U64(src_ptr as usize as u64),
            ForeignValue::U64(dst_ptr as usize as u64),
        ],
    );
    result0.unwrap();

    assert_eq!(&dst_buf, b"everwhat");
}

#[test]
fn test_assemble_host_addr_function_and_callback_function() {
    // C function in "libtest0.so.1"
    // ===============================
    // int do_something(int (*callback_func)(int), int a, int b)
    // {
    //     int s = (callback_func)(a);
    //     return s + b;
    // }
    //
    // VM functions
    // ============
    //
    // fn func0 (a:i32, b:i32)->i32 {
    //     do_something(func1, a, b)
    // }
    //
    // fn func1 (a:i32) -> i32 {
    //     // this is the callback function for external function 'do_something'
    //     a*2
    // }
    //
    // calling path:
    // (11,13) -> func0(VM) -> do_something(C) -> func1(VM) -> do_something(C) -> func0(VM) -> (11*2+13)

    let module_binary = helper_generate_module_image_binary_from_str(
        r#"
        (module $app
            (compiler_version "1.0")
            (depend
                (library $test0 user "libtest0.so.1")
            )
            (external $test0
                (function $do_something "do_something"
                    (params i64 i32 i32)
                    (result i32)
                )
            )
            (function $func0
                (param $a i32)
                (param $b i32)
                (result i32)
                (code
                    (extcall $do_something
                        (host.addr_function $func1)
                        (local.load32_i32 $a)
                        (local.load32_i32 $b)
                    )
                )
            )
            (function $func1
                (param $n i32)
                (result i32)
                (code
                    (i32.mul
                        (local.load32_i32 $n)
                        (i32.imm 2)
                    )
                )
            )
        )
        "#,
    );

    let mut pwd = std::env::current_dir().unwrap();
    if !pwd.ends_with("assembler") {
        // in the VSCode editor `Debug` environment, the `current_dir()` returns
        // the project's root folder.
        // while in both `$ cargo test` and VSCode editor `Run Test` environment,
        // the `current_dir()` returns the current crate path.
        // here canonicalize the test resources path.
        pwd.push("crates");
        pwd.push("assembler");
    }
    pwd.push("tests");

    let program_source_path = pwd.to_str().unwrap();

    let program_resource0 = InMemoryProgramResource::with_settings(
        vec![module_binary],
        &ProgramSettings::new(program_source_path, true, "", ""),
    );

    let process_context0 = program_resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(11), ForeignValue::U32(13)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(11 * 2 + 13)]);

    let result1 = process_function(
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(211), ForeignValue::U32(223)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::U32(211 * 2 + 223)]);
}

fn read_memory_i64(fv: ForeignValue) -> u64 {
    if let ForeignValue::U64(addr) = fv {
        let ptr = addr as *const u64;
        unsafe { std::ptr::read(ptr) }
    } else {
        panic!("The data type of the foreign value does not match.")
    }
}

fn read_memory_i32(fv: ForeignValue) -> u32 {
    if let ForeignValue::U64(addr) = fv {
        let ptr = addr as *const u32;
        unsafe { std::ptr::read(ptr) }
    } else {
        panic!("The data type of the foreign value does not match.")
    }
}

fn read_memory_i16(fv: ForeignValue) -> u16 {
    if let ForeignValue::U64(addr) = fv {
        let ptr = addr as *const u16;
        unsafe { std::ptr::read(ptr) }
    } else {
        panic!("The data type of the foreign value does not match.")
    }
}

fn read_memory_i8(fv: ForeignValue) -> u8 {
    if let ForeignValue::U64(addr) = fv {
        let ptr = addr as *const u8;
        unsafe { std::ptr::read(ptr) }
    } else {
        panic!("The data type of the foreign value does not match.")
    }
}
