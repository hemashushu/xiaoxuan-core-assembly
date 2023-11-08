# XiaoXuan Core Assembly Module

Both _XiaoXuan Core Applications_ and _XiaoXuan Core Shared Libraries_ consist of one or more modules.

For _Shared Libraries_, multiple modules are not dependent on each other, and they provide accessible functions and data to the outside world on an equal basis.

_Applications_ are similar to _Shared Libraries_, but applications have an additional function called `main` that provides the entry point for the application.

## The `Module` Node

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

