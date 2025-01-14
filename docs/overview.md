# Overview

## XiaoXuan Core Module

### Module Structure

```text
project_folder
 |-- module.anc.ason    // module(package/project) configuration
 |-- README.md
 |-- LICENSE.md etc.
 |-- src
 |   |-- lib.anca       // top-level submodule
 |   |-- main.anca      // the default executable unit
 |   |-- foo.anca       // submodule
 |   |-- subfolder
 |       |-- bar.anca   // another submodule
 |
 |-- app
 |   |-- cmd1.anca      // sub-executable unit
 |   |-- cmd2.anca      // sub-executable unit
 |
 |-- tests              // unit test source files
 |   |-- test1.anca     // a testing unit
 |   |-- test2.anca     // another testing unit
 |   |-- subfolder
 |       |-- bar.anca   // submodule for unit testing only
 |
 |-- doc
 |   |-- README.md      // documentations
 |
 |-- build              // the building assets

```

### Code Example

Code of file `./src/main.anca`:

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

### Configuration Example

Content of file `./module.anc.ason`:

```json5
{
    name: "hello"
    version: "1.0.0"
    edition: "2025"
    properties: [
        // Declares properties and there default values for
        // used by the current program (module).
        // This value of the property declared here can be
        // read in the program's souce code by the macro `prop!(...)`.
        //
        //
        // Strings can interoperate with constants using
        // the placeholder `{name}`
        //
        // e.g.
        // version: "{logger_version}"

        "enable_abc": prop::bool(true)
        "enable_xyz": prop::bool(false)
        "logger_version": prop::string("1.0.1")
        "bits": prop::number(32)
        "enable_logger": prop::eval("enable_abc && enable_xyz")
    ]
    modules: [
        "std": module::Runtime
        "digest": module::Share({
            version: "1.0"
            // Pass values to the "properties" of module "digest"
            parameters: [
               "enable_sha2": param::bool(true)
               "enable_md5": param::bool(false)
               "bits": param::prop("bits")
               "enable_abc": param::prop("enable_abc")
               "enable_foo": param::eval("not(enable_md5)")
            ]
            repository: "custom"
        })
        "logger": module::Share({
            version: "{logger_version}"
            condition: cond::is_true("enable_logger")
        })
    ]
    libraries: [
        "libfoo": library::Remote({
            url: "https://github.com/..."
            revision: "v1.0.1"
            path: "/lib/libfoo.so.1"
        })
    ]
    module_repositories: [
        "name": "https://..."
        // ...
    ]
    library_repositories: [
        "name": "https://..."
        // ...
    ]
}
```

## XiaoXuan Core Assembly Language (AncASM)

- [Data types and literals](./datatypes.md)
- [Statements](./statements.md)
- [Expressions](./expressions.md)
- [Instruction](./instructions.md)
