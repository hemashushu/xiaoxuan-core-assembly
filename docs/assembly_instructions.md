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
(drop OPERAND)
(duplicate OPERAND)
(swap OPERAND_LEFT OPERAND_RIGHT)

(select_nez
    OPERAND_WHEN_TRUE
    OPERAND_WHEN_FALSE
    OPERAND_FOR_TEST
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

both INTEGER_NUMBER and FLOATING_POINT_NUMBER can be decimal, dexadecimal, binary.

## Local variable

### local variable loading and storing

(local.load64_i64 $VARIABLE_NAME OPTIONAL_OFFSET_NUMBER:i16)

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

(local.store64 $VARIABLE_NAME OPTIONAL_OFFSET_NUMBER:i16 OPERAND_FOR_STORING)

variants:

- `local.store64`
- `local.store32`
- `local.store16`
- `local.store8`

### local variable loading and storing with dynamical offset

(local.long_load64_i64 $VARIABLE_NAME OPERAND_FOR_OFFSET:i32)

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

(local.long_store64 $VARIABLE_NAME OPERAND_FOR_OFFSET:i32 OPERAND_FOR_STORING)

- `local.long_store64`
- `local.long_store32`
- `local.long_store16`
- `local.long_store8`

## Data

### data loading and storing

(data.load64_i64 $DATA_NAME_PATH OPTIONAL_OFFSET_NUMBER:i16)

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

(data.store64 $DATA_NAME_PATH OPTIONAL_OFFSET_NUMBER:i16 OPERAND_FOR_STORING)

- `data.store64`
- `data.store32`
- `data.store16`
- `data.store8`

### data loading and storing with dynamical offset

(data.long_load64_i64 $DATA_NAME_PATH OPERAND_FOR_OFFSET:i32)

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

(data.long_store64 $DATA_NAME_PATH OPERAND_FOR_OFFSET:i32 OPERAND_FOR_STORING)

variants:

- `data.long_store64`
- `data.long_store32`
- `data.long_store16`
- `data.long_store8`

## Heap

### heap loading and storing

(heap.load64_i64 OPTIONAL_OFFSET_NUMBER:i16 OPERAND_FOR_ADDR)

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

(heap.store64 OPTIONAL_OFFSET_NUMBER:i16 OPERAND_FOR_ADDR OPERAND_FOR_STORING)

variants:

- `heap.store64`
- `heap.store32`
- `heap.store16`
- `heap.store8`

### heap memory

(heap.fill
    OPERAND_FOR_ADDR:i64
    OPERAND_FOR_VALUE:i8
    OPERAND_FOR_LENGTH:i64)

(heap.copy
    OPERAND_FOR_DST_ADDR:i64
    OPERAND_FOR_SRC_ADDR:i64
    OPERAND_FOR_LENGTH:i64)

heap.capacity

(heap.resize OPERAND_FOR_PAGES)

## Conversion

### extend and truncate

(i64.extend_i32_s OPERAND:i32)
(i64.extend_i32_u OPERAND:i32)
(i32.truncate_i64 OPERAND:i64)

### demote and promote

(f64.promote_f32 OPERAND:f32)
(f32.demote_f64 OPERAND:f64)

### floating point to integer

(i32.convert_f32_s OPERAND)
(i32.convert_f32_u OPERAND)
(i32.convert_f64_s OPERAND)
(i32.convert_f64_u OPERAND)

(i64.convert_f32_s OPERAND)
(i64.convert_f32_u OPERAND)
(i64.convert_f64_s OPERAND)
(i64.convert_f64_u OPERAND)

### integer to floating point

(f32.convert_i32_s OPERAND)
(f32.convert_i32_u OPERAND)
(f32.convert_i64_s OPERAND)
(f32.convert_i64_u OPERAND)

(f64.convert_i64_s OPERAND)
(f64.convert_i64_u OPERAND)
(f64.convert_i32_s OPERAND)
(f64.convert_i32_u OPERAND)

## Comparison

### i32

(i32.eqz OPERAND)
(i32.nez OPERAND)

(i32.eq OPERAND_LEFT OPERAND_RIGHT)
(i32.ne OPERAND_LEFT OPERAND_RIGHT)
(i32.lt_s OPERAND_LEFT OPERAND_RIGHT)
(i32.lt_u OPERAND_LEFT OPERAND_RIGHT)
(i32.gt_s OPERAND_LEFT OPERAND_RIGHT)
(i32.gt_u OPERAND_LEFT OPERAND_RIGHT)
(i32.le_s OPERAND_LEFT OPERAND_RIGHT)
(i32.le_u OPERAND_LEFT OPERAND_RIGHT)
(i32.ge_s OPERAND_LEFT OPERAND_RIGHT)
(i32.ge_u OPERAND_LEFT OPERAND_RIGHT)

### i64

(i64.eqz OPERAND)
(i64.nez OPERAND)

(i64.eq OPERAND_LEFT OPERAND_RIGHT)
(i64.ne OPERAND_LEFT OPERAND_RIGHT)
(i64.lt_s OPERAND_LEFT OPERAND_RIGHT)
(i64.lt_u OPERAND_LEFT OPERAND_RIGHT)
(i64.gt_s OPERAND_LEFT OPERAND_RIGHT)
(i64.gt_u OPERAND_LEFT OPERAND_RIGHT)
(i64.le_s OPERAND_LEFT OPERAND_RIGHT)
(i64.le_u OPERAND_LEFT OPERAND_RIGHT)
(i64.ge_s OPERAND_LEFT OPERAND_RIGHT)
(i64.ge_u OPERAND_LEFT OPERAND_RIGHT)

### f32

(f32.eq OPERAND_LEFT OPERAND_RIGHT)
(f32.ne OPERAND_LEFT OPERAND_RIGHT)
(f32.lt OPERAND_LEFT OPERAND_RIGHT)
(f32.gt OPERAND_LEFT OPERAND_RIGHT)
(f32.le OPERAND_LEFT OPERAND_RIGHT)
(f32.ge OPERAND_LEFT OPERAND_RIGHT)

### f64

(f64.eq OPERAND_LEFT OPERAND_RIGHT)
(f64.ne OPERAND_LEFT OPERAND_RIGHT)
(f64.lt OPERAND_LEFT OPERAND_RIGHT)
(f64.gt OPERAND_LEFT OPERAND_RIGHT)
(f64.le OPERAND_LEFT OPERAND_RIGHT)
(f64.ge OPERAND_LEFT OPERAND_RIGHT)

## Arithmetic

### i32
(i32.add OPERAND_LEFT OPERAND_RIGHT)
wrapping add, e.g. 0xffff_ffff + 2 = 1 (-1 + 2 = 1)

(i32.sub OPERAND_LEFT OPERAND_RIGHT)
wrapping sub, e.g. 11 - 211 = -200

(i32.mul OPERAND_LEFT OPERAND_RIGHT)
wrapping mul, e.g. 0xf0e0d0c0 * 2 = 0xf0e0d0c0 << 1

(i32.div_s OPERAND_LEFT OPERAND_RIGHT)
(i32.div_u OPERAND_LEFT OPERAND_RIGHT)
(i32.rem_s OPERAND_LEFT OPERAND_RIGHT)
(i32.rem_u OPERAND_LEFT OPERAND_RIGHT)

(i32.inc AMOUNT_NUMBER:i16 OPERAND)
wrapping inc, e.g. 0xffff_ffff inc 2 = 1

(i32.dec AMOUNT_NUMBER:i16 OPERAND)
wrapping dec, e.g. 0x1 dec 2 = 0xffff_ffff

### i64

(i64.add OPERAND_LEFT OPERAND_RIGHT)
(i64.sub OPERAND_LEFT OPERAND_RIGHT)
(i64.mul OPERAND_LEFT OPERAND_RIGHT)
(i64.div_s OPERAND_LEFT OPERAND_RIGHT)
(i64.div_u OPERAND_LEFT OPERAND_RIGHT)
(i64.rem_s OPERAND_LEFT OPERAND_RIGHT)
(i64.rem_u OPERAND_LEFT OPERAND_RIGHT)

(i64.inc AMOUNT_NUMBER:i16 OPERAND)
(i64.dec AMOUNT_NUMBER:i16 OPERAND)

### f32

(f32.add OPERAND_LEFT OPERAND_RIGHT)
(f32.sub OPERAND_LEFT OPERAND_RIGHT)
(f32.mul OPERAND_LEFT OPERAND_RIGHT)
(f32.div OPERAND_LEFT OPERAND_RIGHT)

### f64

(f64.add OPERAND_LEFT OPERAND_RIGHT)
(f64.sub OPERAND_LEFT OPERAND_RIGHT)
(f64.mul OPERAND_LEFT OPERAND_RIGHT)
(f64.div OPERAND_LEFT OPERAND_RIGHT)

## Bitwise

### i32

(i32.and OPERAND_LEFT OPERAND_RIGHT)
(i32.or OPERAND_LEFT OPERAND_RIGHT)
(i32.xor OPERAND_LEFT OPERAND_RIGHT)
(i32.shift_left OPERAND_LEFT OPERAND_RIGHT)
(i32.shift_right_s OPERAND_LEFT OPERAND_RIGHT)
(i32.shift_right_u OPERAND_LEFT OPERAND_RIGHT)
(i32.rotate_left OPERAND_LEFT OPERAND_RIGHT)
(i32.rotate_right OPERAND_LEFT OPERAND_RIGHT)

(i32.not OPERAND)
(i32.leading_zeros OPERAND)
(i32.trailing_zeros OPERAND)
(i32.count_ones OPERAND)

### i64

(i64.and OPERAND_LEFT OPERAND_RIGHT)
(i64.or OPERAND_LEFT OPERAND_RIGHT)
(i64.xor OPERAND_LEFT OPERAND_RIGHT)
(i64.shift_left OPERAND_LEFT OPERAND_RIGHT)
(i64.shift_right_s OPERAND_LEFT OPERAND_RIGHT)
(i64.shift_right_u OPERAND_LEFT OPERAND_RIGHT)
(i64.rotate_left OPERAND_LEFT OPERAND_RIGHT)
(i64.rotate_right OPERAND_LEFT OPERAND_RIGHT)

(i64.not OPERAND)
(i64.leading_zeros OPERAND)
(i64.trailing_zeros OPERAND)
(i64.count_ones OPERAND)

## Math

### f32

(f32.abs OPERAND)
(f32.neg OPERAND)
(f32.ceil OPERAND)
(f32.floor OPERAND)
(f32.round_half_away_from_zero OPERAND)
(f32.trunc OPERAND)
(f32.fract OPERAND)
(f32.sqrt OPERAND)
(f32.cbrt OPERAND)
(f32.exp OPERAND)
(f32.exp2 OPERAND)
(f32.ln OPERAND)
(f32.log2 OPERAND)
(f32.log10 OPERAND)
(f32.sin OPERAND)
(f32.cos OPERAND)
(f32.tan OPERAND)
(f32.asin OPERAND)
(f32.acos OPERAND)
(f32.atan OPERAND)
(f32.pow OPERAND_LEFT OPERAND_RIGHT)
(f32.log OPERAND_LEFT OPERAND_RIGHT)

### f64

(f64.abs OPERAND)
(f64.neg OPERAND)
(f64.ceil OPERAND)
(f64.floor OPERAND)
(f64.round_half_away_from_zero OPERAND)
(f64.trunc OPERAND)
(f64.fract OPERAND)
(f64.sqrt OPERAND)
(f64.cbrt OPERAND)
(f64.exp OPERAND)
(f64.exp2 OPERAND)
(f64.ln OPERAND)
(f64.log2 OPERAND)
(f64.log10 OPERAND)
(f64.sin OPERAND)
(f64.cos OPERAND)
(f64.tan OPERAND)
(f64.asin OPERAND)
(f64.acos OPERAND)
(f64.atan OPERAND)

(f64.pow OPERAND_LEFT OPERAND_RIGHT)
(f64.log OPERAND_LEFT OPERAND_RIGHT)

## Function Calling

(call $name_path OPERAND_FOR_ARGS...)
(dyncall OPERAND_FOR_FUNC_PUBLIC_INDEX:i32 OPERAND_FOR_ARGS...)
(envcall ENV_CALL_NUMBER:i32 OPERAND_FOR_ARGS...)
(syscall SYS_CALL_NUMBER:i32 OPERAND_FOR_ARGS...)
(extcall $name OPERAND_FOR_ARGS...)

> NOTE:
> the `name_path` in the instructions `call`, data loading and data storing can be a full name path (a path that combined with the path of namespace and the identifier), e.g. `mylib::msg`, `mylib::utils::buf`.
> When the name path is omitted, the instruction will access items within the current module.
> These name path can be a relative path, e.g. `module::utils::buf`, `self::utils::buf`.

## Host

(nop)
(panic)
(unreachable CODE_NUMBER:i32)
(debug CODE_NUMBER:i32)
(host.addr_local $VARIABLE_NAME OPTIONAL_OFFSET_NUMBER:i16)
(host.addr_local_long $VARIABLE_NAME OPERAND_FOR_OFFSET:i32)
(host.addr_data $DATA_NAME_PATH OPTIONAL_OFFSET_NUMBER:i16)
(host.addr_data_long $DATA_NAME_PATH OPERAND_FOR_OFFSET:i32)
(host.addr_heap OPTIONAL_OFFSET_NUMBER:i16 OPERAND_FOR_ADDR)
(host.addr_func $name)
(host.copy_from_heap
    OPERAND_FOR_dst_pointer:i64
    OPERAND_FOR_src_offset:i64
    OPERAND_FOR_length_in_bytes:i64)

(host.copy_to_heap
    OPERAND_FOR_dst_offset:i64
    OPERAND_FOR_src_pointer:i64
    OPERAND_FOR_length_in_bytes:i64)