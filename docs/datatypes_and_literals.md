# Data types and literals

<!-- @import "[TOC]" {cmd="toc" depthFrom=2 depthTo=4 orderedList=false} -->

<!-- code_chunk_output -->

- [Data Types](#data-types)
- [Literals](#literals)
  - [Numbers](#numbers)
  - [Strings](#strings)
  - [List](#list)

<!-- /code_chunk_output -->

## Data Types

Possible data types for parameters and return values of function are:

- i64: 64-bit integers
- i32: 32-bit integers
- f64: 64-bit floating-piont numbers
- f32: 32-bit floating-piont numbers

In addition to the data types above, the data types of local variables (局部变量) (as well as "data" and "external data") support:

- i16: 16-bit integers
- i8: 8-bit integers
- byte[length]: fixed-length byte arryas

Note that when declaring "read-write data", "read-only data" and "external data", an unspecified length byte array `byte[]` is sometimes used. The length of such byte arrays is determined by the specific content, so it is not a new data type.

## Literals

Literals are used for initialization values of `data`, and parts of number literals are used for parameters of instructions.

### Numbers

TODO

- i8
- i16
- i32
- i64
- f32
- f64

### Strings

TODO

### List

[value0, value1, ...]

```js
[
    [value_a0, value_a1, ...]
    [value_b0, value_b1, ...]
]
```
