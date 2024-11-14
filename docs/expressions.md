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

A group returns one or more values, or no values at all, the number of values being determined by the expressions within it.

For example:

- If there are two `load` instructions in the group, two values are returned.
- If there is one `store` instruction and one `load` instruction, one value is returned.
- If there are two `store` instructions, no value is returned.

## Condition without branch

`when testing [locals] {...}`

> `{...}` denotes an instruction, expression, group

There is no return value for 'when' statement.

## Condition with branch

`if () -> (returns) tesing {...} {...}`

The return value is `(returns)`

## Block

`block (params) -> (returns) [locals] {...}`

The return value is `(returns)`

Variants:

- `for (params) -> (returns) [locals] {...}`
  Recurable block

## Break

`break (value0, value1, ...)`

Break the nearest 'for'.

This expression never return.

Variants:

- `break_if testing (value0, value1, ...)`
  Break only when the 'testing' returns true.
- `break_fn (value0, value1, ...)`
  Break to the current function.

## Recur

`recur (value0, value1, ...)`

Recur to the nearest 'for'.

This expression never return.

Variants:

- `recur_if testing (value0, value1, ...)`
  Recur only when the 'testing' returns true.
- `recur_fn (value0, value1, ...)`
  Recur to the current function.
