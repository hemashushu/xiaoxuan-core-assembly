# Data types and literals

## Data Types

Data types for parameters and return values of function:

- i64: unsigned 64-bit integers
- i32: unsigned 32-bit integers
- f64: 64-bit floating-piont numbers
- f32: 32-bit floating-piont numbers

The data types for local variables (局部变量) (as well as "data" and "external data") support fixed-length byte arryas (`byte[length]`) in addition to the data types above.

Note that when declaring "uninitialized data" and "external data", an unspecified length byte array `byte[]` is sometimes used. The length of such byte arrays is determined by the specific content, so it is not a new data type.

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
