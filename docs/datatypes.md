# Data types and literals

<!-- @import "[TOC]" {cmd="toc" depthFrom=2 depthTo=4 orderedList=false} -->

<!-- code_chunk_output -->

- [Data Types](#data-types)
- [Literals](#literals)
  - [Numbers](#numbers)
    - [Decimal and floating-point numbers](#decimal-and-floating-point-numbers)
    - [Hexdecimal](#hexdecimal)
    - [Hex floating-point](#hex-floating-point)
    - [Binary](#binary)
    - [Explict data type](#explict-data-type)
  - [Strings](#strings)
    - [Escape chars](#escape-chars)
    - [Multiline strings](#multiline-strings)
    - [Long strings](#long-strings)
    - [Raw strings](#raw-strings)
    - [Auto-trimmed strings](#auto-trimmed-strings)
  - [Hex byte data](#hex-byte-data)
  - [List](#list)
    - [Nested list](#nested-list)

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

#### Decimal and floating-point numbers

TODO

#### Hexdecimal

TODO

#### Hex floating-point

TODO

#### Binary

TODO

#### Explict data type

TODO

### Strings

TODO

#### Escape chars

TODO

#### Multiline strings

TODO

#### Long strings

TODO

#### Raw strings

TODO

#### Auto-trimmed strings

TODO

### Hex byte data

TODO

### List

TODO

[value0, value1, ...]

#### Nested list

```json
[
    [value_a0, value_a1, ...]
    [value_b0, value_b1, ...]
]
```
