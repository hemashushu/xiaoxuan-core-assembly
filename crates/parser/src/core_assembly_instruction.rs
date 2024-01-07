// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::{collections::HashMap, sync::Once};

use ancvm_types::opcode::Opcode;

// ref:
// https://doc.rust-lang.org/stable/std/collections/struct.HashMap.html
// https://doc.rust-lang.org/std/sync/struct.Once.html
static INIT: Once = Once::new();

// assembly instructions are not identical to VM instructions.
// this is the instructions map.
pub static mut INSTRUCTION_MAP: Option<HashMap<&'static str, InstructionSyntaxKind>> = None;

// group instructions according to the syntax
// for easier parsing.
#[derive(Debug, PartialEq, Clone)]
pub enum InstructionSyntaxKind {
    // inst_name                                // no operands
    // (inst_name)                              // no operands
    // (inst_name OPERAND_0 ... OPERAND_N)      // with operands
    NoParams(Opcode, /* operand_count */ u8),

    // (i32.imm 123)
    // (i32.imm 0x123)
    // (i32.imm 0b1010)
    ImmI32,

    // (i64.imm 123)
    // (i64.imm 0x123)
    // (i64.imm 0b1010)
    ImmI64,

    // (f32.imm 3.14)
    // (f32.imm 0x1.23p4)
    ImmF32,

    // (f64.imm 3.14)
    // (f64.imm 0x1.23p4)
    ImmF64,

    // (local.load $name)
    // (local.load $name offset)                // optional offset
    // (host.addr_local $name)
    LocalLoad(Opcode),

    // (local.store $name VALUE)
    // (local.store $name VALUE offset)         // optional offset
    LocalStore(Opcode),

    // (local.load_offset $name OFFSET)
    // (host.addr_local_offset OFFSET)
    LocalOffsetLoad(Opcode),

    // (local.offset_store $name OFFSET VALUE)
    LocalOffsetStore(Opcode),

    // (data.load $name)
    // (data.load $name offset)                 // optional offset
    // (host.addr_data $name)
    DataLoad(Opcode),

    // (data.store $name VALUE)
    // (data.store $name VALUE offset)          // optional offset
    DataStore(Opcode),

    // (data.load_offset $name OFFSET)
    // (host.addr_data_offset $name OFFSET)
    DataOffsetLoad(Opcode),

    // (data.offset_store $name OFFSET VALUE)
    DataOffsetStore(Opcode),

    // (heap.load ADDR)
    // (host.addr_help ADDR)
    // (heap.load offset ADDR)
    HeapLoad(Opcode),

    // (heap.store ADDR VALUE)
    // (heap.store offset ADDR VALUE)
    HeapStore(Opcode),

    // (inst_name VALUE)
    UnaryOp(Opcode),

    // (i32.inc num VALUE)
    // (i32.dec num VALUE)
    // (i64.inc num VALUE)
    // (i64.dec num VALUE)
    UnaryOpWithImmI16(Opcode),

    // (inst_name LHS RHS)
    BinaryOp(Opcode),

    // (when (local...) TEST CONSEQUENT)
    // pesudo instruction, overwrite the original control flow instructions
    When,

    // (if (param...) (result...) (local...)
    //            TEST CONSEQUENT ALTERNATE)
    // pesudo instruction, overwrite the original control flow instructions
    If,

    // (branch (param...) (result...) (local...)
    //     (case TEST_0 CONSEQUENT_0)
    //     ...
    //     (case TEST_N CONSEQUENT_N)
    //     (default CONSEQUENT_DEFAULT) // optional
    // )
    // pesudo instruction, overwrite the original control flow instructions
    Branch,

    // (for (param...) (result...) (local...) INSTRUCTION)
    // pesudo instruction, overwrite the original control flow instructions
    For,

    // instruction sequence:
    //
    // - 'do', for the tesing and branches
    // - 'break', for break recur
    // - 'recur', for recur
    // - 'return', for exit function
    // - 'rerun', for recur function
    Sequence(&'static str),

    // (call $name OPERAND_0 ... OPERAND_N)
    Call,

    // (dyncall OPERAND_FOR_NUM OPERAND_0 ... OPERAND_N)
    DynCall,

    // (envcall num OPERAND_0 ... OPERAND_N)
    EnvCall,

    // (syscall num OPERAND_0 ... OPERAND_N)
    SysCall,

    // (extcall $name OPERAND_0 ... OPERAND_N)
    ExtCall,

    // (debug num)
    Debug,

    // (unreachable num)
    Unreachable,

    // (host.addr_function $name)
    HostAddrFunction,

    // (macro.get_function_public_index $name)
    MacroGetFunctionPublicIndex,
}

pub fn init_instruction_map() {
    INIT.call_once(|| {
        init_instruction_map_internal();
    });
}

fn init_instruction_map_internal() {
    let mut table: HashMap<&'static str, InstructionSyntaxKind> = HashMap::new();

    let mut add = |name: &'static str, inst_kind: InstructionSyntaxKind| {
        table.insert(name, inst_kind);
    };

