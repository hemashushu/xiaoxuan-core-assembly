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

There is no hierarchical relationship between multiple modules, at the assembly level, they are flat.

_Applications_ are similar to _Shared Libraries_, but applications have an additional function called `entry` that provides the entry point for the application.

## The `module` Node

An assembly text file can only define one module, the top content of an assembly text is a node called `module`. Within this node, functions and data are defined, as well as declarations of functions and data imporetd from other modules or shared libraries. An example of the smallest module is as follows:

```clojure
(module $app
    (compiler_version "1.0")
    (function $test (result i32)
        (code
            (i32.imm 42)
        )
    )
)
```

> Save the above assembly code to a file say "a.anc" and then execute the command `$ ancl a.anc; echo $?`, you should see the output "42".

### Module Name

In the above example, `$app` is an identifier that represents the name of the current module <!-- i.e. the name of the application or library -->. The valid characters of the name are `[a-zA-Z0-9_]`, and the name must be immediately followed by the symbol `module`.

For an application or library with multiple source files, the main module file names are `main.ancasm` and `lib.ancasm`, and any other source files will be used as submodules. The names of submodules must contains the namespace paths. For example, consider a library call `draw` that has 3 source files:

```text
- lib.ancasm
- circle.ancasm
- rectangle.ancasm
```

Their module names should be `draw`, `draw::circle` and `draw::rectangle`, although the module name and the module file name do not necessary need to be the same.

### Runtime Version

The main module must contains a child node called `runtime_version`, it is a parameter of module, which indicates the expected version of the runtime.

The node `runtime_version` is followed by the nodes of user-defined data and functions. A module should at least define one data or function node, otherwise it is useless (although it is a valid module). For an application, at least one function called "entry" should be defined, otherwise it cannot pass the assembler check.

### Other Module Optional Parameters

The main module has some other optional parameters:

- `constructor`: A constructor function that is run after the application is loaded and before the "entry" function. It usually performs some data initilization.

- `destructor`: The destructor function, which is run after the "entry" function and before the application exits. It is usually used to do some resource collection work.

The following is an example that uses these parameters:

```clojure
(module $app
    (compiler_version "1.0")
    (constructor $init)
    (destructor $exit)

    ...
)
```

## The `function` Node

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

bytes DATA_LENGTH:i32 ALIGN:i16

e.g.

(function $function_name
    (local $buf bytes 12 4)
    (code ...)
)

### export function

add 'export' annotation after the function name.

(function $name export ...)

## The `data` node

(data $name (read_only i32 123))
(data $name (read_only i32 0xaabb_ccdd))
(data $name (read_only i32 0b1010_0101))
(data $name (read_only i64 123_456))
(data $name (read_only f32 3.1415927))
(data $name (read_only f32 0x1.23p4))
(data $name (read_only f64 2.718281828459045))
// bytes
(data $name (read_only bytes h"11-13-17-19" OPTIONAL_ALIGN:i16))
(data $name (read_write bytes h"11-13-17-19" OPTIONAL_ALIGN:i16))

there are two variants of data type 'bytes':
- 'string'
- 'cstring'

e.g.

// UTF-8 encoding string
(data $name (read_only string "Hello, World!"))

// type `cstring` will append '\0' at the end of string
(data $name (read_only cstring "Hello, World!"))

they will be converted into 'bytes' by assembler.

other sections than 'read_only'

read-write section:
(data $name (read_write i32 123))

uninitialized section:
(data $name (uninit i32))
(data $name (uninit bytes DATA_LENGTH:i32 OPTIONAL_ALIGN:i16))
(data $name (uninit bytes 12 4))

with 'export' annotation
(data $name export (read_only i32 123))

## The 'depend' node

(depend
    (module $math "math" "1.0" share)
    (library $libc "libc.so.6" share)
)

module type:
- share: `(module $id share "math" "1.0")`
- user:  `(module $id user "format" "1.2")`

library type:
- share:  `(library $id share "math.so.1")`
- system: `(library $id system "libc.so.6")`
- user:   `(library $id user "libtest0.so.1")`

## The 'import' node

(import $math
    (function $add "add" (param i32) (param i32) (result i32))
    (function $add_wrap "wrap::add" (params i32 i32) (results i32))
)

where the '$math' is the depend module id.

### the import function node

(function $add "add" (param i32) (param i32) (result i32))
(function $add_wrap "wrap::add" (params i32 i32) (results i32))

### the import data node

(data $msg "msg" read_only i32)
(data $sum "sum" read_write i64)
(data $buf "utils::buf" uninit bytes)

for the variants of 'bytes' such as 'string' and 'cstring', use 'bytes' instead in the data-import node.

> At the assembly level, sub-modules are transparent to each other, i.e., all
> functions and data (including imported functions, imported data, and
> declared external functions) are public and can be accessed in any sub-module.
> That is, when you import a function (or a data, or declare an external function) into
> a module (or sub-module), the function is just like
> a normal function of the module itself, and it can
> be accessed by any other sub-modules.

## The 'external' node

(external $libtest
    (function $add "add" (param i32) (param i32) (result i32))
    (function $sub_i32 "sub" (params i32 i32) (result i32))
    (function $pause "pause_1s")
)

where the '$libtest' is the depend library id.

### the external function node

there is no identifier in the 'param' nodes, and the parameters can be writtern as compact mode, e.g.

(function $add "add" (params i32 i32) (result i32))
(function $getenv "getenv" (param i64) (result i64))
