# XiaoXuan Core Assembly Module

<!-- @import "[TOC]" {cmd="toc" depthFrom=1 depthTo=6 orderedList=false} -->

<!-- code_chunk_output -->

- [XiaoXuan Core Assembly Module](#xiaoxuan-core-assembly-module)
  - [Fundamnetal](#fundamnetal)
  - [local variable loading and storing](#local-variable-loading-and-storing)
  - [local variable loading and storing with dynamical offset](#local-variable-loading-and-storing-with-dynamical-offset)
  - [data loading and storing](#data-loading-and-storing)
  - [data loading and storing with dynamical offset](#data-loading-and-storing-with-dynamical-offset)
  - [heap loading and storing](#heap-loading-and-storing)
  - [heap memory](#heap-memory)

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

## local variable loading and storing

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

## local variable loading and storing with dynamical offset

(local.load64_i64 $VARIABLE_NAME OPERAND_FOR_OFFSET:i32)

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

(local.load64_i64 $VARIABLE_NAME OPERAND_FOR_OFFSET:i32 OPERAND_FOR_STORING)

- `local.long_store64`
- `local.long_store32`
- `local.long_store16`
- `local.long_store8`

## data loading and storing

(data.load64_i64 $DATA_NAME OPTIONAL_OFFSET_NUMBER:i16)

variants

- `data.load64_i64`
- `data.load64_f64`
- `data.load32_i32`
- `data.load32_i16_s`
- `data.load32_i16_u`
- `data.load32_i8_s`
- `data.load32_i8_u`
- `data.load32_f32`

storing

(data.store64 $DATA_NAME OPTIONAL_OFFSET_NUMBER:i16 OPERAND_FOR_STORING)

- `data.store64`
- `data.store32`
- `data.store16`
- `data.store8`

## data loading and storing with dynamical offset

(data.long_load64_i64 $DATA_NAME OPERAND_FOR_OFFSET:i32)

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

(data.long_store64 $DATA_NAME OPERAND_FOR_OFFSET:i32 OPERAND_FOR_STORING)

variants:

- `data.long_store64`
- `data.long_store32`
- `data.long_store16`
- `data.long_store8`

## heap loading and storing

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

## heap memory

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