    // fundamental
    add("nop", InstructionSyntaxKind::NoParams(Opcode::nop, 0));
    add("zero", InstructionSyntaxKind::NoParams(Opcode::zero, 0));
    add("drop", InstructionSyntaxKind::NoParams(Opcode::drop, 1));
    // add(
    //     "duplicate",
    //     InstructionSyntaxKind::NoParams(Opcode::duplicate, 1),
    // );
    // add("swap", InstructionSyntaxKind::NoParams(Opcode::swap, 2));
    add(
        "select_nez",
        InstructionSyntaxKind::NoParams(Opcode::select_nez, 3),
    );

    // note:
    // 'i32.imm', 'i64.imm', 'f32.imm', 'f64.imm' are replaced with pesudo instructions

    // load variables
    add(
        "local.load64_i64",
        InstructionSyntaxKind::LocalLoad(Opcode::local_load64_i64),
    );
    add(
        "local.load64_f64",
        InstructionSyntaxKind::LocalLoad(Opcode::local_load64_f64),
    );
    add(
        "local.load32_i32",
        InstructionSyntaxKind::LocalLoad(Opcode::local_load32_i32),
    );
    add(
        "local.load32_i16_s",
        InstructionSyntaxKind::LocalLoad(Opcode::local_load32_i16_s),
    );
    add(
        "local.load32_i16_u",
        InstructionSyntaxKind::LocalLoad(Opcode::local_load32_i16_u),
    );
    add(
        "local.load32_i8_s",
        InstructionSyntaxKind::LocalLoad(Opcode::local_load32_i8_s),
    );
    add(
        "local.load32_i8_u",
        InstructionSyntaxKind::LocalLoad(Opcode::local_load32_i8_u),
    );
    add(
        "local.load32_f32",
        InstructionSyntaxKind::LocalLoad(Opcode::local_load32_f32),
    );
    add(
        "local.store64",
        InstructionSyntaxKind::LocalStore(Opcode::local_store64),
    );
    add(
        "local.store32",
        InstructionSyntaxKind::LocalStore(Opcode::local_store32),
    );
    add(
        "local.store16",
        InstructionSyntaxKind::LocalStore(Opcode::local_store16),
    );
    add(
        "local.store8",
        InstructionSyntaxKind::LocalStore(Opcode::local_store8),
    );

    add(
        "local.offset_load64_i64",
        InstructionSyntaxKind::LocalOffsetLoad(Opcode::local_offset_load64_i64),
    );
    add(
        "local.offset_load64_f64",
        InstructionSyntaxKind::LocalOffsetLoad(Opcode::local_offset_load64_f64),
    );
    add(
        "local.offset_load32_i32",
        InstructionSyntaxKind::LocalOffsetLoad(Opcode::local_offset_load32_i32),
    );
    add(
        "local.offset_load32_i16_s",
        InstructionSyntaxKind::LocalOffsetLoad(Opcode::local_offset_load32_i16_s),
    );
    add(
        "local.offset_load32_i16_u",
        InstructionSyntaxKind::LocalOffsetLoad(Opcode::local_offset_load32_i16_u),
    );
    add(
        "local.offset_load32_i8_s",
        InstructionSyntaxKind::LocalOffsetLoad(Opcode::local_offset_load32_i8_s),
    );
    add(
        "local.offset_load32_i8_u",
        InstructionSyntaxKind::LocalOffsetLoad(Opcode::local_offset_load32_i8_u),
    );
    add(
        "local.offset_load32_f32",
        InstructionSyntaxKind::LocalOffsetLoad(Opcode::local_offset_load32_f32),
    );
    add(
        "local.offset_store64",
        InstructionSyntaxKind::LocalOffsetStore(Opcode::local_offset_store64),
    );
    add(
        "local.offset_store32",
        InstructionSyntaxKind::LocalOffsetStore(Opcode::local_offset_store32),
    );
    add(
        "local.offset_store16",
        InstructionSyntaxKind::LocalOffsetStore(Opcode::local_offset_store16),
    );
    add(
        "local.offset_store8",
        InstructionSyntaxKind::LocalOffsetStore(Opcode::local_offset_store8),
    );

