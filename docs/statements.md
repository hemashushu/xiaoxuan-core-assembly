# The Statements

## Using identifiers

To import identifiers (the name of functions or data) from other modules, use the `use` keyword:

`use namepath`

Where `namepath` is formed of the module name, namespace path and identifier, i.e. `module_name::sub_module_name::identifier`.

Example of `use` statement:

```rust
use std::memory::copy
use digest::sha2::init
```

You can rename an imported identifer using the `as` keyword:

`use namepath as identifier`

For example:

```rust
use std::memory::copy as mem_copy
```

Using identifiers from other namespaces of the current module:

- `module`: The current module
- `self`: The current namespace
- `parent`: The parent namespace

For example:

```rust
use module::sub_module::some_func
use self::sub_module::some_func
use parent::sub_sub_module::some_data as other_data
```

## External Functions and Data

To declear external functions or data, use the `external` keyword:

- `external fn library_name::identifier signature [as identifier]`
- `external data library_name::identifier:data_type [as identifier]`

Example of `external` statement:

```rust
external fn libfoo::add(i32, i32) -> i32 as i32_add
external data libfoo::PI:f32 as CONST_PI
```

The possible data types of function's parameters and return value are: `i64`, `i32`, `f64` and `f32`. For external data, in addition to the previous, there are `byte[]`, which means that the target data can be arbitrary.

> Note: XiaoXuan Core VM does not yet support external data.

## Data Definitions

To define data, use the `data` keyword:

`[pub] [readonly|uninit] data name:type [=value]`

Example of `data` statement:

```rust
data foo:i32 = 0x11
pub readonly data bar:i32 = 0x13
pub uninit data baz:i64
```

The possible value of data are:

- Numbers: includes decimal, hexadecimal, binary, float-point, hex float-point.
- Strings: normal string, multiline string, long string, raw string, raw string with hash symbol, auto-trimmed string.
- Hex byte data.
- List. The element of list can be numbers, strings, hex byte data and list.

todo:: examples

There are two ways to declare the length of a byte array:

1. `byte[length]`: Specify the length of byte array directly. If the length of the content if less than the byte array, the remainder of the byte array is padded with the number 0.
2. `byte[]`: Do not specify the length of the byte array, the length is automatically determined by the length of content.

```rust
pub data foo1:byte[32] = h"11 13 17 19" // length is 32
pub data foo1:byte[32] = [0x11_i32, 0x13_i32, 0x17_i32, 0x19_i32] // length is 32
pub data foo2:byte[] = [0x11_i32, 0x13_i32, 0x17_i32, 0x19_i32] // length is 4
pub data foo3:byte[] = "Hello, World!" // length is 13
pub data foo4:byte[] = "Hello, World!\0" // length is 13+1
pub data foo5:byte[] = ["Hello, World!", 0_i8] // length is 13+1
```

The `byte` type can also be specified alignment, e.g.:

- `byte[1024, align=8]`
- `byte[align=4]`

## Function Definitions

To define functions, use the `fn` keyword:

`[pub] fn name (params) -> returns [locals] {...}`

where `[locals]` is an optional list of local variables.

Example of `fn` statement:

```rust
// Function with 2 parameters and 1 return value.
fn add(left:i32, right:i32) -> i32 {
    add_i32(
        local_load_i32_s(left)
        local_load_i32_s(right)
    )
}

// Function with local variables.
pub fn handle(number:i32)
    [var0:i32, var1:byte[16], var2:f32] {
    ...
}
```

The function body can also be an expression, e.g.:

```rust
fn inc_one(num:i32) -> i32
    add_imm_i32(1, local_load_i32_s(left))
```

## Line break rules for source code

**1. A statement or expression must be followed by a newline.**

todo:: examples

**2. Line breaks are allowed after symbols indicating the beginning of a block, such as `(`, `{` and `[`.**

todo:: examples

**3. Line breaks are allowed after symbols indicating that more content will follow, such as `->`, `=`, and `:`.**

todo:: examples

**4. Commas (`,`) can be replaced with newlines for separating content.**

todo:: examples

Note that both commas and new lines can be used together.

todo:: examples

**5. Line breaks are allowed after keywords indicating that specific content will follow, such as `fn`, `data`, and `use`.**

todo:: examples

**6. Newlines cannot be placed after modifier keywords, such as `pub`, `external`, and `readonly`.**

todo:: examples
