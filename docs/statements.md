# Statements

<!-- @import "[TOC]" {cmd="toc" depthFrom=2 depthTo=4 orderedList=false} -->

<!-- code_chunk_output -->

- [The `import` Statements](#the-import-statements)
  - [Multi-Source File Modules](#multi-source-file-modules)
  - [Importing Public Functions and Data](#importing-public-functions-and-data)
  - [Importing Functions and Data within the Same Module](#importing-functions-and-data-within-the-same-module)
- [The `external` Statements](#the-external-statements)
- [The `data` Statements](#the-data-statements)
- [The `fn` Statements](#the-fn-statements)
- [Line Break Rules](#line-break-rules)

<!-- /code_chunk_output -->

## The `import` Statements

To import identifiers (the name of functions or data) from the other modules to the current namespace, use the `import` keyword:

- `import fn full_name signature [as new_name]`
- `import [readonly|uninit] data full_name:data_type [as new_name]`

Where:

- `full_name` = `module_name::name_path`
- `name_path` = `namespace::identifier`
- `namespace` = `sub_module_name::`{0,}

When importing functions or data, you can only reference them by their identifiers (not full names or name paths). If there's a naming conflict between multiple imports, use the 'as' keyword to provide an alias for a specific import.

### Multi-Source File Modules

XiaoXuan Core modules (including "applications" and "shared" modules) can consist of either a single file or multiple files located in the same folder (commonly referred to as a "project folder"). Single-file modules can only serve as script applications and cannot be shared modules. Therefore, shared modules and more complex applications are composed of multiple source files.

In a module composed of multiple source files, each source file is a "submodule". For example, in a module named "hello_world", if there are two files, "one.ancasm" and "two.ancasm", inside the "src" folder (note: XiaoXuan Core requires that project source files must be placed inside the "src" folder), then the submodule names of these two files are "one" and "two" respectively.

If there is a subfolder inside the "src" folder, and there are source files in that subfolder, then the name of that subfolder will also be part of the submodule name.

The correspondence between file names and submodule names is shown in the following table:

| File                   | Submodule name | Submodule full name     |
|------------------------|----------------|-------------------------|
| ./src/one.ancasm       | one            | hello_world::one        |
| ./src/two.ancasm       | two            | hello_world::two        |
| ./src/utils/foo.ancasm | utils::foo     | hello_world::utils::foo |
| ./src/utils/codegen/bar.ancasm | utils::codegen::bar | hello_world::utils::codegen::bar |
| ./src/lib.ancasm       | -              | hello_world             |

### Importing Public Functions and Data

When importing public functions and data located within a submodule, the name of the submodule must be added. For example, if there is a function "do_this()" in the submodule "foo", the import statement would be:

`import fn hello_world::one::do_this()`

Note that in the table above, the submodule name for the file "lib.ancasm" is empty (i.e., ""), because this file is the top-level file of the module. When importing public functions and data that reside in the main file, it is not necessary to add the submodule name. For example, if there is a function "do_that()" in "lib.ancasm", the import statement would be:

`import fn hello_world::do_that()`

For applications, there is also a file "src/app.ancasm", which is also the top-level file of the module, so the submodule name is also empty.

| File             | Submodule name | Full submodule name |
|------------------|----------------|---------------------|
| ./src/app.ancasm | -              | -                   |

It's worth nothing that the source files for "multiple executable units" in the "app" folder, as well as the unit test source files in the "test" folder, although they are also normal submodules, cannot be imported by other modules.

| File              | Submodule name | Full submodule name |
|-------------------|----------------|---------------------|
| ./app/cmd1.ancasm | cmd1           | hello_world::cmd1   |
| ./app/cmd2.ancasm | cmd2           | hello_world::cmd2   |
| ./test/one.ancasm | one            | hello_world::one    |
| ./test/utils/foo.ancasm | utils::foo | hello_world::utils::foo |

Source files in the "test" folder are generated only during unit testing and are not included in the binary image of the module for distribution. Therefore, never import unit test functions and data.

> Note: Unlike XiaoXuan Core, all executable units in XiaoXuan Native are independent, so like top-level files, they do not have submodule names.

### Importing Functions and Data within the Same Module

If you want to import functions and data located in other submodules within the same module, you can use the special name "module" instead of the actual name of the current module, e.g.:

- `import fn module::hello_world::do_this()`
- `import fn module::hello_world::one::do_that()`
- `import readonly data module::hello_world::two::message:byte[]`

### Specify Module

The name of module is implicit in the full name of the imported function and data, so the import statement does not need to specify the module. However, sometimes the module name does not match the full name, such as in a merged module (the XiaoXuan Core linker allows multiple modules to be merged into a single module), in which case you can use the `from` keyword to specify the module name, e.g.

- `import fn foo::bar(i32) -> i32 from mymod`
- `import fn network::http_client::get(i64) -> i64 as http_get from commons_module`

## The `external` Statements

To declear external functions or data, use the `external` keyword:

- `external fn full_name signature [as new_name]`
- `external data full_name:data_type [as new_name]`

Where:

- `full_name` = `library_name::identifier`

Example of `external` statement:

```rust
external fn libfoo::add(i32, i32) -> i32 as i32_add
external data libfoo::PI:f32 as CONST_PI
```

The possible data types of function's parameters and return value are: `i64`, `i32`, `f64` and `f32`. For external data, in addition to the previous, there are `byte[]`, which means that the target data can be arbitrary.

> Note: XiaoXuan Core VM does not yet support external data.

## The `data` Statements

To define data, use the `data` keyword:

- `[pub] [readonly] data name:type = value`
- `[pub] uninit data name:type`

The keyword `pub` is used to indicate the visibility of this item when this module is used as a shared module.

Note that in the case of static linking, the item is always visible to other modules with or without this keyword.

Example of `data` statement:

```rust
data foo:i32 = 42
uninit data bar:i64
pub readonly data msg:byte[] = "Hello world!"
pub data buf:byte[16] = h"11 13 17 19"
```

### Data Values

The possible value of data are:

- Numbers: includes decimal, hexadecimal, binary, float-point, hex float-point.
- Strings: normal string, multiline string, long string, raw string, raw string with hash symbol, auto-trimmed string.
- Hex byte data string.
- List: the element of list can be numbers, strings, hex byte data string, and list.

List example:

```rust
pub data lst:byte[] = [0x11, 0x13, 0x17, 0x19]

// The content of lst is:
//
// 0x11, 0, 0, 0,
// 0x13, 0, 0, 0,
// 0x17, 0, 0, 0,
// 0x19, 0, 0, 0,
//
// the number `0x11` in memory is `0x11, 0, 0, 0` since
// the default type of number is `i32`.

pub data obj:byte[align=8] = [
    "foo", 0_i8,
    [0x23_i32, 0x29_i32],
    [0x31_i16, 0x37_i16],
    0xff_i64
]

// The content of obj is:
//
// 0x66, 0x6f, 0x6f, 0,
// 0x23, 0, 0, 0, 0x29, 0, 0, 0,
// 0x31, 0, 0x37, 0
// 0xff, 0, 0, 0, 0, 0, 0, 0
```

There are two ways to declare the length of a byte array:

1. `byte[length]`: Specify the length of byte array directly. If the length of the content if less than the byte array, the remainder of the byte array is padded with the number 0.
2. `byte[]`: Do not specify the length of the byte array, the length is automatically determined by the length of content.

```rust
pub data foo1:byte[32] = h"11 13 17 19" // length is 32
pub data foo1:byte[32] = [0x11_i8, 0x13_i8, 0x17_i8, 0x19_i8] // length is 32
pub data foo2:byte[] = [0x11_i8, 0x13_i8, 0x17_i8, 0x19_i8] // length is 4
pub data foo3:byte[] = "Hello, World!" // length is 13
pub data foo4:byte[] = "Hello, World!\0" // length is 13+1
pub data foo5:byte[] = ["Hello, World!", 0_i8] // length is 13+1
```

The `byte` type can also be specified alignment, e.g.:

- `byte[1024, align=8]`
- `byte[align=4]`

### Numeric Literal Type Automatic Conversion

literal_i8
literal_i16
literal_i32 (default int)
literal_i64
literal_f32
literal_f64 (default fp)

TODO::

## The `fn` Statements

To define functions, use the `fn` keyword:

`[pub] fn name (params) -> results [locals] {...}`

where `[locals]` is an optional list of local variables.

The keyword `pub` is used to indicate the visibility of this item when this module is used as a shared module.

Note that in the case of static linking, the item is always visible to other modules with or without this keyword.

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

## Line Break Rules

Ancasm has only 5 types of statements: `use`, `import`, `external`, `data`, and `fn`. Unlike programming languages such as C/C++/Java, Ancasm statements do not require a semicolon (`;`) as a statement terminator. This is because the semantics of Ancasm statements are unambiguous, meaning that no matter how you break lines, indent, or write all statements together, it will not lead to ambiguity. Therefore, semicolons or newlines are not needed to indicate the end of statement.

Of course, for better readability, it is recommended to insert a newline after ecah statement. For example, the following five statements are all terminated with a newline, and an extra newline is inserted between different types of statements:

```ancasm
import fn std::math::sqrt(f64)->f64
import data mymod::count:i32

data readonly msg:byte[] = "hello world!\0"
data foo:i32 = 42

fn bar() {...}
```

In addition to the unambiguous semantics of statements, Ancasm expressions are also semantically unambiguous. Therefore, when writing expressions, you do not need to use semicolons (`;`) or newlines to indicate the end. You can even write all expressions on the same line or insert a newline after each token, which is the same for the Assembler. For example, the following two code blocks are equivalent:

```ancasm
imm_i32(10)
```

and

```ancasm
imm_i32
(
10
)
```

Of course, to prevent programmers from taking advantage of this "highly flexible" syntax to write code that is very difficult to read (such as writing all code on the same line), Ancasm adds a restriction to the syntax: comma separation is required for function arguments.

When calling a function, arguments must be separated by commas (`,`). Similary, when defining a function, mutiple parameters and multiple return values must also be separated by commas. (It is worth mentioning that commas can be replaces with newlines, or commas and newlines can be mixed in this case.)

Example:

```ancasm
fn add(left:i32, right:i32)->i32 {
    add_i32(
        local_load_i32s(left)
        local_load_i32s(right)
    )
}
```

In the above code, the comma between the parameters `left` and `right` cannot be omitted (although it can be replaced with a newline), and the two `local_load_i32s` instruction expressions need to be separated by a comma or newline.

<!--
2. Newline separation for parallel expressions.

In expression group (i.e., expressions enclosed in a pair of curly braces, also known as a code block), multiple parallel expressions must be separated by newlines.

```ancasm
when nez(local_load_i32s(num)) {
    local_store_i32(a, imm_i32(11))
    local_store_i32(b, imm_i32(13))
}
```

The "parallel expressions" refer to expressions at the same level in the same group. For example, the two `local_store_i32` instruction expressions in the above code are parallel. However, the instruction expressions `local_store_i32` and `imm_i32` are not parallel but nested, so `imm_i32` does not need to be written on a separate line.
-->
