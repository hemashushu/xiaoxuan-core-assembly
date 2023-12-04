# XiaoXuan Core Assembly Module

<!-- @import "[TOC]" {cmd="toc" depthFrom=1 depthTo=6 orderedList=false} -->

<!-- code_chunk_output -->

- [XiaoXuan Core Assembly Module](#xiaoxuan-core-assembly-module)
  - [The `module` Node](#the-module-node)
    - [Module Optional Parameters](#module-optional-parameters)
  - [The `fn` Node](#the-fn-node)
    - [local variables](#local-variables)
    - [exporting function](#exporting-function)
  - [The `data` node](#the-data-node)

<!-- /code_chunk_output -->

Both _XiaoXuan Core Applications_ and _XiaoXuan Core Shared Libraries_ consist of one or more modules.

For _Shared Libraries_, multiple modules are not dependent on each other, and they provide accessible functions and data to the outside world on an equal basis.

_Applications_ are similar to _Shared Libraries_, but applications have an additional function called `entry` that provides the entry point for the application.

## The `module` Node

An assembly text file can only define one module, so the content of an assembly text is a large node called `module`. Within this node, functions and data are defined, as well as declarations of functions and data imporetd from other modules or shared libraries. An example of the smallest module is as follows:

```clojure
(module $app
    (runtime_version "1.0")
    (function $test (result i32)
        (code
            (i32.imm 42)
        )
    )
)
```

> Save the above assembly code to a file say "a.anc" and then execute the command `$ ancl a.anc; echo $?`, you should see the output "42".

### Module Name

In the above example, `$app` is an identifier that represents the name of the current module, i.e. the name of the application or library. The valid characters of the name are `[a-zA-Z0-9_]`, and the name must be immediately followed by the symbol `module`.

For an application or library with multiple source files, the main module file names are `main.ancasm` and `lib.ancasm`, and any other source files will be used as submodules. The names of submodules must contains the namespace paths. For example, consider a library call `draw` that has 3 source files:

```text
- lib.ancasm
- circle.ancasm
- rectangle.ancasm
```

Their module names should be `draw`, `draw::circle` and `draw::rectangle`.

### Runtime Version

A module node must contains the child node `runtime_version`, it is a parameter of node `module`, which indicates the expected version of the runtime.

The node `runtime_version` is followed by the nodes of user-defined data and functions. A module should at least define one data or function node, otherwise it is useless (although it is a valid module). For an application, at least one function called "entry" should be defined, otherwise it cannot pass the assembler check.

### Module Optional Parameters

The node `module` has some optional parameters:

- `constructor`: A constructor function that is run after the application is loaded and before the "entry" function. It usually performs some data initilization.

- `destructor`: The destructor function, which is run after the "entry" function and before the application exits. It is usually used to do some resource collection work.

The following is an example that uses these parameters:

```clojure
(module $app
    (runtime_version "1.0")

    (constructor $init)
    (destructor $exit)

    (function $init ...)
    (function $test ...)
    (function $exit ...)
)
```

## The `fn` Node

(function $name (param $p0 i32) (param $p1 i32) (result i32)
    (code ...)
)

parameters and results data type:

- i32
- i64
- f32
- f64

no parameters

(function $name (result i32)
    (code ...)
)

no return values

(function $name
    (code ...)
)

multiple return values

(function $name (result i32) (result i32)
    (code ...)
)

or

(function $name (results i32 i32)
    (code ...)
)

> the identifier of function can not contains the namespace path separator `::`.


### local variables

(function $function_name
    (local $local_variable_name_0 i32)
    (local $local_variable_name_1 i32)
    (code ...)
)

local variable data types:

- i32
- i64
- f32
- f64
- bytes

bytes syntax:

(bytes DATA_LENGTH_NUMBER:i32 ALIGN_NUMBER:i16)

e.g.

(function $function_name
    (local $buf (bytes 12 4))
    (code ...)
)

### export function

add 'export' annotation after the function name.

(function $name export ...)

## The `data` node

(data $name (read_only i32 123))
(data $name (read_only i64 123_456))
(data $name (read_only f32 3.1415927))
(data $name (read_only f64 2.718281828459045))
(data $name (read_only i32 0xaabb_ccdd))
(data $name (read_only f32 0xdb0f_4940))    ;; Pi
(data $name (read_only i32 0b1010_0101))

;; data
(data $name (read_only (bytes ALIGN_NUMBER:i16) d"11-13-17-19"))

there are two variants of 'bytes': 'string' and 'cstring', e.g.

;; UTF-8 encoding string
(data $name (read_only string "Hello, World!"))

;; type `cstring` will append '\0' at the end of string
(data $name (read_only cstring "Hello, World!"))

they will be converted into 'bytes' by assembler.

other sections than 'read_only'

read-write section:
(data $name (read_write i32 123))

uninitialized section:
(data $name (uninit i32))
(data $name (uninit (bytes 12 4)))
(data $name (uninit (bytes DATA_LENGTH_NUMBER:i32 ALIGN_NUMBER:i16)))

with 'export' annotation
(data $name export (read_only i32 123))

## The 'external' node

(external (library share "math.so.1")
    (function $add "add" (param i32) (param i32) (result i32))
    (function $sub_i32 "sub" (params i32 i32) (result i32))
    (function $pause "pause_1s")
)

there is no identifier in the 'param' nodes, and the parameters can be writtern as compact mode, e.g.

(function $add "add" (params i32 i32) (result i32))

library type:
- (library share "math.so.1")
- (library system "libc.so.6")
- (library user "lib-test-0.so.1")

(external (library system "libc.so.6")
    (function $getuid "getuid" (result i32))
    (function $getenv "getenv" (param (;name;) i64) (result i64))
)

## The 'import' node

import functions:

(import (module share "math" "1.0")
    (function $add "add" (param i32) (param i32) (result i32))
    (function $add_wrap "wrap::add" (params i32 i32) (results i32))
)

import data:

(import (module user "format" "1.2")
    (data $msg "msg" (read_only i32))
    (data $sum "sum" (read_write i64))
    (data $buf "utils::buf" (uninit bytes))
)

for the variants of 'bytes' such as 'string' and 'cstring', use 'bytes' instead in the data-import node.

> At the assembly level, submodules are transparent to each other, i.e., all
> functions and data (including imported functions, imported data, and
> declared external functions) are public and can be accessed in any submodule.