    // data
    add(
        "data.load64_i64",
        InstructionSyntaxKind::DataLoad(Opcode::data_load64_i64),
    );
    add(
        "data.load64_f64",
        InstructionSyntaxKind::DataLoad(Opcode::data_load64_f64),
    );
    add(
        "data.load32_i32",
        InstructionSyntaxKind::DataLoad(Opcode::data_load32_i32),
    );
    add(
        "data.load32_i16_s",
        InstructionSyntaxKind::DataLoad(Opcode::data_load32_i16_s),
    );
    add(
        "data.load32_i16_u",
        InstructionSyntaxKind::DataLoad(Opcode::data_load32_i16_u),
    );
    add(
        "data.load32_i8_s",
        InstructionSyntaxKind::DataLoad(Opcode::data_load32_i8_s),
    );
    add(
        "data.load32_i8_u",
        InstructionSyntaxKind::DataLoad(Opcode::data_load32_i8_u),
    );
    add(
        "data.load32_f32",
        InstructionSyntaxKind::DataLoad(Opcode::data_load32_f32),
    );
    add(
        "data.store64",
        InstructionSyntaxKind::DataStore(Opcode::data_store64),
    );
    add(
        "data.store32",
        InstructionSyntaxKind::DataStore(Opcode::data_store32),
    );
    add(
        "data.store16",
        InstructionSyntaxKind::DataStore(Opcode::data_store16),
    );
    add(
        "data.store8",
        InstructionSyntaxKind::DataStore(Opcode::data_store8),
    );

    add(
        "data.offset_load64_i64",
        InstructionSyntaxKind::DataOffsetLoad(Opcode::data_offset_load64_i64),
    );
    add(
        "data.offset_load64_f64",
        InstructionSyntaxKind::DataOffsetLoad(Opcode::data_offset_load64_f64),
    );
    add(
        "data.offset_load32_i32",
        InstructionSyntaxKind::DataOffsetLoad(Opcode::data_offset_load32_i32),
    );
    add(
        "data.offset_load32_i16_s",
        InstructionSyntaxKind::DataOffsetLoad(Opcode::data_offset_load32_i16_s),
    );
    add(
        "data.offset_load32_i16_u",
        InstructionSyntaxKind::DataOffsetLoad(Opcode::data_offset_load32_i16_u),
    );
    add(
        "data.offset_load32_i8_s",
        InstructionSyntaxKind::DataOffsetLoad(Opcode::data_offset_load32_i8_s),
    );
    add(
        "data.offset_load32_i8_u",
        InstructionSyntaxKind::DataOffsetLoad(Opcode::data_offset_load32_i8_u),
    );
    add(
        "data.offset_load32_f32",
        InstructionSyntaxKind::DataOffsetLoad(Opcode::data_offset_load32_f32),
    );
    add(
        "data.offset_store64",
        InstructionSyntaxKind::DataOffsetStore(Opcode::data_offset_store64),
    );
    add(
        "data.offset_store32",
        InstructionSyntaxKind::DataOffsetStore(Opcode::data_offset_store32),
    );
    add(
        "data.offset_store16",
        InstructionSyntaxKind::DataOffsetStore(Opcode::data_offset_store16),
    );
    add(
        "data.offset_store8",
        InstructionSyntaxKind::DataOffsetStore(Opcode::data_offset_store8),
    );

    // heap
    add(
        "heap.load64_i64",
        InstructionSyntaxKind::HeapLoad(Opcode::heap_load64_i64),
    );
    add(
        "heap.load64_f64",
        InstructionSyntaxKind::HeapLoad(Opcode::heap_load64_f64),
    );
    add(
        "heap.load32_i32",
        InstructionSyntaxKind::HeapLoad(Opcode::heap_load32_i32),
    );
    add(
        "heap.load32_i16_s",
        InstructionSyntaxKind::HeapLoad(Opcode::heap_load32_i16_s),
    );
    add(
        "heap.load32_i16_u",
        InstructionSyntaxKind::HeapLoad(Opcode::heap_load32_i16_u),
    );
    add(
        "heap.load32_i8_s",
        InstructionSyntaxKind::HeapLoad(Opcode::heap_load32_i8_s),
    );
    add(
        "heap.load32_i8_u",
        InstructionSyntaxKind::HeapLoad(Opcode::heap_load32_i8_u),
    );
    add(
        "heap.load32_f32",
        InstructionSyntaxKind::HeapLoad(Opcode::heap_load32_f32),
    );
    add(
        "heap.store64",
        InstructionSyntaxKind::HeapStore(Opcode::heap_store64),
    );
    add(
        "heap.store32",
        InstructionSyntaxKind::HeapStore(Opcode::heap_store32),
    );
    add(
        "heap.store16",
        InstructionSyntaxKind::HeapStore(Opcode::heap_store16),
    );
    add(
        "heap.store8",
        InstructionSyntaxKind::HeapStore(Opcode::heap_store8),
    );

