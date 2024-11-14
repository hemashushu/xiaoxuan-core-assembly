# Instructions

## Base

```rust
nop()  ->  ()
```

## Immediately Numbers

```rust
imm_i32(literal_i32) -> i32
imm_i64(literal_i64) -> i64
imm_f32(literal_f32) -> f32
imm_f64(literal_f64) -> f64
```

## Local Loading/Storing

```rust
local_load_i64(  identifier, rindex=literal_i16, offset=literal_i16)  ->  i64
local_load_i32_s(identifier, rindex=literal_i16, offset=literal_i16)  ->  i32
local_load_i32_u(identifier, rindex=literal_i16, offset=literal_i16)  ->  i32
local_load_i16_s(identifier, rindex=literal_i16, offset=literal_i16)  ->  i16
local_load_i16_u(identifier, rindex=literal_i16, offset=literal_i16)  ->  i16
local_load_i8_s( identifier, rindex=literal_i16, offset=literal_i16)  ->  i8
local_load_i8_u( identifier, rindex=literal_i16, offset=literal_i16)  ->  i8
local_load_f32(  identifier, rindex=literal_i16, offset=literal_i16)  ->  f32
local_load_f64(  identifier, rindex=literal_i16, offset=literal_i16)  ->  f64

local_store_i64(identifier, value:i64, rindex=literal_i16, offset=literal_i16)  ->  ()
local_store_i32(identifier, value:i32, rindex=literal_i16, offset=literal_i16)  ->  ()
local_store_i16(identifier, value:i16, rindex=literal_i16, offset=literal_i16)  ->  ()
local_store_i8( identifier, value:i8,  rindex=literal_i16, offset=literal_i16)  ->  ()
local_store_f64(identifier, value:f64, rindex=literal_i16, offset=literal_i16)  ->  ()
local_store_f32(identifier, value:f32, rindex=literal_i16, offset=literal_i16)  ->  ()
```

## Local Loading/Storing Extension

```rust
local_load_extend_i64(  identifier, offset:i64, rindex=literal_i16)  ->  i64
local_load_extend_i32_s(identifier, offset:i64, rindex=literal_i16)  ->  i32
local_load_extend_i32_u(identifier, offset:i64, rindex=literal_i16)  ->  i32
local_load_extend_i16_s(identifier, offset:i64, rindex=literal_i16)  ->  i16
local_load_extend_i16_u(identifier, offset:i64, rindex=literal_i16)  ->  i16
local_load_extend_i8_s( identifier, offset:i64, rindex=literal_i16)  ->  i8
local_load_extend_i8_u( identifier, offset:i64, rindex=literal_i16)  ->  i8
local_load_extend_f64(  identifier, offset:i64, rindex=literal_i16)  ->  f64
local_load_extend_f32(  identifier, offset:i64, rindex=literal_i16)  ->  f32

local_store_extend_i64(identifier, offset:i64, value:i64, rindex=literal_i16)  ->  ()
local_store_extend_i32(identifier, offset:i64, value:i32, rindex=literal_i16)  ->  ()
local_store_extend_i16(identifier, offset:i64, value:i16, rindex=literal_i16)  ->  ()
local_store_extend_i8( identifier, offset:i64, value:i8,  rindex=literal_i16)  ->  ()
local_store_extend_f64(identifier, offset:i64, value:f64, rindex=literal_i16)  ->  ()
local_store_extend_f32(identifier, offset:i64, value:f32, rindex=literal_i16)  ->  ()
```

## Data Loading/Storing

```rust
data_load_i64(  identifier, offset=literal_i16)  ->  i64
data_load_i32_s(identifier, offset=literal_i16)  ->  i32
data_load_i32_u(identifier, offset=literal_i16)  ->  i32
data_load_i16_s(identifier, offset=literal_i16)  ->  i16
data_load_i16_u(identifier, offset=literal_i16)  ->  i16
data_load_i8_s( identifier, offset=literal_i16)  ->  i8
data_load_i8_u( identifier, offset=literal_i16)  ->  i8
data_load_f32(  identifier, offset=literal_i16)  ->  f32
data_load_f64(  identifier, offset=literal_i16)  ->  f64

data_store_i64(identifier, value:i64, offset=literal_i16)  ->  ()
data_store_i32(identifier, value:i32, offset=literal_i16)  ->  ()
data_store_i16(identifier, value:i16, offset=literal_i16)  ->  ()
data_store_i8( identifier, value:i8,  offset=literal_i16)  ->  ()
data_store_f64(identifier, value:f64, offset=literal_i16)  ->  ()
data_store_f32(identifier, value:f32, offset=literal_i16)  ->  ()
```

## Data Loading/Storing Extension

```rust
data_load_extend_i64(  identifier, offset:i64)  ->  i64
data_load_extend_i32_s(identifier, offset:i64)  ->  i32
data_load_extend_i32_u(identifier, offset:i64)  ->  i32
data_load_extend_i16_s(identifier, offset:i64)  ->  i16
data_load_extend_i16_u(identifier, offset:i64)  ->  i16
data_load_extend_i8_s( identifier, offset:i64)  ->  i8
data_load_extend_i8_u( identifier, offset:i64)  ->  i8
data_load_extend_f32(  identifier, offset:i64)  ->  f32
data_load_extend_f64(  identifier, offset:i64)  ->  f64

data_store_extend_i64(identifier, offset:i64, value:i64)  ->  ()
data_store_extend_i32(identifier, offset:i64, value:i32)  ->  ()
data_store_extend_i16(identifier, offset:i64, value:i16)  ->  ()
data_store_extend_i8( identifier, offset:i64, value:i8 )  ->  ()
data_store_extend_f64(identifier, offset:i64, value:f64)  ->  ()
data_store_extend_f32(identifier, offset:i64, value:f32)  ->  ()
```

