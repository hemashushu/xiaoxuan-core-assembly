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
pub static mut INSTRUCTION_KIND_TABLE: Option<HashMap<&'static str, InstructionKind>> = None;

// the kind of assembly instruction
#[derive(Debug, PartialEq, Clone)]
pub enum InstructionKind {
    // inst_name                            ;; no operands
    // (inst_name)                          ;; no operands
    // (inst_name OPERAND_0 ... OPERAND_N)   ;; has operands
    NoParams(Opcode, /* operand_count */ u8),

    // (local.load $name)
    // (local.load $name offset)              ;; optional offset
    //
    // includes 'host_addr_local'
    LocalLoad(Opcode),

    // (local.store $name OPERAND)
    // (local.store $name offset OPERAND)     ;; optional offset
    LocalStore(Opcode),

    // (local.load_long $name OPERAND_FOR_OFFSET)
    //
    // includes 'host_addr_local_long'
    LocalLongLoad(Opcode),

    // (local.long_store $name OPERAND_FOR_OFFSET OPERAND)
    LocalLongStore(Opcode),

    // (data.load $name)
    // (data.load $name offset)              ;; optional offset
    //
    // includes 'host_addr_data'
    DataLoad(Opcode),

    // (data.store $name OPERAND)
    // (data.store $name offset OPERAND)     ;; optional offset
    DataStore(Opcode),

    // (data.load_long $name OPERAND_FOR_OFFSET)
    //
    // includes 'host_addr_data_long'
    DataLongLoad(Opcode),

    // (data.long_store $name OPERAND_FOR_OFFSET OPERAND)
    DataLongStore(Opcode),

    // (heap.load OPERAND_FOR_ADDR)
    // (heap.load offset OPERAND_FOR_ADDR)
    //
    // includes 'host_addr_heap'
    HeapLoad(Opcode),

    // (heap.store OPERAND_FOR_ADDR OPERAND)
    // (heap.store offset OPERAND_FOR_ADDR OPERAND)
    HeapStore(Opcode),

    // (inst_name OPERAND)
    UnaryOp(Opcode),

    // (i32.inc num OPERAND)
    // (i32.dec num OPERAND)
    // (i64.inc num OPERAND)
    // (i64.dec num OPERAND)
    UnaryOpParamI16(Opcode),

    // (inst_name OPERAND_LHS OPERAND_RHS)
    BinaryOp(Opcode),

    // (i32.imm 123)
    // (i32.imm 0x123)
    // (i32.imm 0b1010)
    //
    // pesudo instruction, overwrite the original instruction i32.imm
    ImmI32,

    // (i64.imm 123)
    // (i64.imm 0x123)
    // (i64.imm 0b1010)
    //
    // pesudo instruction, overwrite the original instruction i64.imm
    ImmI64,

    // (f32.imm 3.14)
    // (f32.imm 0x314)
    // (f32.imm 0b0011)
    //
    // pesudo instruction, overwrite the original instruction f32.imm
    ImmF32,

    // (f64.imm 3.14)
    // (f64.imm 0x314)
    // (f64.imm 0b0011)
    //
    // pesudo instruction, overwrite the original instruction f64.imm
    ImmF64,

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
    //     (default CONSEQUENT_DEFAULT) ;; optional
    // )
    // pesudo instruction, overwrite the original control flow instructions
    Branch,

    // (for (param...) (result...) (local...) INSTRUCTION)
    // pesudo instruction, overwrite the original control flow instructions
    For,

    // instruction sequence:
    //
    // - 'code', for the function body
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

    //
    Debug,
    Unreachable,

    // (host.addr_function $name)
    HostAddrFunction,

    // (macro.get_function_public_index $name)
    MacroGetFunctionPublicIndex,
}

pub fn init_instruction_kind_table() {
    INIT.call_once(|| {
        init_instruction_kind_table_internal();
    });
}

