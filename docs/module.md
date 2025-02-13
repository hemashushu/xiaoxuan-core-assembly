# Module

<!-- @import "[TOC]" {cmd="toc" depthFrom=2 depthTo=4 orderedList=false} -->

<!-- code_chunk_output -->

- [Source Structure](#source-structure)
- [Module Example](#module-example)

<!-- /code_chunk_output -->

TODO::

## Source Structure

TODO::

## Module Example

```rust
// imports functions and data from other submodules or shared modules
import fn std::memory::copy(i64, i64)
import readonly data digest::sha2::message:byte[]

// import functions and data from external libraries
external fn libfoo::add(i32, i32) -> i32 as i32_add
external data libfoo::PI:f32 as CONST_PI

// define data
data foo:i32 = 42
uninit data bar:i64
pub readonly data msg:byte[] = "Hello world!"
pub data buf:byte[16] = h"11 13 17 19"
pub data obj:byte[align=8] = [
    "foo", 0_i8,
    [0x23_i32, 0x29_i32],
    [0x31_i16, 0x37_i16],
    0xff_i64
]

// define function "add"
fn add(left:i32, right:i32) -> i32 {
    add_i32(
        local_load_i32_s(left)
        local_load_i32_s(right)
    )
}

// define function "inc"
pub fn inc(num:i32) -> i32 {
    // call function "add"
    add(
        local_load_i32_s(num)
        imm_i32(1)
    )
}

// the default entry function
pub fn _start() -> i32 {
    imm_i32(0)
}
```