## Heap Loading/Storing

```rust
heap_load_i64(  addr:i64, offset=literal_i16)  ->  i64
heap_load_i32_s(addr:i64, offset=literal_i16)  ->  i32
heap_load_i32_u(addr:i64, offset=literal_i16)  ->  i32
heap_load_i16_s(addr:i64, offset=literal_i16)  ->  i16
heap_load_i16_u(addr:i64, offset=literal_i16)  ->  i16
heap_load_i8_s( addr:i64, offset=literal_i16)  ->  i8
heap_load_i8_u( addr:i64, offset=literal_i16)  ->  i8
heap_load_f32(  addr:i64, offset=literal_i16)  ->  f32
heap_load_f64(  addr:i64, offset=literal_i16)  ->  f64

heap_store_i64(addr:i64, value:i64, offset=literal_i16)  ->  ()
heap_store_i32(addr:i64, value:i32, offset=literal_i16)  ->  ()
heap_store_i16(addr:i64, value:i16, offset=literal_i16)  ->  ()
heap_store_i8( addr:i64, value:i8,  offset=literal_i16)  ->  ()
heap_store_f64(addr:i64, value:f64, offset=literal_i16)  ->  ()
heap_store_f32(addr:i64, value:f32, offset=literal_i16)  ->  ()
```

```rust
heap_fill(addr:i64, value:i8, count:i64)  ->  ()
heap_copy(dst_addr:i64, src_addr:i64, count:i64)  ->  ()
heap_capacity()  ->  i64
heap_resize(pages:i64)  ->  i64
```

## Conversion

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

## Comparison

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

## Arithmetic

```rust
add_i32(left:i32 right:i32) -> i32
sub_i32(left:i32 right:i32) -> i32
add_imm_i32(imm:literal_i16, number:i32) -> i32
sub_imm_i32(imm:literal_i16, number:i32) -> i32
mul_i32(left:i32 right:i32) -> i32
div_i32_s(left:i32 right:i32) -> i32
div_i32_u(left:i32 right:i32) -> i32
rem_i32_s(left:i32 right:i32) -> i32
rem_i32_u(left:i32 right:i32) -> i32
```

```rust
add_i64(left:i64 right:i64) -> i64
sub_i64(left:i64 right:i64) -> i64
add_imm_i64(imm:literal_i16, number:i64) -> i64
sub_imm_i64(imm:literal_i16, number:i64) -> i64
mul_i64(left:i64 right:i64) -> i64
div_i64_s(left:i64 right:i64) -> i64
div_i64_u(left:i64 right:i64) -> i64
rem_i64_s(left:i64 right:i64) -> i64
rem_i64_u(left:i64 right:i64) -> i64
```

```rust
add_f32(left:f32 right:f32)  ->  f32
sub_f32(left:f32 right:f32)  ->  f32
mul_f32(left:f32 right:f32)  ->  f32
div_f32(left:f32 right:f32)  ->  f32
add_f64(left:f64 right:f64)  ->  f64
sub_f64(left:f64 right:f64)  ->  f64
mul_f64(left:f64 right:f64)  ->  f64
div_f64(left:f64 right:f64)  ->  f64
```

## Bitwise

```rust
and(left:i64 right:i64)  ->  i64
or(left:i64 right:i64)  ->  i64
xor(left:i64 right:i64)  ->  i64
not(number:i64)  ->  i64
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

## Math

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

## Calling

- `call(identifier, value0, value1, ...)`
   call a function
- `dyncall(fn_pub_index:i32, value0, value1, ...)`
   dynamic call
- `envcall(env_call_number:i32, value0, value1, ...)`
   environment call
- `syscall(sys_call_number:i32, value0, value1, ...)`
   syscall
- `extcall(identifier, value0, value1, ...)`

The arguments to `*call` can be the return values of other instructions, or other functions or groups, as long as they have the same number of arguments. For example, if a function takes three arguments, it can composed of an instruction that returns one value and a function call that returns two values, e.g.:

```rust
call(i_need_2_args
    local_load_i32_u(left)
    local_load_i32_u(right)
)

call(i_need_3_args
    local_load_i32_u(init)
    call(i_return_2_values)
)
```

If a function call returns more than the number of arguments needed for a function, or in a different order, some method (such as using a local variable) is used to discard or swap some of the return values.

```rust
call(i_need_1_args
    // discard the last return value
    local_store_i32(
        trash   // The `trash` is a local variable
        call(i_return_2_values)
    )
)

// store the return values to cache
local_store_i32(left,
    local_store_i32(right,
        call(i_return_2_values)
    )
)

// load cache in reverse order
call(i_need_2_args
    local_load_i32_u(right)
    local_load_i32_u(left)
)
```

## Host

```rust
panic(code:literal_i32)  ->  (never return)
host_addr_local(identifier, rindex=literal_i16, offset=literal_i16) -> i64
host_addr_local_extend(identifier, offset:i64, rindex=literal_i16) -> i64
host_addr_data(identifier, offset=literal_i16) -> i64
host_addr_data_extend(identifier, offset:i64) -> i64
host_addr_heap(addr:i64, offset=literal_i16) -> i64
host_addr_function(identifier) -> i64
host_copy_heap_to_memory(dst_pointer:i64, src_addr:i64, count:i64) -> ()
host_copy_memory_to_heap(dst_addr:i64, src_pointer:i64, count:i64) -> ()
host_memory_copy(dst_pointer:i64, src_pointer:i64, count:i64) -> ()
```
