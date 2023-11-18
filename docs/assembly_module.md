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

_Applications_ are similar to _Shared Libraries_, but applications have an additional function called `main` that provides the entry point for the application.

## The `module` Node

An assembly text file can only define one module, so the content of an assembly text is a large node called `module`. Within this node, functions and data are defined, as well as declarations of functions and data imporetd from other modules or shared libraries. An example of the smallest module is as follows:

```clojure
(module $app
    (runtime_version "1.0")
    (fn $main (result i32)
        (code
            (i32.imm 42)
        )
    )
)
```

In the above example, `$app` is an identifier that represents the name of the current module (note that the name must be immediately followed by the symbol `module`). The module name also expresses the module's namespace, for example, `std__math__complex` represents the namespace `std::math::complex`.

After the name is the child node `runtime_version`, it is a parameter of node `module`, which indicates the expected version of the runtime, followed by the nodes of user-defined data and functions.

A module should at least define one data or function node, otherwise it is useless (although it is a valid module). For an application, at least one function called "main" should be defined, otherwise it cannot pass the assembler check.

> Save the above assembly code to a file say "a.anc" and then execute the command `$ ancl a.anc; echo $?`, you should see the output "42".

### Module Optional Parameters

The node `module` has some optional parameters:

- `constructor`: A constructor function that is run after the application is loaded and before the main function. It usually performs some data initilization.

- `destructor`: The destructor function, which is run after the main function and before the application exits. It is usually used to do some resource collection work.

The following is an example that uses these parameters:

```clojure
(module $app
    (runtime_version "1.0")

    (constructor $init)
    (destructor $exit)

    (fn $init ...)
    (fn $main ...)
    (fn $exit ...)
)
```

## The `fn` Node

(fn $name (param $p0 i32) (param $p1 i32) (result i32)
    (code ...)
)

parameters and results data type:

- i32
- i64
- f32
- f64

no parameters

(fn $name (result i32)
    (code ...)
)

no return values

(fn $name
    (code ...)
)

multiple return values

(fn $name (result i32) (result i32)
    (code ...)
)

or

(fn $name (results i32 i32)
    (code ...)
)

### local variables

(fn $func_name
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

(fn $func_name
    (local $buf (bytes 12 4))
    (code ...)
)

### export function

add 'exported' annotation after the function name.

(fn $name exported ...)

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

also
;; UTF-8 encoding string
(data $name (read_only string "Hello, World!"))

;; type `cstring` will append '\0' at the end of string
(data $name (read_only cstring "Hello, World!"))

other sections than 'read_only'

read-write section:
(data $name (read_write i32 123))

uninitialized section:
(data $name (uninit i32))
(data $name (uninit (bytes 12 4)))
(data $name (uninit (bytes DATA_LENGTH_NUMBER:i32 ALIGN_NUMBER:i16)))

with 'exported' annotation
(data $name exported (read_only i32 123))
