# Data types and literals

<!-- @import "[TOC]" {cmd="toc" depthFrom=2 depthTo=4 orderedList=false} -->

<!-- code_chunk_output -->

- [Data Types](#data-types)
- [Literals](#literals)
  - [Numbers](#numbers)
  - [Strings](#strings)
  - [Byte Arrays](#byte-arrays)
  - [List](#list)

<!-- /code_chunk_output -->

## Data Types

Possible data types for parameters and return values of function are:

- `i64`: 64-bit integers
- `i32`: 32-bit integers
- `f64`: 64-bit floating-piont numbers
- `f32`: 32-bit floating-piont numbers

In addition to the data types above, the data types of local variables (局部变量), as well as "(thread-local) data" and "external data" support:

- `i16`: 16-bit integers
- `i8`: 8-bit integers
- `byte[length]`: fixed-length byte arryas

Note that when declaring "read-write data", "read-only data" and "external data", an unspecified length byte keyword `byte[]` is allowed, the length of such byte arrays is determined by the specific content. Note that it is still fixed-length byte array instead of a new data type.

Type `byte[length]` has an optional parameter `align=N`, which is used to specify the alignment. e.g. `byte[128, align=8]` declares a 128-byte array with 8-byte alignment.

## Literals

Literals are used for initialization values of `data`, where number literals are also used for parameters of instructions.

### Numbers

TODO

- `i8`
- `i16`
- `i32`
- `i64`
- `f32`
- `f64`

### Strings

TODO

### Byte Arrays

TODO

### List

TODO

[value0, value1, ...]

```json
[
    [value_a0, value_a1, ...]
    [value_b0, value_b1, ...]
]
```
