# Expressions

## Groups

```rust
{
    instructions
    expressions
    groups
    ...
}
```

The value of a group is the value of the last instruction/expression/group.

## Condition without branch

`when testing [locals] {...}`

> `{...}` denotes an instruction, expression, group

There is no return value for 'when' statement.

## Condition with branch

`if -> (returns) tesing then {...} else {...}`

The return value is `(returns)`

## Block

`block (params) -> (returns) [locals] {...}`

The return value is `(returns)`

Variants:

- `for (params) -> (returns) [locals] {...}`
  Recurable block

## Break

`break (value0, value1, ...)`

Break the nearest 'block' or 'for'.

This expression never return.

Variants:

- `break_if testing (value0, value1, ...)`
  Break only when the 'testing' returns true.
- `break_fn (value0, value1, ...)`
  Break to the current functin.

## Recur

`recur (value0, value1, ...)`

Recur to the nearest 'for'.

This expression never return.

Variants:

- `recur_if testing (value0, value1, ...)`
  Recur only when the 'testing' returns true.
- `recur_fn (value0, value1, ...)`
  Recur to the current function.
