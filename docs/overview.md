# Overview

## Code

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

## Configuration

content of the file `module.ason`:

```json5
// @type: anc
{
    name: "hello"
    version: "1.0.0"
    runtime_version: "1.0"
    entry: "entry"
    modules: [
        "std": module::Runtime
        "digest": module::Share({
            repository: Option::Some("internal")
            version: "1.0"
            // Values can be passed to the "properties" of module "digest"
            // via the "values" field.
            //
            // e.g.
            //
            // values: {
            //    "enable_sha2": value::Bool(true)
            //    "enable_md5": value::Bool(false)
            //    "enable_foo": value::calc("{enable_xyz}")
            // }
            //
            // Where `{enable_xyz}` is an property or constant
            // declared by the current configuration file.

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
        // Declares properties and there default values for
        // use by the current program (module).
        // This value of the property declared here can be
        // read in the program's souce code by the macro `prop!(...)`.
        //
        // e.g.
        //
        // "enable_abc": prop::default::Bool(true)
        // "enable_def": prop::default::Bool(false)
        // "enable_xyz": prop::calc("{enable_abc} && {enable_def}")
    }
    constants: {
        // Declares constants and their values for used
        // by the current configuration file.
        //
        // e.g.
        //
        // "foo": const::Number(123)
        // "bar": const::String("abc")
        //
        // Note that the name of a constant cannot duplicate
        // the name of property.
    }
}
```
