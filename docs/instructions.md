# Instruction Expressions

<!-- @import "[TOC]" {cmd="toc" depthFrom=2 depthTo=4 orderedList=false} -->

<!-- code_chunk_output -->

- [Instructions](#instructions)
  - [Base](#base)
  - [Local Loading/Storing](#local-loadingstoring)
  - [Local Loading/Storing Extension](#local-loadingstoring-extension)
  - [Data Loading/Storing](#data-loadingstoring)
  - [Data Loading/Storing Extension](#data-loadingstoring-extension)
  - [Memory Loading/Storing](#memory-loadingstoring)
  - [Memory Management](#memory-management)
  - [Conversion](#conversion)
  - [Comparison](#comparison)
  - [Arithmetic](#arithmetic)
  - [Bitwise](#bitwise)
  - [Math](#math)
  - [Calling](#calling)
  - [Host](#host)

<!-- /code_chunk_output -->

Instruction expressions are used to generate instructions for the virtual machine (VM).

The Syntax of instruction expresions resembles function calls, following the format:

```rust
inst_name(arg0, arg1, ...)
```

Examples:

- `imm_i64(42)`: This loads the immediate 64-integer value 42.
- `local_load_i64(num)`: This loads a 64-bit integer from a local variable named "num".

Type of arguments:

1. Positional arguments

Values can be numberical literals, identifiers (names of functions or data), or the return values of other instruction expressions.

Note that XiaoXuan Core assembly language does not support variable definitions. Therefor, when an argument requires the return value of another instruction, it must be nested. For example:

```rust
local_store_i64(
    temp,
    local_load_i64(num)
)
```

2. Named arguments

These are optional and have the format `name=value`. Values are usually numerical literals. For example:

```rust
local_store_i64(
    temp,           // positional argument
    imm_i64(42),    // positional argument
    offset=4        // named (optional) argument
)
```

Instruction expressions have an almost one-to-one correspondence with the instructions of the VM. However, some parameters will be converted by the assembler. For example, the identifier in the instruction expression `local_load_i64` will be automatically converted to the `index` and `rev-index` of the local varialbe. Additionally, all VM control flow instructions are replaced by the [control flow expressions](./expressions/#control-flow-expressions).

3. Numeric Literal Type Automatic Conversion

imm_i8
imm_i16
imm_i32 (default int)
imm_i64
imm_f32
imm_f64 (default fp)

TODO

## Instructions

### Base

```rust
nop() -> ()
```

Immediately numbers

```rust
imm_i32(imm_i32) -> i32
imm_i64(imm_i64) -> i64
imm_f32(imm_f32) -> f32
imm_f64(imm_f64) -> f64
```

### Local Loading/Storing

```rust
local_load_i64(  identifier, offset=imm_i16) -> i64
local_load_i32_s(identifier, offset=imm_i16) -> i32
local_load_i32_u(identifier, offset=imm_i16) -> i32
local_load_i16_s(identifier, offset=imm_i16) -> i16
local_load_i16_u(identifier, offset=imm_i16) -> i16
local_load_i8_s( identifier, offset=imm_i16) -> i8
local_load_i8_u( identifier, offset=imm_i16) -> i8
local_load_f32(  identifier, offset=imm_i16) -> f32
local_load_f64(  identifier, offset=imm_i16) -> f64

local_store_i64(identifier, value:i64, offset=imm_i16) -> (remain_values)
local_store_i32(identifier, value:i32, offset=imm_i16) -> (remain_values)
local_store_i16(identifier, value:i16, offset=imm_i16) -> (remain_values)
local_store_i8( identifier, value:i8,  offset=imm_i16) -> (remain_values)
local_store_f64(identifier, value:f64, offset=imm_i16) -> (remain_values)
local_store_f32(identifier, value:f32, offset=imm_i16) -> (remain_values)
```

The `identifier` argument is the name of local variables or function parameters.

About the "remain_values"

If there is more than one operand on the stack, the instruction "store" removes the first operand from the stack and leaves the remaining operands. If you think of "store" as a function and the operands as a list, then this function will return a new list that consists of the remaining elements. e.g.

```rust
let mut operands = vec![1,2,3]
let remains = store(&mut operands, &mut local_var_a)
assert!(remains, vec![2,3])
```

Note that there is not instructions that store more than one operand at a time in the XiaoXuan Core ISA, if an instruction (such as 'call') returns more than one operand, you'll need to call the "store" instructon multiple times to store all the return values.

Of course, if there is only one operand on the stack, the return value of this function is NULL.

### Local Loading/Storing Extension

```rust
local_load_extend_i64(  identifier, offset:i64) -> i64
local_load_extend_i32_s(identifier, offset:i64) -> i32
local_load_extend_i32_u(identifier, offset:i64) -> i32
local_load_extend_i16_s(identifier, offset:i64) -> i16
local_load_extend_i16_u(identifier, offset:i64) -> i16
local_load_extend_i8_s( identifier, offset:i64) -> i8
local_load_extend_i8_u( identifier, offset:i64) -> i8
local_load_extend_f64(  identifier, offset:i64) -> f64
local_load_extend_f32(  identifier, offset:i64) -> f32

local_store_extend_i64(identifier, offset:i64, value:i64) -> (remain_values)
local_store_extend_i32(identifier, offset:i64, value:i32) -> (remain_values)
local_store_extend_i16(identifier, offset:i64, value:i16) -> (remain_values)
local_store_extend_i8( identifier, offset:i64, value:i8) -> (remain_values)
local_store_extend_f64(identifier, offset:i64, value:f64) -> (remain_values)
local_store_extend_f32(identifier, offset:i64, value:f32) -> (remain_values)
```

### Data Loading/Storing

```rust
data_load_i64(  identifier, offset=imm_i16) -> i64
data_load_i32_s(identifier, offset=imm_i16) -> i32
data_load_i32_u(identifier, offset=imm_i16) -> i32
data_load_i16_s(identifier, offset=imm_i16) -> i16
data_load_i16_u(identifier, offset=imm_i16) -> i16
data_load_i8_s( identifier, offset=imm_i16) -> i8
data_load_i8_u( identifier, offset=imm_i16) -> i8
data_load_f32(  identifier, offset=imm_i16) -> f32
data_load_f64(  identifier, offset=imm_i16) -> f64

data_store_i64(identifier, value:i64, offset=imm_i16) -> (remain_values)
data_store_i32(identifier, value:i32, offset=imm_i16) -> (remain_values)
data_store_i16(identifier, value:i16, offset=imm_i16) -> (remain_values)
data_store_i8( identifier, value:i8,  offset=imm_i16) -> (remain_values)
data_store_f64(identifier, value:f64, offset=imm_i16) -> (remain_values)
data_store_f32(identifier, value:f32, offset=imm_i16) -> (remain_values)
```

The `identifier` argument is the name of data, note that name path is not allowed.

### Data Loading/Storing Extension

```rust
data_load_extend_i64(  identifier, offset:i64) -> i64
data_load_extend_i32_s(identifier, offset:i64) -> i32
data_load_extend_i32_u(identifier, offset:i64) -> i32
data_load_extend_i16_s(identifier, offset:i64) -> i16
data_load_extend_i16_u(identifier, offset:i64) -> i16
data_load_extend_i8_s( identifier, offset:i64) -> i8
data_load_extend_i8_u( identifier, offset:i64) -> i8
data_load_extend_f32(  identifier, offset:i64) -> f32
data_load_extend_f64(  identifier, offset:i64) -> f64

data_store_extend_i64(identifier, offset:i64, value:i64) -> (remain_values)
data_store_extend_i32(identifier, offset:i64, value:i32) -> (remain_values)
data_store_extend_i16(identifier, offset:i64, value:i16) -> (remain_values)
data_store_extend_i8( identifier, offset:i64, value:i8 ) -> (remain_values)
data_store_extend_f64(identifier, offset:i64, value:f64) -> (remain_values)
data_store_extend_f32(identifier, offset:i64, value:f32) -> (remain_values)
```

### Memory Loading/Storing

```rust
memory_load_i64(  addr:i64, offset=imm_i16) -> i64
memory_load_i32_s(addr:i64, offset=imm_i16) -> i32
memory_load_i32_u(addr:i64, offset=imm_i16) -> i32
memory_load_i16_s(addr:i64, offset=imm_i16) -> i16
memory_load_i16_u(addr:i64, offset=imm_i16) -> i16
memory_load_i8_s( addr:i64, offset=imm_i16) -> i8
memory_load_i8_u( addr:i64, offset=imm_i16) -> i8
memory_load_f32(  addr:i64, offset=imm_i16) -> f32
memory_load_f64(  addr:i64, offset=imm_i16) -> f64

memory_store_i64(addr:i64, value:i64, offset=imm_i16) -> (remain_values)
memory_store_i32(addr:i64, value:i32, offset=imm_i16) -> (remain_values)
memory_store_i16(addr:i64, value:i16, offset=imm_i16) -> (remain_values)
memory_store_i8( addr:i64, value:i8,  offset=imm_i16) -> (remain_values)
memory_store_f64(addr:i64, value:f64, offset=imm_i16) -> (remain_values)
memory_store_f32(addr:i64, value:f32, offset=imm_i16) -> (remain_values)
```

### Memory Management

```rust
memory_fill(addr:i64, value:i8, count:i64) -> ()
memory_copy(dst_addr:i64, src_addr:i64, count:i64) -> ()
memory_capacity() -> i64
memory_resize(pages:i64) -> i64
```

### Conversion

```rust
truncate_i64_to_i32(number:i64) -> i32
extend_i32_s_to_i64(number:i32) -> i64
extend_i32_u_to_i64(number:i32) -> i64
demote_f64_to_f32(number:f64) -> f32
promote_f32_to_f64(number:f32) -> f64
```

Convert float to int

```rust
convert_f32_to_i32_s(number:f32) -> i32
convert_f32_to_i32_u(number:f32) -> i32
convert_f64_to_i32_s(number:f64) -> i32
convert_f64_to_i32_u(number:f64) -> i32
convert_f32_to_i64_s(number:f32) -> i64
convert_f32_to_i64_u(number:f32) -> i64
convert_f64_to_i64_s(number:f64) -> i64
convert_f64_to_i64_u(number:f64) -> i64
```

Convert int to float

```rust
convert_i32_s_to_f32(number:i32) -> f32
convert_i32_u_to_f32(number:i32) -> f32
convert_i64_s_to_f32(number:i64) -> f32
convert_i64_u_to_f32(number:i64) -> f32
convert_i32_s_to_f64(number:i32) -> f64
convert_i32_u_to_f64(number:i32) -> f64
convert_i64_s_to_f64(number:i64) -> f64
convert_i64_u_to_f64(number:i64) -> f64
```

### Comparison

```rust
eqz_i32(number:i32) -> i64
nez_i32(number:i32) -> i64
eq_i32(left:i32 right:i32) -> i64
ne_i32(left:i32 right:i32) -> i64
lt_i32_s(left:i32 right:i32) -> i64
lt_i32_u(left:i32 right:i32) -> i64
gt_i32_s(left:i32 right:i32) -> i64
gt_i32_u(left:i32 right:i32) -> i64
le_i32_s(left:i32 right:i32) -> i64
le_i32_u(left:i32 right:i32) -> i64
ge_i32_s(left:i32 right:i32) -> i64
ge_i32_u(left:i32 right:i32) -> i64
```

```rust
eqz_i64(number:i64) -> i64
nez_i64(number:i64) -> i64
eq_i64(left:i64 right:i64) -> i64
ne_i64(left:i64 right:i64) -> i64
lt_i64_s(left:i64 right:i64) -> i64
lt_i64_u(left:i64 right:i64) -> i64
gt_i64_s(left:i64 right:i64) -> i64
gt_i64_u(left:i64 right:i64) -> i64
le_i64_s(left:i64 right:i64) -> i64
le_i64_u(left:i64 right:i64) -> i64
ge_i64_s(left:i64 right:i64) -> i64
ge_i64_u(left:i64 right:i64) -> i64
```

```rust
eq_f32(left:f32 right:f32) -> i64
ne_f32(left:f32 right:f32) -> i64
lt_f32(left:f32 right:f32) -> i64
gt_f32(left:f32 right:f32) -> i64
le_f32(left:f32 right:f32) -> i64
ge_f32(left:f32 right:f32) -> i64
eq_f64(left:f64 right:f64) -> i64
ne_f64(left:f64 right:f64) -> i64
lt_f64(left:f64 right:f64) -> i64
gt_f64(left:f64 right:f64) -> i64
le_f64(left:f64 right:f64) -> i64
ge_f64(left:f64 right:f64) -> i64
```

### Arithmetic

```rust
add_i32(left:i32 right:i32) -> i32
sub_i32(left:i32 right:i32) -> i32
add_imm_i32(imm:imm_i16, number:i32) -> i32
sub_imm_i32(imm:imm_i16, number:i32) -> i32
mul_i32(left:i32 right:i32) -> i32
div_i32_s(left:i32 right:i32) -> i32
div_i32_u(left:i32 right:i32) -> i32
rem_i32_s(left:i32 right:i32) -> i32
rem_i32_u(left:i32 right:i32) -> i32
```

```rust
add_i64(left:i64 right:i64) -> i64
sub_i64(left:i64 right:i64) -> i64
add_imm_i64(imm:imm_i16, number:i64) -> i64
sub_imm_i64(imm:imm_i16, number:i64) -> i64
mul_i64(left:i64 right:i64) -> i64
div_i64_s(left:i64 right:i64) -> i64
div_i64_u(left:i64 right:i64) -> i64
rem_i64_s(left:i64 right:i64) -> i64
rem_i64_u(left:i64 right:i64) -> i64
```

```rust
add_f32(left:f32 right:f32) -> f32
sub_f32(left:f32 right:f32) -> f32
mul_f32(left:f32 right:f32) -> f32
div_f32(left:f32 right:f32) -> f32
add_f64(left:f64 right:f64) -> f64
sub_f64(left:f64 right:f64) -> f64
mul_f64(left:f64 right:f64) -> f64
div_f64(left:f64 right:f64) -> f64
```

### Bitwise

```rust
and(left:i64 right:i64) -> i64
or(left:i64 right:i64) -> i64
xor(left:i64 right:i64) -> i64
not(number:i64) -> i64
```

```rust
shift_left_i32(number:i32 move_bits:i32) -> i32
shift_right_i32_s(number:i32 move_bits:i32) -> i32
shift_right_i32_u(number:i32 move_bits:i32) -> i32
rotate_left_i32(number:i32 move_bits:i32) -> i32
rotate_right_i32(number:i32 move_bits:i32) -> i32
count_leading_zeros_i32(number:i32) -> i32
count_leading_ones_i32(number:i32) -> i32
count_trailing_zeros_i32(number:i32) -> i32
count_ones_i32(number:i32) -> i32
```

```rust
shift_left_i64(number:i64 move_bits:i32) -> i64
shift_right_i64_s(number:i64 move_bits:i32) -> i64
shift_right_i64_u(number:i64 move_bits:i32) -> i64
rotate_left_i64(number:i64 move_bits:i32) -> i64
rotate_right_i64(number:i64 move_bits:i32) -> i64
count_leading_zeros_i64(number:i64) -> i32
count_leading_ones_i64(number:i64) -> i32
count_trailing_zeros_i64(number:i64) -> i32
count_ones_i64(number:i64) -> i32
```

### Math

```rust
abs_i32(number:i32) -> i32
neg_i32(number:i32) -> i32
abs_i64(number:i64) -> i64
neg_i64(number:i64) -> i64
```

```rust
abs_f32(number:f32) -> f32
neg_f32(number:f32) -> f32
copysign_f32(num:f32, sign:f32) -> f32
sqrt_f32(number:f32) -> f32
min_f32(left:f32 right:f32) -> f32
max_f32(left:f32 right:f32) -> f32
ceil_f32(number:f32) -> f32
floor_f32(number:f32) -> f32
round_half_away_from_zero_f32(number:f32) -> f32
round_half_to_even_f32(number:f32) -> f32
trunc_f32(number:f32) -> f32
fract_f32(number:f32) -> f32
cbrt_f32(number:f32) -> f32
exp_f32(number:f32) -> f32
exp2_f32(number:f32) -> f32
ln_f32(number:f32) -> f32
log2_f32(number:f32) -> f32
log10_f32(number:f32) -> f32
sin_f32(number:f32) -> f32
cos_f32(number:f32) -> f32
tan_f32(number:f32) -> f32
asin_f32(number:f32) -> f32
acos_f32(number:f32) -> f32
atan_f32(number:f32) -> f32
pow_f32(left:f32 right:f32) -> f32
log_f32(left:f32 right:f32) -> f32
```

```rust
abs_f64(number:f64) -> f64
neg_f64(number:f64) -> f64
copysign_f64(num:f64, sign:f64) -> f64
sqrt_f64(number:f64) -> f64
min_f64(left:f64 right:f64) -> f64
max_f64(left:f64 right:f64) -> f64
ceil_f64(number:f64) -> f64
floor_f64(number:f64) -> f64
round_half_away_from_zero_f64(number:f64) -> f64
round_half_to_even_f64(number:f64) -> f64
trunc_f64(number:f64) -> f64
fract_f64(number:f64) -> f64
cbrt_f64(number:f64) -> f64
exp_f64(number:f64) -> f64
exp2_f64(number:f64) -> f64
ln_f64(number:f64) -> f64
log2_f64(number:f64) -> f64
log10_f64(number:f64) -> f64
sin_f64(number:f64) -> f64
cos_f64(number:f64) -> f64
tan_f64(number:f64) -> f64
asin_f64(number:f64) -> f64
acos_f64(number:f64) -> f64
atan_f64(number:f64) -> f64
pow_f64(left:f64 right:f64) -> f64
log_f64(left:f64 right:f64) -> f64
```

### Calling

- `call(identifier, value0, value1, ...) -> (values)`
   call a function

- `extcall(identifier, value0, value1, ...) -> return_value:void/i32/i64/f32/f64`
   call a external function

- `envcall(env_call_number:liter_i32, value0, value1, ...) -> (values)`
   environment call

- `syscall(syscall_num:i32, value0, value1, ...) -> (return_value:i64, error_no:i32)`
   system call

- `get_function(identifier) -> i32`
   get the public index of the specified function

- `dyncall(fn_pub_index:i32, value0, value1, ...) -> (values)`
   dynamic call

The `identifier` argument is the name of function, note that name path is not allowed.

<!--
- The name of function or data.
- A relative name path, e.g. "sub_module::some_func".
- A relative name path starts with identifier imported by `use` statements.
- A full name, e.g. "module_name::sub_module::some_data".
-->

The argument values of `*call` instruction expression can be the return values of other instructions, or other functions or groups, as long as they have the same number of arguments. For example, if a function takes three arguments, it can composed of an instruction that returns one value and a function call that returns two values, e.g.:

```rust
call(fn_with_2_args
    local_load_i32_u(left)
    local_load_i32_u(right)
)

call(fn_with_3_args
    local_load_i32_u(init)
    call(fn_return_2_values)
)
```

If a function call returns more than the number of arguments needed for a function, or in a different order, a temporary local variable can be used to discard or swap some of the return values.

Discard some return values:

```rust
call(fn_with_1_args
    // discard the last return value
    local_store_i32(
        trash   // The `trash` is a local variable
        call(fn_return_2_values)
    )
)
```

Swap return values:

```rust
// store the return values to cache
local_store_i32(left,
    local_store_i32(right,
        call(fn_return_2_values)
    )
)

call(fn_with_2_args
    // load cache in reverse order
    local_load_i32_u(right)
    local_load_i32_u(left)
)
```

### Host

```rust
host_addr_local(identifier, offset=imm_i16) -> i64
host_addr_local_extend(identifier, offset:i64) -> i64
host_addr_data(identifier, offset=imm_i16) -> i64
host_addr_data_extend(identifier, offset:i64) -> i64
host_addr_function(identifier) -> i64
```

```rust
panic(code:imm_i32) NEVER RETURN
host_addr_memory(addr:i64, offset=imm_i16) -> i64
host_copy_from_memory(dst_pointer:i64, src_addr:i64, count:i64) -> ()
host_copy_to_memory(dst_addr:i64, src_pointer:i64, count:i64) -> ()
host_external_memory_copy(dst_pointer:i64, src_pointer:i64, count:i64) -> ()
```
