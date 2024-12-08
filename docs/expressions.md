# Expressions

<!-- @import "[TOC]" {cmd="toc" depthFrom=2 depthTo=4 orderedList=false} -->

<!-- code_chunk_output -->

- [Groups](#groups)
- [Control Flow Expressions](#control-flow-expressions)
  - [When](#when)
  - [If](#if)
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

### When

Condition without branch.

`when [locals] testing consequence`

Where:

- `[locals]` is a list of local variables, e.g. `[foo:i32, bar:byte[16], align(baz:byte[32], 4)]`
- `consequence` is an expression.

`when` expressions have no return value.

### If

Condition with alternative branch.

`if -> results tesing consequence alternative`

Where:

- `consequence` and `alternative` they are both an expression.
- `results` indicates the type of the return value of `if` expression. It can be:
  - `()` means no return value.
  - `data_type` indicates that only one value is returned.
  - `(data_type0, data_type1, ...)` returns multiple values.

Note that:

- `if` expression has no list of local variables.
- If the expression has no return value, the `-> results` can be omitted.
- If the expression has no params but has return values, the format is `() -> results`.
- If the expression has no params and no return value, the entire `params -> results` can be omitted.

<!--
### Branch

```json
branch {
    case testing consequence
    case testing consequence
    default consequence
}
```
-->

### Block

`block param_values -> results [locals] body`

Where:

- `param_values` is a list of parameters and values, e.g.`(left:i32=value, right:i32=value)`, or `()` if the expression has no parameteres, note that this part cannot be omitted.
- `results` is a list of the types of return values, as in `if` expressions, and if there is no return value, the entire `-> ()` can be omitted.
- `body` is an expression, usually it is a `group` expression.

### Break

`break (value0, value1, ...)`

Break the nearest `for` expression, this expression never return.

Variants:

<!--
- `break_if testing (value0, value1, ...)`
  Break only when the `testing` expression returns true.
-->
- `break_fn (value0, value1, ...)`
  Break to the current function.

### Recur

`recur (value0, value1, ...)`

Recur to the nearest `for` expression , this expression never return.

Variants:

<!--
- `recur_if testing (value0, value1, ...)`
  Recur only when the `testing` expression returns true.
-->

- `recur_fn (value0, value1, ...)`
  Recur to the current function.
