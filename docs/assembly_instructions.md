# XiaoXuan Core Assembly Instructions

<!-- @import "[TOC]" {cmd="toc" depthFrom=1 depthTo=6 orderedList=false} -->

<!-- code_chunk_output -->

- [XiaoXuan Core Assembly Instructions](#xiaoxuan-core-assembly-instructions)
  - [Fundamnetal](#fundamnetal)
  - [Local variable](#local-variable)
    - [local variable loading and storing](#local-variable-loading-and-storing)
    - [local variable loading and storing with dynamical offset](#local-variable-loading-and-storing-with-dynamical-offset)
  - [Data](#data)
    - [data loading and storing](#data-loading-and-storing)
    - [data loading and storing with dynamical offset](#data-loading-and-storing-with-dynamical-offset)
  - [Heap](#heap)
    - [heap loading and storing](#heap-loading-and-storing)
    - [heap memory](#heap-memory)
  - [Conversion](#conversion)
    - [extend and truncate](#extend-and-truncate)
    - [demote and promote](#demote-and-promote)
    - [floating point to integer](#floating-point-to-integer)
    - [integer to floating point](#integer-to-floating-point)
  - [Comparison](#comparison)
    - [i32](#i32)
    - [i64](#i64)
    - [f32](#f32)
    - [f64](#f64)
  - [Arithmetic](#arithmetic)
    - [i32](#i32-1)
    - [i64](#i64-1)
    - [f32](#f32-1)
    - [f64](#f64-1)
  - [Bitwise](#bitwise)
    - [i32](#i32-2)
    - [i64](#i64-2)
  - [Math](#math)
    - [f32](#f32-2)
    - [f64](#f64-2)
  - [Function Calling](#function-calling)
  - [Host](#host)

<!-- /code_chunk_output -->

## Fundamnetal

zero
(drop VALUE)
(duplicate VALUE)
(swap LEFT RIGHT)

(select_nez
    VALUE_WHEN_TRUE
    VALUE_WHEN_FALSE
    VALUE_FOR_TEST
)

(select_nez
    (i32.imm 11)    ;; when true
    (i32.imm 13)    ;; when false
    (i32.imm 1)     ;; test
) -> 11

immediate number

(i32.imm INTEGER_NUMBER)
(i64.imm INTEGER_NUMBER)
(f32.imm FLOATING_POINT_NUMBER)
(f64.imm FLOATING_POINT_NUMBER)

INTEGER_NUMBER can be decimal, dexadecimal and binary liternals.
FLOATING_POINT_NUMBER can be decimal and dexadecimal liternals.

## Local variable

### local variable loading and storing

(local.load64_i64 $VARIABLE_NAME OPTIONAL_OFFSET:i16)

local loading instruction variants:

- `local.load64_i64`
- `local.load64_f64`
- `local.load32_i32`
- `local.load32_i16_s`
- `local.load32_i16_u`
- `local.load32_i8_s`
- `local.load32_i8_u`
- `local.load32_f32`

local storing

(local.store64 $VARIABLE_NAME OPTIONAL_OFFSET:i16 VALUE)

variants:

- `local.store64`
- `local.store32`
- `local.store16`
- `local.store8`

### local variable loading and storing with dynamical offset

(local.long_load64_i64 $VARIABLE_NAME OFFSET_I32)

variants:

- `local.long_load64_i64`
- `local.long_load64_f64`
- `local.long_load32_i32`
- `local.long_load32_i16_s`
- `local.long_load32_i16_u`
- `local.long_load32_i8_s`
- `local.long_load32_i8_u`
- `local.long_load32_f32`

storing

(local.long_store64 $VARIABLE_NAME OFFSET_I32 VALUE)

- `local.long_store64`
- `local.long_store32`
- `local.long_store16`
- `local.long_store8`

## Data

### data loading and storing

(data.load64_i64 $DATA_NAME_PATH OPTIONAL_OFFSET:i16)

variants

- `data.load64_i64`
- `data.load64_f64`
- `data.load32_i32`
- `data.load32_i16_s`
- `data.load32_i16_u`
- `data.load32_i8_s`
- `data.load32_i8_u`
- `data.load32_f32`

> NOTE:
> the data name should contains the full namespace path, e.g. `mylib::msg`, `mylib::utils::buf`.
> The namespace path can also be omitted, in which case the instruction will access items within the current module.

storing

(data.store64 $DATA_NAME_PATH OPTIONAL_OFFSET:i16 VALUE)

- `data.store64`
- `data.store32`
- `data.store16`
- `data.store8`

### data loading and storing with dynamical offset

(data.long_load64_i64 $DATA_NAME_PATH OFFSET_I32)

variants:

- `data.long_load64_i64`
- `data.long_load64_f64`
- `data.long_load32_i32`
- `data.long_load32_i16_s`
- `data.long_load32_i16_u`
- `data.long_load32_i8_s`
- `data.long_load32_i8_u`
- `data.long_load32_f32`

storing

(data.long_store64 $DATA_NAME_PATH OFFSET_I32 VALUE)

variants:

- `data.long_store64`
- `data.long_store32`
- `data.long_store16`
- `data.long_store8`

## Heap

### heap loading and storing

(heap.load64_i64 OPTIONAL_OFFSET:i16 ADDR)

variants:

- `heap.load64_i64`
- `heap.load64_f64`
- `heap.load32_i32`
- `heap.load32_i16_s`
- `heap.load32_i16_u`
- `heap.load32_i8_s`
- `heap.load32_i8_u`
- `heap.load32_f32`

storing

(heap.store64 OPTIONAL_OFFSET:i16 ADDR VALUE)

variants:

- `heap.store64`
- `heap.store32`
- `heap.store16`
- `heap.store8`

### heap memory

(heap.fill
    ADDR_I64
    VALUE_I8
    LENGTH_I64)

(heap.copy
    DST_ADDR_I64
    SRC_ADDR_I64
    LENGTH_I64)

heap.capacity

(heap.resize PAGES)

## Conversion

### extend and truncate

(i64.extend_i32_s VALUE_I32)
(i64.extend_i32_u VALUE_I32)
(i32.truncate_i64 VALUE_I64)

### demote and promote

(f64.promote_f32 VALUE_F32)
(f32.demote_f64 VALUE_I64)

### floating point to integer

(i32.convert_f32_s VALUE)
(i32.convert_f32_u VALUE)
(i32.convert_f64_s VALUE)
(i32.convert_f64_u VALUE)

(i64.convert_f32_s VALUE)
(i64.convert_f32_u VALUE)
(i64.convert_f64_s VALUE)
(i64.convert_f64_u VALUE)

### integer to floating point

(f32.convert_i32_s VALUE)
(f32.convert_i32_u VALUE)
(f32.convert_i64_s VALUE)
(f32.convert_i64_u VALUE)

(f64.convert_i64_s VALUE)
(f64.convert_i64_u VALUE)
(f64.convert_i32_s VALUE)
(f64.convert_i32_u VALUE)

## Comparison

### i32

(i32.eqz VALUE)
(i32.nez VALUE)

(i32.eq LEFT RIGHT)
(i32.ne LEFT RIGHT)
(i32.lt_s LEFT RIGHT)
(i32.lt_u LEFT RIGHT)
(i32.gt_s LEFT RIGHT)
(i32.gt_u LEFT RIGHT)
(i32.le_s LEFT RIGHT)
(i32.le_u LEFT RIGHT)
(i32.ge_s LEFT RIGHT)
(i32.ge_u LEFT RIGHT)

### i64

(i64.eqz VALUE)
(i64.nez VALUE)

(i64.eq LEFT RIGHT)
(i64.ne LEFT RIGHT)
(i64.lt_s LEFT RIGHT)
(i64.lt_u LEFT RIGHT)
(i64.gt_s LEFT RIGHT)
(i64.gt_u LEFT RIGHT)
(i64.le_s LEFT RIGHT)
(i64.le_u LEFT RIGHT)
(i64.ge_s LEFT RIGHT)
(i64.ge_u LEFT RIGHT)

### f32

(f32.eq LEFT RIGHT)
(f32.ne LEFT RIGHT)
(f32.lt LEFT RIGHT)
(f32.gt LEFT RIGHT)
(f32.le LEFT RIGHT)
(f32.ge LEFT RIGHT)

### f64

(f64.eq LEFT RIGHT)
(f64.ne LEFT RIGHT)
(f64.lt LEFT RIGHT)
(f64.gt LEFT RIGHT)
(f64.le LEFT RIGHT)
(f64.ge LEFT RIGHT)

## Arithmetic

### i32
(i32.add LEFT RIGHT)
wrapping add, e.g. 0xffff_ffff + 2 = 1 (-1 + 2 = 1)

(i32.sub LEFT RIGHT)
wrapping sub, e.g. 11 - 211 = -200

(i32.mul LEFT RIGHT)
wrapping mul, e.g. 0xf0e0d0c0 * 2 = 0xf0e0d0c0 << 1

(i32.div_s LEFT RIGHT)
(i32.div_u LEFT RIGHT)
(i32.rem_s LEFT RIGHT)
(i32.rem_u LEFT RIGHT)

(i32.inc IMM:i16 VALUE)
wrapping inc, e.g. 0xffff_ffff inc 2 = 1

(i32.dec IMM:i16 VALUE)
wrapping dec, e.g. 0x1 dec 2 = 0xffff_ffff

### i64

(i64.add LEFT RIGHT)
(i64.sub LEFT RIGHT)
(i64.mul LEFT RIGHT)
(i64.div_s LEFT RIGHT)
(i64.div_u LEFT RIGHT)
(i64.rem_s LEFT RIGHT)
(i64.rem_u LEFT RIGHT)

(i64.inc IMM:i16 VALUE)
(i64.dec IMM:i16 VALUE)

### f32

(f32.add LEFT RIGHT)
(f32.sub LEFT RIGHT)
(f32.mul LEFT RIGHT)
(f32.div LEFT RIGHT)

### f64

(f64.add LEFT RIGHT)
(f64.sub LEFT RIGHT)
(f64.mul LEFT RIGHT)
(f64.div LEFT RIGHT)

## Bitwise

### i32

(i32.and LEFT RIGHT)
(i32.or LEFT RIGHT)
(i32.xor LEFT RIGHT)
(i32.shift_left LEFT RIGHT)
(i32.shift_right_s LEFT RIGHT)
(i32.shift_right_u LEFT RIGHT)
(i32.rotate_left LEFT RIGHT)
(i32.rotate_right LEFT RIGHT)

(i32.not VALUE)
(i32.leading_zeros VALUE)
(i32.trailing_zeros VALUE)
(i32.count_ones VALUE)

### i64

(i64.and LEFT RIGHT)
(i64.or LEFT RIGHT)
(i64.xor LEFT RIGHT)
(i64.shift_left LEFT RIGHT)
(i64.shift_right_s LEFT RIGHT)
(i64.shift_right_u LEFT RIGHT)
(i64.rotate_left LEFT RIGHT)
(i64.rotate_right LEFT RIGHT)

(i64.not VALUE)
(i64.leading_zeros VALUE)
(i64.trailing_zeros VALUE)
(i64.count_ones VALUE)

## Math

### f32

(f32.abs VALUE)
(f32.neg VALUE)
(f32.ceil VALUE)
(f32.floor VALUE)
(f32.round_half_away_from_zero VALUE)
(f32.trunc VALUE)
(f32.fract VALUE)
(f32.sqrt VALUE)
(f32.cbrt VALUE)
(f32.exp VALUE)
(f32.exp2 VALUE)
(f32.ln VALUE)
(f32.log2 VALUE)
(f32.log10 VALUE)
(f32.sin VALUE)
(f32.cos VALUE)
(f32.tan VALUE)
(f32.asin VALUE)
(f32.acos VALUE)
(f32.atan VALUE)
(f32.pow LEFT RIGHT)
(f32.log LEFT RIGHT)

### f64

(f64.abs VALUE)
(f64.neg VALUE)
(f64.ceil VALUE)
(f64.floor VALUE)
(f64.round_half_away_from_zero VALUE)
(f64.trunc VALUE)
(f64.fract VALUE)
(f64.sqrt VALUE)
(f64.cbrt VALUE)
(f64.exp VALUE)
(f64.exp2 VALUE)
(f64.ln VALUE)
(f64.log2 VALUE)
(f64.log10 VALUE)
(f64.sin VALUE)
(f64.cos VALUE)
(f64.tan VALUE)
(f64.asin VALUE)
(f64.acos VALUE)
(f64.atan VALUE)

(f64.pow LEFT RIGHT)
(f64.log LEFT RIGHT)

## Function Calling

(call $id ARG_0 ARG_1 ... ARG_N)
(dyncall FUNC_PUBLIC_INDEX_I32 ARG_0 ARG_1 ... ARG_N)
(envcall ENV_CALL_NUMBER:i32 ARG_0 ARG_1 ... ARG_N)
(syscall SYS_CALL_NUMBER:i32 ARG_0 ARG_1 ... ARG_N)
(extcall $id ARG_0 ARG_1 ... ARG_N)

> NOTE:
> the `id` in the instructions `call`, data loading and data storing can be a full name path (a path that combined with the path of namespace and the identifier), e.g. `mylib::msg`, `mylib::utils::buf`.
> When the name path is omitted, the instruction will access items within the current module.
> These name path can be a relative path, e.g. `module::utils::buf`, `self::utils::buf`.

## Host

(nop)
(panic)
(unreachable CODE:i32)
(debug CODE:i32)

(host.addr_local $VARIABLE_NAME OPTIONAL_OFFSET:i16)
(host.addr_local_long $VARIABLE_NAME OFFSET_I32)
(host.addr_data $DATA_NAME_PATH OPTIONAL_OFFSET:i16)
(host.addr_data_long $DATA_NAME_PATH OFFSET_I32)
(host.addr_heap OPTIONAL_OFFSET:i16 ADDR)

(host.addr_function $name)

(host.copy_from_heap
    DST_POINTER_I64
    SRC_OFFSET_I64
    LENGTH_IN_BYTES_I64)

(host.copy_to_heap
    DST_OFFSET_I64
    SRC_POINTER_I64
    LENGTH_IN_BYTES_I64)