fn init_instruction_kind_table_internal() {
    let mut table: HashMap<&'static str, InstructionKind> = HashMap::new();

    let mut add = |name: &'static str, inst_kind: InstructionKind| {
        table.insert(name, inst_kind);
    };

    // fundamental
    add("nop", InstructionKind::NoParams(Opcode::nop, 0));
    add("zero", InstructionKind::NoParams(Opcode::zero, 0));
    add("drop", InstructionKind::NoParams(Opcode::drop, 1));
    add("duplicate", InstructionKind::NoParams(Opcode::duplicate, 1));
    add("swap", InstructionKind::NoParams(Opcode::swap, 2));
    add(
        "select_nez",
        InstructionKind::NoParams(Opcode::select_nez, 3),
    );

    // note:
    // 'i32.imm', 'i64.imm', 'f32.imm', 'f64.imm' are replaced with pesudo instructions

    // load variables
    add(
        "local.load64_i64",
        InstructionKind::LocalLoad(Opcode::local_load64_i64),
    );
    add(
        "local.load64_f64",
        InstructionKind::LocalLoad(Opcode::local_load64_f64),
    );
    add(
        "local.load32_i32",
        InstructionKind::LocalLoad(Opcode::local_load32_i32),
    );
    add(
        "local.load32_i16_s",
        InstructionKind::LocalLoad(Opcode::local_load32_i16_s),
    );
    add(
        "local.load32_i16_u",
        InstructionKind::LocalLoad(Opcode::local_load32_i16_u),
    );
    add(
        "local.load32_i8_s",
        InstructionKind::LocalLoad(Opcode::local_load32_i8_s),
    );
    add(
        "local.load32_i8_u",
        InstructionKind::LocalLoad(Opcode::local_load32_i8_u),
    );
    add(
        "local.load32_f32",
        InstructionKind::LocalLoad(Opcode::local_load32_f32),
    );
    add(
        "local.store64",
        InstructionKind::LocalStore(Opcode::local_store64),
    );
    add(
        "local.store32",
        InstructionKind::LocalStore(Opcode::local_store32),
    );
    add(
        "local.store16",
        InstructionKind::LocalStore(Opcode::local_store16),
    );
    add(
        "local.store8",
        InstructionKind::LocalStore(Opcode::local_store8),
    );

    add(
        "local.long_load64_i64",
        InstructionKind::LocalLongLoad(Opcode::local_long_load64_i64),
    );
    add(
        "local.long_load64_f64",
        InstructionKind::LocalLongLoad(Opcode::local_long_load64_f64),
    );
    add(
        "local.long_load32_i32",
        InstructionKind::LocalLongLoad(Opcode::local_long_load32_i32),
    );
    add(
        "local.long_load32_i16_s",
        InstructionKind::LocalLongLoad(Opcode::local_long_load32_i16_s),
    );
    add(
        "local.long_load32_i16_u",
        InstructionKind::LocalLongLoad(Opcode::local_long_load32_i16_u),
    );
    add(
        "local.long_load32_i8_s",
        InstructionKind::LocalLongLoad(Opcode::local_long_load32_i8_s),
    );
    add(
        "local.long_load32_i8_u",
        InstructionKind::LocalLongLoad(Opcode::local_long_load32_i8_u),
    );
    add(
        "local.long_load32_f32",
        InstructionKind::LocalLongLoad(Opcode::local_long_load32_f32),
    );
    add(
        "local.long_store64",
        InstructionKind::LocalLongStore(Opcode::local_long_store64),
    );
    add(
        "local.long_store32",
        InstructionKind::LocalLongStore(Opcode::local_long_store32),
    );
    add(
        "local.long_store16",
        InstructionKind::LocalLongStore(Opcode::local_long_store16),
    );
    add(
        "local.long_store8",
        InstructionKind::LocalLongStore(Opcode::local_long_store8),
    );

    // data
    add(
        "data.load64_i64",
        InstructionKind::DataLoad(Opcode::data_load64_i64),
    );
    add(
        "data.load64_f64",
        InstructionKind::DataLoad(Opcode::data_load64_f64),
    );
    add(
        "data.load32_i32",
        InstructionKind::DataLoad(Opcode::data_load32_i32),
    );
    add(
        "data.load32_i16_s",
        InstructionKind::DataLoad(Opcode::data_load32_i16_s),
    );
    add(
        "data.load32_i16_u",
        InstructionKind::DataLoad(Opcode::data_load32_i16_u),
    );
    add(
        "data.load32_i8_s",
        InstructionKind::DataLoad(Opcode::data_load32_i8_s),
    );
    add(
        "data.load32_i8_u",
        InstructionKind::DataLoad(Opcode::data_load32_i8_u),
    );
    add(
        "data.load32_f32",
        InstructionKind::DataLoad(Opcode::data_load32_f32),
    );
    add(
        "data.store64",
        InstructionKind::DataStore(Opcode::data_store64),
    );
    add(
        "data.store32",
        InstructionKind::DataStore(Opcode::data_store32),
    );
    add(
        "data.store16",
        InstructionKind::DataStore(Opcode::data_store16),
    );
    add(
        "data.store8",
        InstructionKind::DataStore(Opcode::data_store8),
    );

    add(
        "data.long_load64_i64",
        InstructionKind::DataLongLoad(Opcode::data_long_load64_i64),
    );
    add(
        "data.long_load64_f64",
        InstructionKind::DataLongLoad(Opcode::data_long_load64_f64),
    );
    add(
        "data.long_load32_i32",
        InstructionKind::DataLongLoad(Opcode::data_long_load32_i32),
    );
    add(
        "data.long_load32_i16_s",
        InstructionKind::DataLongLoad(Opcode::data_long_load32_i16_s),
    );
    add(
        "data.long_load32_i16_u",
        InstructionKind::DataLongLoad(Opcode::data_long_load32_i16_u),
    );
    add(
        "data.long_load32_i8_s",
        InstructionKind::DataLongLoad(Opcode::data_long_load32_i8_s),
    );
    add(
        "data.long_load32_i8_u",
        InstructionKind::DataLongLoad(Opcode::data_long_load32_i8_u),
    );
    add(
        "data.long_load32_f32",
        InstructionKind::DataLongLoad(Opcode::data_long_load32_f32),
    );
    add(
        "data.long_store64",
        InstructionKind::DataLongStore(Opcode::data_long_store64),
    );
    add(
        "data.long_store32",
        InstructionKind::DataLongStore(Opcode::data_long_store32),
    );
    add(
        "data.long_store16",
        InstructionKind::DataLongStore(Opcode::data_long_store16),
    );
    add(
        "data.long_store8",
        InstructionKind::DataLongStore(Opcode::data_long_store8),
    );

    // heap
    add(
        "heap.load64_i64",
        InstructionKind::HeapLoad(Opcode::heap_load64_i64),
    );
    add(
        "heap.load64_f64",
        InstructionKind::HeapLoad(Opcode::heap_load64_f64),
    );
    add(
        "heap.load32_i32",
        InstructionKind::HeapLoad(Opcode::heap_load32_i32),
    );
    add(
        "heap.load32_i16_s",
        InstructionKind::HeapLoad(Opcode::heap_load32_i16_s),
    );
    add(
        "heap.load32_i16_u",
        InstructionKind::HeapLoad(Opcode::heap_load32_i16_u),
    );
    add(
        "heap.load32_i8_s",
        InstructionKind::HeapLoad(Opcode::heap_load32_i8_s),
    );
    add(
        "heap.load32_i8_u",
        InstructionKind::HeapLoad(Opcode::heap_load32_i8_u),
    );
    add(
        "heap.load32_f32",
        InstructionKind::HeapLoad(Opcode::heap_load32_f32),
    );
    add(
        "heap.store64",
        InstructionKind::HeapStore(Opcode::heap_store64),
    );
    add(
        "heap.store32",
        InstructionKind::HeapStore(Opcode::heap_store32),
    );
    add(
        "heap.store16",
        InstructionKind::HeapStore(Opcode::heap_store16),
    );
    add(
        "heap.store8",
        InstructionKind::HeapStore(Opcode::heap_store8),
    );

    add("heap.fill", InstructionKind::NoParams(Opcode::heap_fill, 3));
    add("heap.copy", InstructionKind::NoParams(Opcode::heap_copy, 3));
    add(
        "heap.capacity",
        InstructionKind::NoParams(Opcode::heap_capacity, 0),
    );
    add(
        "heap.resize",
        InstructionKind::NoParams(Opcode::heap_resize, 1),
    );

    // conversion
    add(
        "i32.truncate_i64",
        InstructionKind::UnaryOp(Opcode::i32_truncate_i64),
    );
    add(
        "i64.extend_i32_s",
        InstructionKind::UnaryOp(Opcode::i64_extend_i32_s),
    );
    add(
        "i64.extend_i32_u",
        InstructionKind::UnaryOp(Opcode::i64_extend_i32_u),
    );
    add(
        "f32.demote_f64",
        InstructionKind::UnaryOp(Opcode::f32_demote_f64),
    );
    add(
        "f64.promote_f32",
        InstructionKind::UnaryOp(Opcode::f64_promote_f32),
    );

    add(
        "i32.convert_f32_s",
        InstructionKind::UnaryOp(Opcode::i32_convert_f32_s),
    );
    add(
        "i32.convert_f32_u",
        InstructionKind::UnaryOp(Opcode::i32_convert_f32_u),
    );
    add(
        "i32.convert_f64_s",
        InstructionKind::UnaryOp(Opcode::i32_convert_f64_s),
    );
    add(
        "i32.convert_f64_u",
        InstructionKind::UnaryOp(Opcode::i32_convert_f64_u),
    );
    add(
        "i64.convert_f32_s",
        InstructionKind::UnaryOp(Opcode::i64_convert_f32_s),
    );
    add(
        "i64.convert_f32_u",
        InstructionKind::UnaryOp(Opcode::i64_convert_f32_u),
    );
    add(
        "i64.convert_f64_s",
        InstructionKind::UnaryOp(Opcode::i64_convert_f64_s),
    );
    add(
        "i64.convert_f64_u",
        InstructionKind::UnaryOp(Opcode::i64_convert_f64_u),
    );

    add(
        "f32.convert_i32_s",
        InstructionKind::UnaryOp(Opcode::f32_convert_i32_s),
    );
    add(
        "f32.convert_i32_u",
        InstructionKind::UnaryOp(Opcode::f32_convert_i32_u),
    );
    add(
        "f32.convert_i64_s",
        InstructionKind::UnaryOp(Opcode::f32_convert_i64_s),
    );
    add(
        "f32.convert_i64_u",
        InstructionKind::UnaryOp(Opcode::f32_convert_i64_u),
    );
    add(
        "f64.convert_i32_s",
        InstructionKind::UnaryOp(Opcode::f64_convert_i32_s),
    );
    add(
        "f64.convert_i32_u",
        InstructionKind::UnaryOp(Opcode::f64_convert_i32_u),
    );
    add(
        "f64.convert_i64_s",
        InstructionKind::UnaryOp(Opcode::f64_convert_i64_s),
    );
    add(
        "f64.convert_i64_u",
        InstructionKind::UnaryOp(Opcode::f64_convert_i64_u),
    );

    // comparsion
    add("i32.eqz", InstructionKind::UnaryOp(Opcode::i32_eqz)); // UnaryOp
    add("i32.nez", InstructionKind::UnaryOp(Opcode::i32_nez)); // UnaryOp
    add("i32.eq", InstructionKind::BinaryOp(Opcode::i32_eq));
    add("i32.ne", InstructionKind::BinaryOp(Opcode::i32_ne));
    add("i32.lt_s", InstructionKind::BinaryOp(Opcode::i32_lt_s));
    add("i32.lt_u", InstructionKind::BinaryOp(Opcode::i32_lt_u));
    add("i32.gt_s", InstructionKind::BinaryOp(Opcode::i32_gt_s));
    add("i32.gt_u", InstructionKind::BinaryOp(Opcode::i32_gt_u));
    add("i32.le_s", InstructionKind::BinaryOp(Opcode::i32_le_s));
    add("i32.le_u", InstructionKind::BinaryOp(Opcode::i32_le_u));
    add("i32.ge_s", InstructionKind::BinaryOp(Opcode::i32_ge_s));
    add("i32.ge_u", InstructionKind::BinaryOp(Opcode::i32_ge_u));

    add("i64.eqz", InstructionKind::UnaryOp(Opcode::i64_eqz)); // UnaryOp
    add("i64.nez", InstructionKind::UnaryOp(Opcode::i64_nez)); // UnaryOp
    add("i64.eq", InstructionKind::BinaryOp(Opcode::i64_eq));
    add("i64.ne", InstructionKind::BinaryOp(Opcode::i64_ne));
    add("i64.lt_s", InstructionKind::BinaryOp(Opcode::i64_lt_s));
    add("i64.lt_u", InstructionKind::BinaryOp(Opcode::i64_lt_u));
    add("i64.gt_s", InstructionKind::BinaryOp(Opcode::i64_gt_s));
    add("i64.gt_u", InstructionKind::BinaryOp(Opcode::i64_gt_u));
    add("i64.le_s", InstructionKind::BinaryOp(Opcode::i64_le_s));
    add("i64.le_u", InstructionKind::BinaryOp(Opcode::i64_le_u));
    add("i64.ge_s", InstructionKind::BinaryOp(Opcode::i64_ge_s));
    add("i64.ge_u", InstructionKind::BinaryOp(Opcode::i64_ge_u));

    add("f32.eq", InstructionKind::BinaryOp(Opcode::f32_eq));
    add("f32.ne", InstructionKind::BinaryOp(Opcode::f32_ne));
    add("f32.lt", InstructionKind::BinaryOp(Opcode::f32_lt));
    add("f32.gt", InstructionKind::BinaryOp(Opcode::f32_gt));
    add("f32.le", InstructionKind::BinaryOp(Opcode::f32_le));
    add("f32.ge", InstructionKind::BinaryOp(Opcode::f32_ge));

    add("f64.eq", InstructionKind::BinaryOp(Opcode::f64_eq));
    add("f64.ne", InstructionKind::BinaryOp(Opcode::f64_ne));
    add("f64.lt", InstructionKind::BinaryOp(Opcode::f64_lt));
    add("f64.gt", InstructionKind::BinaryOp(Opcode::f64_gt));
    add("f64.le", InstructionKind::BinaryOp(Opcode::f64_le));
    add("f64.ge", InstructionKind::BinaryOp(Opcode::f64_ge));

    // arithmetic
    add("i32.add", InstructionKind::BinaryOp(Opcode::i32_add));
    add("i32.sub", InstructionKind::BinaryOp(Opcode::i32_sub));
    add("i32.mul", InstructionKind::BinaryOp(Opcode::i32_mul));
    add("i32.div_s", InstructionKind::BinaryOp(Opcode::i32_div_s));
    add("i32.div_u", InstructionKind::BinaryOp(Opcode::i32_div_u));
    add("i32.rem_s", InstructionKind::BinaryOp(Opcode::i32_rem_s));
    add("i32.rem_u", InstructionKind::BinaryOp(Opcode::i32_rem_u));
    add("i32.inc", InstructionKind::UnaryOpParamI16(Opcode::i32_inc)); // UnaryOpParamI16
    add("i32.dec", InstructionKind::UnaryOpParamI16(Opcode::i32_dec)); // UnaryOpParamI16

    add("i64.add", InstructionKind::BinaryOp(Opcode::i64_add));
    add("i64.sub", InstructionKind::BinaryOp(Opcode::i64_sub));
    add("i64.mul", InstructionKind::BinaryOp(Opcode::i64_mul));
    add("i64.div_s", InstructionKind::BinaryOp(Opcode::i64_div_s));
    add("i64.div_u", InstructionKind::BinaryOp(Opcode::i64_div_u));
    add("i64.rem_s", InstructionKind::BinaryOp(Opcode::i64_rem_s));
    add("i64.rem_u", InstructionKind::BinaryOp(Opcode::i64_rem_u));
    add("i64.inc", InstructionKind::UnaryOpParamI16(Opcode::i64_inc)); // UnaryOpParamI16
    add("i64.dec", InstructionKind::UnaryOpParamI16(Opcode::i64_dec)); // UnaryOpParamI16

    add("f32.add", InstructionKind::BinaryOp(Opcode::f32_add));
    add("f32.sub", InstructionKind::BinaryOp(Opcode::f32_sub));
    add("f32.mul", InstructionKind::BinaryOp(Opcode::f32_mul));
    add("f32.div", InstructionKind::BinaryOp(Opcode::f32_div));

    add("f64.add", InstructionKind::BinaryOp(Opcode::f64_add));
    add("f64.sub", InstructionKind::BinaryOp(Opcode::f64_sub));
    add("f64.mul", InstructionKind::BinaryOp(Opcode::f64_mul));
    add("f64.div", InstructionKind::BinaryOp(Opcode::f64_div));

    // bitwise
    add("i32.and", InstructionKind::BinaryOp(Opcode::i32_and));
    add("i32.or", InstructionKind::BinaryOp(Opcode::i32_or));
    add("i32.xor", InstructionKind::BinaryOp(Opcode::i32_xor));
    add(
        "i32.shift_left",
        InstructionKind::BinaryOp(Opcode::i32_shift_left),
    );
    add(
        "i32.shift_right_s",
        InstructionKind::BinaryOp(Opcode::i32_shift_right_s),
    );
    add(
        "i32.shift_right_u",
        InstructionKind::BinaryOp(Opcode::i32_shift_right_u),
    );
    add(
        "i32.rotate_left",
        InstructionKind::BinaryOp(Opcode::i32_rotate_left),
    );
    add(
        "i32.rotate_right",
        InstructionKind::BinaryOp(Opcode::i32_rotate_right),
    );
    add("i32.not", InstructionKind::UnaryOp(Opcode::i32_not)); // UnaryOp
    add(
        "i32.leading_zeros",
        InstructionKind::UnaryOp(Opcode::i32_leading_zeros),
    ); // UnaryOp
    add(
        "i32.trailing_zeros",
        InstructionKind::UnaryOp(Opcode::i32_trailing_zeros),
    ); // UnaryOp
    add(
        "i32.count_ones",
        InstructionKind::UnaryOp(Opcode::i32_count_ones),
    ); // UnaryOp

    add("i64.and", InstructionKind::BinaryOp(Opcode::i64_and));
    add("i64.or", InstructionKind::BinaryOp(Opcode::i64_or));
    add("i64.xor", InstructionKind::BinaryOp(Opcode::i64_xor));
    add(
        "i64.shift_left",
        InstructionKind::BinaryOp(Opcode::i64_shift_left),
    );
    add(
        "i64.shift_right_s",
        InstructionKind::BinaryOp(Opcode::i64_shift_right_s),
    );
    add(
        "i64.shift_right_u",
        InstructionKind::BinaryOp(Opcode::i64_shift_right_u),
    );
    add(
        "i64.rotate_left",
        InstructionKind::BinaryOp(Opcode::i64_rotate_left),
    );
    add(
        "i64.rotate_right",
        InstructionKind::BinaryOp(Opcode::i64_rotate_right),
    );
    add("i64.not", InstructionKind::UnaryOp(Opcode::i64_not)); // UnaryOp
    add(
        "i64.leading_zeros",
        InstructionKind::UnaryOp(Opcode::i64_leading_zeros),
    ); // UnaryOp
    add(
        "i64.trailing_zeros",
        InstructionKind::UnaryOp(Opcode::i64_trailing_zeros),
    ); // UnaryOp
    add(
        "i64.count_ones",
        InstructionKind::UnaryOp(Opcode::i64_count_ones),
    ); // UnaryOp

    // math
    add("f32.abs", InstructionKind::UnaryOp(Opcode::f32_abs));
    add("f32.neg", InstructionKind::UnaryOp(Opcode::f32_neg));
    add("f32.ceil", InstructionKind::UnaryOp(Opcode::f32_ceil));
    add("f32.floor", InstructionKind::UnaryOp(Opcode::f32_floor));
    add(
        "f32.round_half_away_from_zero",
        InstructionKind::UnaryOp(Opcode::f32_round_half_away_from_zero),
    );
    add("f32.trunc", InstructionKind::UnaryOp(Opcode::f32_trunc));
    add("f32.fract", InstructionKind::UnaryOp(Opcode::f32_fract));
    add("f32.sqrt", InstructionKind::UnaryOp(Opcode::f32_sqrt));
    add("f32.cbrt", InstructionKind::UnaryOp(Opcode::f32_cbrt));
    add("f32.exp", InstructionKind::UnaryOp(Opcode::f32_exp));
    add("f32.exp2", InstructionKind::UnaryOp(Opcode::f32_exp2));
    add("f32.ln", InstructionKind::UnaryOp(Opcode::f32_ln));
    add("f32.log2", InstructionKind::UnaryOp(Opcode::f32_log2));
    add("f32.log10", InstructionKind::UnaryOp(Opcode::f32_log10));
    add("f32.sin", InstructionKind::UnaryOp(Opcode::f32_sin));
    add("f32.cos", InstructionKind::UnaryOp(Opcode::f32_cos));
    add("f32.tan", InstructionKind::UnaryOp(Opcode::f32_tan));
    add("f32.asin", InstructionKind::UnaryOp(Opcode::f32_asin));
    add("f32.acos", InstructionKind::UnaryOp(Opcode::f32_acos));
    add("f32.atan", InstructionKind::UnaryOp(Opcode::f32_atan));
    add("f32.pow", InstructionKind::BinaryOp(Opcode::f32_pow)); // BinaryOp
    add("f32.log", InstructionKind::BinaryOp(Opcode::f32_log)); // BinaryOp

    add("f64.abs", InstructionKind::UnaryOp(Opcode::f64_abs));
    add("f64.neg", InstructionKind::UnaryOp(Opcode::f64_neg));
    add("f64.ceil", InstructionKind::UnaryOp(Opcode::f64_ceil));
    add("f64.floor", InstructionKind::UnaryOp(Opcode::f64_floor));
    add(
        "f64.round_half_away_from_zero",
        InstructionKind::UnaryOp(Opcode::f64_round_half_away_from_zero),
    );
    add("f64.trunc", InstructionKind::UnaryOp(Opcode::f64_trunc));
    add("f64.fract", InstructionKind::UnaryOp(Opcode::f64_fract));
    add("f64.sqrt", InstructionKind::UnaryOp(Opcode::f64_sqrt));
    add("f64.cbrt", InstructionKind::UnaryOp(Opcode::f64_cbrt));
    add("f64.exp", InstructionKind::UnaryOp(Opcode::f64_exp));
    add("f64.exp2", InstructionKind::UnaryOp(Opcode::f64_exp2));
    add("f64.ln", InstructionKind::UnaryOp(Opcode::f64_ln));
    add("f64.log2", InstructionKind::UnaryOp(Opcode::f64_log2));
    add("f64.log10", InstructionKind::UnaryOp(Opcode::f64_log10));
    add("f64.sin", InstructionKind::UnaryOp(Opcode::f64_sin));
    add("f64.cos", InstructionKind::UnaryOp(Opcode::f64_cos));
    add("f64.tan", InstructionKind::UnaryOp(Opcode::f64_tan));
    add("f64.asin", InstructionKind::UnaryOp(Opcode::f64_asin));
    add("f64.acos", InstructionKind::UnaryOp(Opcode::f64_acos));
    add("f64.atan", InstructionKind::UnaryOp(Opcode::f64_atan));
    add("f64.pow", InstructionKind::BinaryOp(Opcode::f64_pow)); // BinaryOp
    add("f64.log", InstructionKind::BinaryOp(Opcode::f64_log)); // BinaryOp

    // control flow
    // note: all instructions in this catalog are replaced with pesudo instructions

    // function call
    // note: all instructions in this catalog are replaced with pesudo instructions

    // host
    add("panic", InstructionKind::NoParams(Opcode::panic, 0));
    add("unreachable", InstructionKind::Unreachable);
    add("debug", InstructionKind::Debug);

    add(
        "host.addr_local",
        InstructionKind::LocalLoad(Opcode::host_addr_local),
    );
    add(
        "host.addr_local_long",
        InstructionKind::LocalLongLoad(Opcode::host_addr_local_long),
    );
    add(
        "host.addr_data",
        InstructionKind::DataLoad(Opcode::host_addr_data),
    );
    add(
        "host.addr_data_long",
        InstructionKind::DataLongLoad(Opcode::host_addr_data_long),
    );
    add(
        "host.addr_heap",
        InstructionKind::HeapLoad(Opcode::host_addr_heap),
    );
    add("host.addr_function", InstructionKind::HostAddrFunction);
    add(
        "host.copy_from_heap",
        InstructionKind::NoParams(Opcode::host_copy_from_heap, 3),
    );

    add(
        "host.copy_to_heap",
        InstructionKind::NoParams(Opcode::host_copy_to_heap, 3),
    );

    // pesudo instructions
    add("i32.imm", InstructionKind::ImmI32);
    add("i64.imm", InstructionKind::ImmI64);
    add("f32.imm", InstructionKind::ImmF32);
    add("f64.imm", InstructionKind::ImmF64);

    add("when", InstructionKind::When);
    add("if", InstructionKind::If);
    add("branch", InstructionKind::Branch);
    add("for", InstructionKind::For);

    add("do", InstructionKind::Sequence("do"));
    add("break", InstructionKind::Sequence("break"));
    add("return", InstructionKind::Sequence("return"));
    add("recur", InstructionKind::Sequence("recur"));
    add("rerun", InstructionKind::Sequence("rerun"));

    add("call", InstructionKind::Call);
    add("dyncall", InstructionKind::DynCall);
    add("envcall", InstructionKind::EnvCall);
    add("syscall", InstructionKind::SysCall);
    add("extcall", InstructionKind::ExtCall);

    // macros
    add(
        "macro.get_function_public_index",
        InstructionKind::MacroGetFunctionPublicIndex,
    );

    unsafe { INSTRUCTION_KIND_TABLE = Some(table) };
}
