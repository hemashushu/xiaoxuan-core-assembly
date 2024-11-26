# Expressions

<!-- @import "[TOC]" {cmd="toc" depthFrom=2 depthTo=4 orderedList=false} -->

<!-- code_chunk_output -->

- [Groups](#groups)
- [Control Flow Expressions](#control-flow-expressions)
  - [Condition without branch](#condition-without-branch)
  - [Condition with branch](#condition-with-branch)
  - [Block](#block)
  - [Break](#break)
  - [Recur](#recur)

<!-- /code_chunk_output -->

The function body consists of expressions. There are three types of expressions:

- Instruction expressions
- Control flow expressions
- Groups

## Groups

```rust
{
    expression0
    expression1
    ...
}
```

A group returns one or more values, or no values at all, the number of values being determined by the expressions within it.

For example:

- If there are two `_load_` instructions in the group, two values are returned.
- If there is one `_store_` instruction and one `_load_` instruction, one value is returned.
- If there are two `_store_` instructions, no value is returned.

## Control Flow Expressions

### Condition without branch

`when testing [locals] consequence`

Where:

- `consequence` is an expression.
- `[locals]` is a list of local variables, e.g. `[foo:i32, bar:byte[16], align(baz:byte[32], 4)]`

`when` expressions have no return value.

### Condition with branch

`if params -> returns tesing consequence alternative`

Where:

- `consequence` and `alternative` they are both an expression.
- `returns` indicates the type of the return value of `if` expression. It can be:
  - `()` means no return value.
  - `data_type` indicates that only one value is returned.
  - `(data_type0, data_type1, ...)` returns multiple values.

Note that:

- `if` expression has no list of local variables.
- If the expression has no return value, the `-> returns` can be omitted.
- If the expression has no params but has return values, the format is `() -> returns`.
- If the expression has no params and no return value, the entire `params -> returns` can be omitted.

### Block

`block params -> returns [locals] body`

Where:

- `params` is a list of parameters, e.g.`(left:i32, right:i32)`, or `()` if the expression has no parameteres, note that this part cannot be omitted.
- `returns` is a list of the types of return values, as in `if` expressions, and if there is no return value, the entire `-> ()` can be omitted.
- `body` is an expression, usually it is a `group` expression.

Variants:

- `for (params) -> returns [locals] body` a recurable block

### Break

`break (value0, value1, ...)`

Break the nearest `for` expression, this expression never return.

Variants:

- `break_if testing (value0, value1, ...)`
  Break only when the `testing` expression returns true.
- `break_fn (value0, value1, ...)`
  Break to the current function.

### Recur

`recur (value0, value1, ...)`

Recur to the nearest `for` expression , this expression never return.

Variants:

- `recur_if testing (value0, value1, ...)`
  Recur only when the `testing` expression returns true.
- `recur_fn (value0, value1, ...)`
  Recur to the current function.
