# XiaoXuan Core Assembly Syntax

<!-- @import "[TOC]" {cmd="toc" depthFrom=2 depthTo=4 orderedList=false} -->

<!-- code_chunk_output -->

- [Symbol](#symbol)
- [Identifier](#identifier)
- [Number](#number)
- [String](#string)
  - [Long String](#long-string)
  - [Multi-line String](#multi-line-string)
  - [Raw String](#raw-string)
  - [Paragraph String](#paragraph-string)
- [Byte Data](#byte-data)
- [Comments](#comments)
  - [Line Comment](#line-comment)
  - [Block Comment](#block-comment)
  - [Node Comment](#node-comment)

<!-- /code_chunk_output -->

The text of _XiaoXuan Core Assembly_ is consists of symbols, identifiers, numbers, string and byte data, and is written as S-expressions. e.g.

```clojure
(module $app
    (runtime_version "1.0")
    (fn $main (param $num i32) (result i32)
        (i32.add
            (local.load32 $num)
            (i32.imm 11)
        )
    )
)
```

## Symbol

Symbols are the names of internal objects. For example, the name of module elements and instructions.

Symbols consist of characters `[a-zA-Z0-9_.]`, and cannot start with a number.

Example of valid symbols:

`module`, `fn`, `param`, `result`, `i32`, `i32.add`

Every node starts with a symbol. e.g.

`(module ...)`, `(fn ...)`, `(i32.add ...)`

## Identifier

Identifiers are the names of user-defined objects, such as modules, functions and parameters.

An identifier starts with a dollar sign `$` followed by the characters `[a-zA-Z0-9_]`. The name portion of an identifier cannot begin with a number either.

Example of valid identifiers:

`$app`, `$main`, `$num`

## Number

_XiaoXuan Core Assembly_ supports three types of number representation: decimal, hexadecimal and binary. Decimal supports an addition of plus or minus sign and the representation of floating point numbers. In addition, arbitrary underscores can be added between digits.

Example of numbers:

`211`, `223_211`, `0x1113`, `0x1719_abcd`, `0b1100`, `3.14`, `2.998e8`, `6.626e-34`, `+2017`, `-2027`

Floating point numbers can also be represented in hexadecimal, which is the value of a floating pointer number encoded in memory using the [IEEE 754 standard](https://en.wikipedia.org/wiki/IEEE_754). Do not confuse this with [the hexadecimal floating pointer literals](https://en.cppreference.com/w/c/language/floating_constant) in C/C++. Also, hexadecimal and binary do not support the addition of minus sign.

Example of invalid numbers:

`-0xaabb`, `-0b1100`, `0x3.14`, `0b11.10`

## String

A string is a sequence of characters surrounded by a pair of quotes. Strings support any Unicode character (including emojis), and also support the following escape characters:

- `\t`: Horizontal tabulation
- `\r`: Carriage return (CR)
- `\n`: New line character (line feed, LF)
- `\0`: Null character
- `\u{...}`, Unicode code point, e.g. `\u{2d}`, `\u{6587}`
- `\"`: Doube quote
- `\\`: Escape character itself

The following escape characters are used in other language, but they are not supported in the _XiaoXuan Core Assembly_:

- `\v`: Vertical tabulation
- `\f`: Page breaking control character
- `\x..`: ASCII code

Example of strings:

`"abc文字😊"`, `"\u{2d}\u{6587}\0"`, `"foo\nbar"`

> Strings are encoded using UTF-8.

### Long String

Sometimes the contents of a string may be too long to fit on one line. _XiaoXuan Core Assembly_ supports spliting long string into multiple lines by inserting a backslash and a line break in the middle of the string. e.g.

```text
"Hel\
    lo, \
    World!"
```

The above string is equivalent to `"Hello, World!"`. Note that the leading whitespace is automatically trimmed from each line.

### Multi-line String

String also support multiple lines, simply insert line breaks into the string as usual, e.g.

```text
"Hello,
    World! I'm XiaoXuan
    Core Assembly."
```

The above string is equivalent to `"Hello,\n    World! I'm XiaoXuan\n    Core Assembly."`.

Note that leading whitespace is not automatically removed with this format.

### Raw String

Adding a letter `r`` before the first quote to indicate a string is a _raw string_.

The raw strings do not escape any characters, all content will be output as is, e.g.

`r"Hello\nWorld!"` is equivalent to `"Hello\\nWorld!"`

Since raw strings don't support escaping characters, if you need to output the "double quote", you can use the variant of raw strings, i.e. use `r#"..."#` to enclose the string, e.g.

`r#"One "two" three"#` is equivalent to `"One \"two\" three"`.

### Paragraph String

Paragraph strings are used to write long text, where characters are not escaped and leading whitespace on each line is automatically trimmed based on the number of leading spaces in the first line of text.

A paragraph string uses `###` on a separate line to indicate the start and end markers, for example:

```text
    """
    NAME
        ls - list directory contents

    DESCRIPTION
        List information about the FILEs (the current directory by default).
        Sort entries alphabetically if none of -cftuvSUX nor --sort is
        specified.
    """
```

In the above example, since there are 4 leading space characters in the first line, each line truncates 4 leading spaces.

> Note that the `###` must be on a separate line.

## Byte Data

Byte data is used to represent a piece of binary data in memory or on the storage, starting with the letter `d` followed by a pair of double quotes. Inside the quotes is the content of the data, which uses two letters `[0-9a-zA-Z]` to represent a byte, and the characters `[ -:\t\r\n]` are ignored in the content. For example, the following represents the same 4-byte data:

```text
d"0011aabb",
d"0011AABB",
d"00 11 aa bb",
d"00-11-aa-bb",
d"00:11:aa:bb",
d"00 11
  aa bb"
```

## Comments

_XiaoXuan Core Assembly_ supports 3 styles of comments: line comments, block comments and node comments.

### Line Comment

Line comments start with symbol `;;` and continue until the end of the line, e.g.

```clojure
(module
    ;; this is a comment
    (fn $main   ;; this is a comment also
    )
)
```

### Block Comment

Block comments start with the symbol `(;` and end with the symbol `;)`, and nested block comments are supported, for example:

```clojure
(module
    (fn $main
        (; this is a block comment ;)
        (; level one (; level two ;);)
    )
)
```

Block comments have the highest priority, and line comments within block comments are ignored, e.g.

```clojure
(module
    (; block comment // still block comment ;)
)
```

### Node Comment

Adding the `#` sign before left parenthesis of a node will comment out that node and all children of that node, nested node comments are also supported, e.g.

```clojure
(module
    #(fn $main
        #(param $left i32) (param $right i32)
    )
)
```

In the examaple above, the entire node `fn` is commented.

Node comments have the lowest priority, and block and line comments within node comments are still valid, e.g.

```clojure
(module
    #(fn $main
        // this right paren ')' does not end the node comment
        (; this right paren ')' also does not end the node comment ;)
    )
)
```

When modifying assembly text, node comments provide a convenient way to temporarily switch some parameters or child nodes.