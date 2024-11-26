# The Statements

<!-- @import "[TOC]" {cmd="toc" depthFrom=2 depthTo=4 orderedList=false} -->

<!-- code_chunk_output -->

- [The `use` Statements](#the-use-statements)
- [The `external` Statements](#the-external-statements)
- [The `data` Statements](#the-data-statements)
- [The `fn` Statements](#the-fn-statements)
- [Line Break Rules](#line-break-rules)

<!-- /code_chunk_output -->

A module consists of statements.

## The `use` Statements

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

## The `external` Statements

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

## The `data` Statements

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

## The `fn` Statements

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

## Line Break Rules

Anasm has only four types of statements: `use`, `external`, `data`, and `fn`. Unlike programming languages such as C/C++/Java, Anasm statements do not require a semicolon (`;`) as a statement terminator. This is because the semantics of Anasm statements are unambiguous, meaning that no matter how you break lines, indent, or write all statements together, it will not lead to ambiguity. Therefore, semicolons or newlines are not needed to indicate the end of statement.

Of course, for better readability, it is recommended to insert a newline after ecah statement. For example, the following five statements are all terminated with a newline, and an extra newline is inserted between different types of statements:

```anasm
use std::math::sqrt
use mymod::format

data readonly msg:byte[] = "hello world!\0"
data foo:i32 = 42

fn bar() {...}
```

In addition to the unambiguous semantics of statements, Anasm expressions are also semantically unambiguous. Therefore, when writing expressions, you do not need to use semicolons (`;`) or newlines to indicate the end. You can even write all expressions on the same line or insert a newline after each token, which is the same for the Assembler. For example, the following two code blocks are equivalent:

```anasm
imm_i32(10)
```

and

```anasm
imm_i32
(
10
)
```

Of course, to prevent programmers from taking advantage of this "highly flexible" syntax to write code that is very difficult to read (such as writing all code on the same line), Anasm adds two restrictions to the syntax:

1. Comma separation for function arguments.

When calling a function, arguments must be separated by commas (`,`). Similary, when defining a function, mutiple parameters and multiple return values must also be separated by commas. (It is worth mentioning that commas can be replaces with newlines, or commas and newlines can be mixed in this case.)

Example:

```anasm
fn add(left:i32, right:i32)->i32 {
    add_i32(
        local_load_i32s(left)
        local_load_i32s(right)
    )
}
```

In the above code, the comma between the parameters `left` and `right` cannot be omitted (although it can be replaced with a newline), and the two `local_load_i32s` instruction expressions need to be separated by a comma or newline.

2. Newline separation for parallel expressions.

In expression group (i.e., expressions enclosed in a pair of curly braces, also known as a code block), multiple parallel expressions must be separated by newlines.

```anasm
when nez(local_load_i32s(num)) {
    local_store_i32(a, imm_i32(11))
    local_store_i32(b, imm_i32(13))
}
```

The "parallel expressions" refer to expressions at the same level in the same group. For example, the two `local_store_i32` instruction expressions in the above code are parallel. However, the instruction expressions `local_store_i32` and `imm_i32` are not parallel but nested, so `imm_i32` does not need to be written on a separate line.
