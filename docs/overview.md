# Overview

Example:

```rust
// using functions from other shared modules
use std::memory::copy
use digest::sha2::init

// using functions and datas from other namespace of the current module
use module::sub_module::some_func
use self::sub_module::some_func
use parent::sub_sub_module::some_data as other_data

// declear the functions and datas from external libraries
external fn libfoo::add(i32, i32) -> i32 as i32_add
external data libfoo::PI:f32 as CONST_PI

data foo:i32 = 0x11
pub data bar:byte[32] = b"01 02 03 04 ..."
pub readonly data baz:i32 = 0x13
pub uninit buz:i64

fn add(left:i32, right:i32) -> i32 {
    add_i32(
        local_load_i32_s(left)
        local_load_i32_s(right)
    )
}

pub fn inc(num:i32) -> i32 {
    add_imm_i32(
        local_load_i32_s(num)
    )
}

pub fn entry() -> i32 {
    imm_i32(42)
}
```

content of the file `module.ason`:

```json5
{
    type: "anc"
    name: "hello"
    version: "1.0.0"
    runtime_version: "1.0"
    entry: "entry"
    dependencies: [
        "std": module::Runtime
        "digest": module::Share({
            repository: Option::Some("internal")
            version: "1.0"
            // properties: {}
        })
    ]
    libraries: [
        "libfoo": library::Remote({
            url: "https://github.com/..."
            revision: "v1.0.1"
            path: "/lib/libfoo.so.1"
        })
    ]
    properties: {
        // ...
    }
    variables: {
        // ...
    }
}
```

data types:

- i64
- i32
- f64
- f32
- byte[length]

the data types for function parameters and return values can only be `i64/i32/f64/f32`.