# Overview

## XiaoXuan Core Module

### Module Structure

```text
project_folder
 |-- module.anc.ason    // module(package/project) configuration
 |-- README.md
 |-- LICENSE.md etc.
 |-- src
 |   |-- lib.ancasm     // top-level submodule
 |   |-- foo.ancasm     // submodule
 |   |-- subfolder
 |   |   |-- bar.ancasm // another submodule
 |   |-- app.ancasm     // the default executable unit
 |
 |-- app
 |   |-- cmd1.ancasm    // sub-executable unit
 |   |-- cmd2.ancasm    // sub-executable unit
 |
 |-- test               // unit test source files
 |   |-- test1.ancasm
 |   |-- test2.ancasm
 |
 |-- doc
 |   |-- README.md      // documentations
```

### Code Example

Code of file `./src/lib.ancasm`:

```rust
// imports functions and data from other submodules or shared modules
import fn std::memory::copy(i64, i64)
import readonly data digest::sha2::message:byte[]

// import functions and data from external libraries
external fn libfoo::add(i32, i32) -> i32 as i32_add
external data libfoo::PI:f32 as CONST_PI

// define data
data foo:i32 = 0x11
uninit data bar:i64
pub readonly data baz:i32 = 0x13
pub data buf:byte[32] = b"01 02 03 04 ..."

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
pub fn entry() -> i32 {
    imm_i32(0)
}
```

### Configuration Example

Content of file `./module.anc.ason`:

```json5
{
    name: "hello"
    version: "1.0.0"
    runtime_version: "1.0"
    constants: {
        // Declares constants and their values for used
        // by the current configuration file.
        //
        // e.g.
        //
        // "foo": const::number(123)
        // "bar": const::bool(true)
        // "logger_version": const::string("1.1")
        //
        // Note that the name of a constant cannot duplicate
        // the name of property.
        //
        // Strings can interoperate with constants using
        // the placeholder `{name}`
    }
    properties: {
        // Declares properties and there default values for
        // use by the current program (module).
        // This value of the property declared here can be
        // read in the program's souce code by the macro `prop!(...)`.
        //
        // e.g.
        //
        // "enable_abc": prop::default::bool(true)
        // "enable_def": prop::default::bool(false)
        // "enable_logger": prop::eval("{enable_abc} && {enable_def}")
    }
    modules: [
        "std": module::Runtime
        "digest": module::Share({
            repository: Option::Some("internal")
            version: "1.0"
            // Pass values to the "properties" of module "digest"
            values: {
               "enable_sha2": value::bool(true)
               "enable_md5": value::bool(false)

               /* Where `{enable_xyz}` is the name of a property or constant
                  declared in the current configuration file.
               */
               "enable_foo": value::eval("{enable_xyz}")
            }
        })
        "logger": module::Share({
            version: "{logger_version}"
            condition: Option::Some(cond::is_true("PROFILE_DEVEL"))
        })
    ]
    libraries: [
        "libfoo": library::Remote({
            url: "https://github.com/..."
            revision: "v1.0.1"
            path: "/lib/libfoo.so.1"
        })
    ]
}
```

## XiaoXuan Core Assembly Language (Ancasm)

- [Data types and literals](./datatypes.md)
- [Statements](./statements.md)
- [Expressions](./expressions.md)
- [Instruction](./instructions.md)