    add(
        "heap.fill",
        InstructionSyntaxKind::NoParams(Opcode::heap_fill, 3),
    );
    add(
        "heap.copy",
        InstructionSyntaxKind::NoParams(Opcode::heap_copy, 3),
    );
    add(
        "heap.capacity",
        InstructionSyntaxKind::NoParams(Opcode::heap_capacity, 0),
    );
    add(
        "heap.resize",
        InstructionSyntaxKind::NoParams(Opcode::heap_resize, 1),
    );

    // conversion
    add(
        "i32.truncate_i64",
        InstructionSyntaxKind::UnaryOp(Opcode::i32_truncate_i64),
    );
    add(
        "i64.extend_i32_s",
        InstructionSyntaxKind::UnaryOp(Opcode::i64_extend_i32_s),
    );
    add(
        "i64.extend_i32_u",
        InstructionSyntaxKind::UnaryOp(Opcode::i64_extend_i32_u),
    );
    add(
        "f32.demote_f64",
        InstructionSyntaxKind::UnaryOp(Opcode::f32_demote_f64),
    );
    add(
        "f64.promote_f32",
        InstructionSyntaxKind::UnaryOp(Opcode::f64_promote_f32),
    );

    add(
        "i32.convert_f32_s",
        InstructionSyntaxKind::UnaryOp(Opcode::i32_convert_f32_s),
    );
    add(
        "i32.convert_f32_u",
        InstructionSyntaxKind::UnaryOp(Opcode::i32_convert_f32_u),
    );
    add(
        "i32.convert_f64_s",
        InstructionSyntaxKind::UnaryOp(Opcode::i32_convert_f64_s),
    );
    add(
        "i32.convert_f64_u",
        InstructionSyntaxKind::UnaryOp(Opcode::i32_convert_f64_u),
    );
    add(
        "i64.convert_f32_s",
        InstructionSyntaxKind::UnaryOp(Opcode::i64_convert_f32_s),
    );
    add(
        "i64.convert_f32_u",
        InstructionSyntaxKind::UnaryOp(Opcode::i64_convert_f32_u),
    );
    add(
        "i64.convert_f64_s",
        InstructionSyntaxKind::UnaryOp(Opcode::i64_convert_f64_s),
    );
    add(
        "i64.convert_f64_u",
        InstructionSyntaxKind::UnaryOp(Opcode::i64_convert_f64_u),
    );

    add(
        "f32.convert_i32_s",
        InstructionSyntaxKind::UnaryOp(Opcode::f32_convert_i32_s),
    );
    add(
        "f32.convert_i32_u",
        InstructionSyntaxKind::UnaryOp(Opcode::f32_convert_i32_u),
    );
    add(
        "f32.convert_i64_s",
        InstructionSyntaxKind::UnaryOp(Opcode::f32_convert_i64_s),
    );
    add(
        "f32.convert_i64_u",
        InstructionSyntaxKind::UnaryOp(Opcode::f32_convert_i64_u),
    );
    add(
        "f64.convert_i32_s",
        InstructionSyntaxKind::UnaryOp(Opcode::f64_convert_i32_s),
    );
    add(
        "f64.convert_i32_u",
        InstructionSyntaxKind::UnaryOp(Opcode::f64_convert_i32_u),
    );
    add(
        "f64.convert_i64_s",
        InstructionSyntaxKind::UnaryOp(Opcode::f64_convert_i64_s),
    );
    add(
        "f64.convert_i64_u",
        InstructionSyntaxKind::UnaryOp(Opcode::f64_convert_i64_u),
    );

    // comparsion
    add("i32.eqz", InstructionSyntaxKind::UnaryOp(Opcode::i32_eqz)); // UnaryOp
    add("i32.nez", InstructionSyntaxKind::UnaryOp(Opcode::i32_nez)); // UnaryOp
    add("i32.eq", InstructionSyntaxKind::BinaryOp(Opcode::i32_eq));
    add("i32.ne", InstructionSyntaxKind::BinaryOp(Opcode::i32_ne));
    add(
        "i32.lt_s",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_lt_s),
    );
    add(
        "i32.lt_u",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_lt_u),
    );
    add(
        "i32.gt_s",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_gt_s),
    );
    add(
        "i32.gt_u",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_gt_u),
    );
    add(
        "i32.le_s",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_le_s),
    );
    add(
        "i32.le_u",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_le_u),
    );
    add(
        "i32.ge_s",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_ge_s),
    );
    add(
        "i32.ge_u",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_ge_u),
    );

    add("i64.eqz", InstructionSyntaxKind::UnaryOp(Opcode::i64_eqz)); // UnaryOp
    add("i64.nez", InstructionSyntaxKind::UnaryOp(Opcode::i64_nez)); // UnaryOp
    add("i64.eq", InstructionSyntaxKind::BinaryOp(Opcode::i64_eq));
    add("i64.ne", InstructionSyntaxKind::BinaryOp(Opcode::i64_ne));
    add(
        "i64.lt_s",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_lt_s),
    );
    add(
        "i64.lt_u",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_lt_u),
    );
    add(
        "i64.gt_s",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_gt_s),
    );
    add(
        "i64.gt_u",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_gt_u),
    );
    add(
        "i64.le_s",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_le_s),
    );
    add(
        "i64.le_u",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_le_u),
    );
    add(
        "i64.ge_s",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_ge_s),
    );
    add(
        "i64.ge_u",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_ge_u),
    );

    add("f32.eq", InstructionSyntaxKind::BinaryOp(Opcode::f32_eq));
    add("f32.ne", InstructionSyntaxKind::BinaryOp(Opcode::f32_ne));
    add("f32.lt", InstructionSyntaxKind::BinaryOp(Opcode::f32_lt));
    add("f32.gt", InstructionSyntaxKind::BinaryOp(Opcode::f32_gt));
    add("f32.le", InstructionSyntaxKind::BinaryOp(Opcode::f32_le));
    add("f32.ge", InstructionSyntaxKind::BinaryOp(Opcode::f32_ge));

    add("f64.eq", InstructionSyntaxKind::BinaryOp(Opcode::f64_eq));
    add("f64.ne", InstructionSyntaxKind::BinaryOp(Opcode::f64_ne));
    add("f64.lt", InstructionSyntaxKind::BinaryOp(Opcode::f64_lt));
    add("f64.gt", InstructionSyntaxKind::BinaryOp(Opcode::f64_gt));
    add("f64.le", InstructionSyntaxKind::BinaryOp(Opcode::f64_le));
    add("f64.ge", InstructionSyntaxKind::BinaryOp(Opcode::f64_ge));

    // arithmetic
    add("i32.add", InstructionSyntaxKind::BinaryOp(Opcode::i32_add));
    add("i32.sub", InstructionSyntaxKind::BinaryOp(Opcode::i32_sub));
    add("i32.mul", InstructionSyntaxKind::BinaryOp(Opcode::i32_mul));
    add(
        "i32.div_s",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_div_s),
    );
    add(
        "i32.div_u",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_div_u),
    );
    add(
        "i32.rem_s",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_rem_s),
    );
    add(
        "i32.rem_u",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_rem_u),
    );
    add(
        "i32.inc",
        InstructionSyntaxKind::UnaryOpWithImmI16(Opcode::i32_inc),
    ); // UnaryOpParamI16
    add(
        "i32.dec",
        InstructionSyntaxKind::UnaryOpWithImmI16(Opcode::i32_dec),
    ); // UnaryOpParamI16

    add("i64.add", InstructionSyntaxKind::BinaryOp(Opcode::i64_add));
    add("i64.sub", InstructionSyntaxKind::BinaryOp(Opcode::i64_sub));
    add("i64.mul", InstructionSyntaxKind::BinaryOp(Opcode::i64_mul));
    add(
        "i64.div_s",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_div_s),
    );
    add(
        "i64.div_u",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_div_u),
    );
    add(
        "i64.rem_s",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_rem_s),
    );
    add(
        "i64.rem_u",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_rem_u),
    );
    add(
        "i64.inc",
        InstructionSyntaxKind::UnaryOpWithImmI16(Opcode::i64_inc),
    ); // UnaryOpParamI16
    add(
        "i64.dec",
        InstructionSyntaxKind::UnaryOpWithImmI16(Opcode::i64_dec),
    ); // UnaryOpParamI16

    add("f32.add", InstructionSyntaxKind::BinaryOp(Opcode::f32_add));
    add("f32.sub", InstructionSyntaxKind::BinaryOp(Opcode::f32_sub));
    add("f32.mul", InstructionSyntaxKind::BinaryOp(Opcode::f32_mul));
    add("f32.div", InstructionSyntaxKind::BinaryOp(Opcode::f32_div));

    add("f64.add", InstructionSyntaxKind::BinaryOp(Opcode::f64_add));
    add("f64.sub", InstructionSyntaxKind::BinaryOp(Opcode::f64_sub));
    add("f64.mul", InstructionSyntaxKind::BinaryOp(Opcode::f64_mul));
    add("f64.div", InstructionSyntaxKind::BinaryOp(Opcode::f64_div));

    // bitwise
    add("i32.and", InstructionSyntaxKind::BinaryOp(Opcode::i32_and));
    add("i32.or", InstructionSyntaxKind::BinaryOp(Opcode::i32_or));
    add("i32.xor", InstructionSyntaxKind::BinaryOp(Opcode::i32_xor));
    add(
        "i32.shift_left",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_shift_left),
    );
    add(
        "i32.shift_right_s",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_shift_right_s),
    );
    add(
        "i32.shift_right_u",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_shift_right_u),
    );
    add(
        "i32.rotate_left",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_rotate_left),
    );
    add(
        "i32.rotate_right",
        InstructionSyntaxKind::BinaryOp(Opcode::i32_rotate_right),
    );
    add("i32.not", InstructionSyntaxKind::UnaryOp(Opcode::i32_not)); // UnaryOp
    add(
        "i32.leading_zeros",
        InstructionSyntaxKind::UnaryOp(Opcode::i32_leading_zeros),
    ); // UnaryOp
    add(
        "i32.leading_ones",
        InstructionSyntaxKind::UnaryOp(Opcode::i32_leading_ones),
    ); // UnaryOp
    add(
        "i32.trailing_zeros",
        InstructionSyntaxKind::UnaryOp(Opcode::i32_trailing_zeros),
    ); // UnaryOp
    add(
        "i32.count_ones",
        InstructionSyntaxKind::UnaryOp(Opcode::i32_count_ones),
    ); // UnaryOp

    add("i64.and", InstructionSyntaxKind::BinaryOp(Opcode::i64_and));
    add("i64.or", InstructionSyntaxKind::BinaryOp(Opcode::i64_or));
    add("i64.xor", InstructionSyntaxKind::BinaryOp(Opcode::i64_xor));
    add(
        "i64.shift_left",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_shift_left),
    );
    add(
        "i64.shift_right_s",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_shift_right_s),
    );
    add(
        "i64.shift_right_u",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_shift_right_u),
    );
    add(
        "i64.rotate_left",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_rotate_left),
    );
    add(
        "i64.rotate_right",
        InstructionSyntaxKind::BinaryOp(Opcode::i64_rotate_right),
    );
    add("i64.not", InstructionSyntaxKind::UnaryOp(Opcode::i64_not)); // UnaryOp
    add(
        "i64.leading_zeros",
        InstructionSyntaxKind::UnaryOp(Opcode::i64_leading_zeros),
    ); // UnaryOp
    add(
        "i64.leading_ones",
        InstructionSyntaxKind::UnaryOp(Opcode::i64_leading_ones),
    ); // UnaryOp
    add(
        "i64.trailing_zeros",
        InstructionSyntaxKind::UnaryOp(Opcode::i64_trailing_zeros),
    ); // UnaryOp
    add(
        "i64.count_ones",
        InstructionSyntaxKind::UnaryOp(Opcode::i64_count_ones),
    ); // UnaryOp

    // math
    add("i32.abs", InstructionSyntaxKind::UnaryOp(Opcode::i32_abs));
    add("i32.neg", InstructionSyntaxKind::UnaryOp(Opcode::i32_neg));
    //
    add("i64.abs", InstructionSyntaxKind::UnaryOp(Opcode::i64_abs));
    add("i64.neg", InstructionSyntaxKind::UnaryOp(Opcode::i64_neg));
    //
    add("f32.abs", InstructionSyntaxKind::UnaryOp(Opcode::f32_abs));
    add("f32.neg", InstructionSyntaxKind::UnaryOp(Opcode::f32_neg));
    add("f32.ceil", InstructionSyntaxKind::UnaryOp(Opcode::f32_ceil));
    add(
        "f32.floor",
        InstructionSyntaxKind::UnaryOp(Opcode::f32_floor),
    );
    add(
        "f32.round_half_away_from_zero",
        InstructionSyntaxKind::UnaryOp(Opcode::f32_round_half_away_from_zero),
    );
    add(
        "f32.round_half_to_even",
        InstructionSyntaxKind::UnaryOp(Opcode::f32_round_half_to_even),
    );
    add(
        "f32.trunc",
        InstructionSyntaxKind::UnaryOp(Opcode::f32_trunc),
    );
    add(
        "f32.fract",
        InstructionSyntaxKind::UnaryOp(Opcode::f32_fract),
    );
    add("f32.sqrt", InstructionSyntaxKind::UnaryOp(Opcode::f32_sqrt));
    add("f32.cbrt", InstructionSyntaxKind::UnaryOp(Opcode::f32_cbrt));
    add("f32.exp", InstructionSyntaxKind::UnaryOp(Opcode::f32_exp));
    add("f32.exp2", InstructionSyntaxKind::UnaryOp(Opcode::f32_exp2));
    add("f32.ln", InstructionSyntaxKind::UnaryOp(Opcode::f32_ln));
    add("f32.log2", InstructionSyntaxKind::UnaryOp(Opcode::f32_log2));
    add(
        "f32.log10",
        InstructionSyntaxKind::UnaryOp(Opcode::f32_log10),
    );
    add("f32.sin", InstructionSyntaxKind::UnaryOp(Opcode::f32_sin));
    add("f32.cos", InstructionSyntaxKind::UnaryOp(Opcode::f32_cos));
    add("f32.tan", InstructionSyntaxKind::UnaryOp(Opcode::f32_tan));
    add("f32.asin", InstructionSyntaxKind::UnaryOp(Opcode::f32_asin));
    add("f32.acos", InstructionSyntaxKind::UnaryOp(Opcode::f32_acos));
    add("f32.atan", InstructionSyntaxKind::UnaryOp(Opcode::f32_atan));
    add(
        "f32.copysign",
        InstructionSyntaxKind::BinaryOp(Opcode::f32_copysign),
    ); // BinaryOp
    add("f32.pow", InstructionSyntaxKind::BinaryOp(Opcode::f32_pow)); // BinaryOp
    add("f32.log", InstructionSyntaxKind::BinaryOp(Opcode::f32_log)); // BinaryOp
    add("f32.min", InstructionSyntaxKind::BinaryOp(Opcode::f32_min)); // BinaryOp
    add("f32.max", InstructionSyntaxKind::BinaryOp(Opcode::f32_max)); // BinaryOp

    add("f64.abs", InstructionSyntaxKind::UnaryOp(Opcode::f64_abs));
    add("f64.neg", InstructionSyntaxKind::UnaryOp(Opcode::f64_neg));
    add("f64.ceil", InstructionSyntaxKind::UnaryOp(Opcode::f64_ceil));
    add(
        "f64.floor",
        InstructionSyntaxKind::UnaryOp(Opcode::f64_floor),
    );
    add(
        "f64.round_half_away_from_zero",
        InstructionSyntaxKind::UnaryOp(Opcode::f64_round_half_away_from_zero),
    );
    add(
        "f64.round_half_to_even",
        InstructionSyntaxKind::UnaryOp(Opcode::f64_round_half_to_even),
    );
    add(
        "f64.trunc",
        InstructionSyntaxKind::UnaryOp(Opcode::f64_trunc),
    );
    add(
        "f64.fract",
        InstructionSyntaxKind::UnaryOp(Opcode::f64_fract),
    );
    add("f64.sqrt", InstructionSyntaxKind::UnaryOp(Opcode::f64_sqrt));
    add("f64.cbrt", InstructionSyntaxKind::UnaryOp(Opcode::f64_cbrt));
    add("f64.exp", InstructionSyntaxKind::UnaryOp(Opcode::f64_exp));
    add("f64.exp2", InstructionSyntaxKind::UnaryOp(Opcode::f64_exp2));
    add("f64.ln", InstructionSyntaxKind::UnaryOp(Opcode::f64_ln));
    add("f64.log2", InstructionSyntaxKind::UnaryOp(Opcode::f64_log2));
    add(
        "f64.log10",
        InstructionSyntaxKind::UnaryOp(Opcode::f64_log10),
    );
    add("f64.sin", InstructionSyntaxKind::UnaryOp(Opcode::f64_sin));
    add("f64.cos", InstructionSyntaxKind::UnaryOp(Opcode::f64_cos));
    add("f64.tan", InstructionSyntaxKind::UnaryOp(Opcode::f64_tan));
    add("f64.asin", InstructionSyntaxKind::UnaryOp(Opcode::f64_asin));
    add("f64.acos", InstructionSyntaxKind::UnaryOp(Opcode::f64_acos));
    add("f64.atan", InstructionSyntaxKind::UnaryOp(Opcode::f64_atan));
    add(
        "f64.copysign",
        InstructionSyntaxKind::BinaryOp(Opcode::f64_copysign),
    ); // BinaryOp
    add("f64.pow", InstructionSyntaxKind::BinaryOp(Opcode::f64_pow)); // BinaryOp
    add("f64.log", InstructionSyntaxKind::BinaryOp(Opcode::f64_log)); // BinaryOp
    add("f64.min", InstructionSyntaxKind::BinaryOp(Opcode::f64_min)); // BinaryOp
    add("f64.max", InstructionSyntaxKind::BinaryOp(Opcode::f64_max)); // BinaryOp

    // control flow
    // note: all instructions in this catalog are replaced with pesudo instructions

    // function call
    // note: all instructions in this catalog are replaced with pesudo instructions

    // host
    add("panic", InstructionSyntaxKind::NoParams(Opcode::panic, 0));
    add("unreachable", InstructionSyntaxKind::Unreachable);
    add("debug", InstructionSyntaxKind::Debug);

    add(
        "host.addr_local",
        InstructionSyntaxKind::LocalLoad(Opcode::host_addr_local),
    );
    add(
        "host.addr_local_offset",
        InstructionSyntaxKind::LocalOffsetLoad(Opcode::host_addr_local_offset),
    );
    add(
        "host.addr_data",
        InstructionSyntaxKind::DataLoad(Opcode::host_addr_data),
    );
    add(
        "host.addr_data_offset",
        InstructionSyntaxKind::DataOffsetLoad(Opcode::host_addr_data_offset),
    );
    add(
        "host.addr_heap",
        InstructionSyntaxKind::HeapLoad(Opcode::host_addr_heap),
    );
    add(
        "host.addr_function",
        InstructionSyntaxKind::HostAddrFunction,
    );
    add(
        "host.copy_heap_to_memory",
        InstructionSyntaxKind::NoParams(Opcode::host_copy_heap_to_memory, 3),
    );
    add(
        "host.copy_memory_to_heap",
        InstructionSyntaxKind::NoParams(Opcode::host_copy_memory_to_heap, 3),
    );
    add(
        "host.memory_copy",
        InstructionSyntaxKind::NoParams(Opcode::host_memory_copy, 3),
    );

    // pesudo instructions
    add("i32.imm", InstructionSyntaxKind::ImmI32);
    add("i64.imm", InstructionSyntaxKind::ImmI64);
    add("f32.imm", InstructionSyntaxKind::ImmF32);
    add("f64.imm", InstructionSyntaxKind::ImmF64);

    add("when", InstructionSyntaxKind::When);
    add("if", InstructionSyntaxKind::If);
    add("branch", InstructionSyntaxKind::Branch);
    add("for", InstructionSyntaxKind::For);

    add("do", InstructionSyntaxKind::Sequence("do"));
    add("break", InstructionSyntaxKind::Sequence("break"));
    add("return", InstructionSyntaxKind::Sequence("return"));
    add("recur", InstructionSyntaxKind::Sequence("recur"));
    add("rerun", InstructionSyntaxKind::Sequence("rerun"));

    add("call", InstructionSyntaxKind::Call);
    add("dyncall", InstructionSyntaxKind::DynCall);
    add("envcall", InstructionSyntaxKind::EnvCall);
    add("syscall", InstructionSyntaxKind::SysCall);
    add("extcall", InstructionSyntaxKind::ExtCall);

    // macros
    add(
        "macro.get_function_public_index",
        InstructionSyntaxKind::MacroGetFunctionPublicIndex,
    );

    unsafe { INSTRUCTION_MAP = Some(table) };
}
