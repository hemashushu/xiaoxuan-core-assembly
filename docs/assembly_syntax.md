# XiaoXuan Core Assembly Syntax

## Comments

- line comment: from the double semi-colon to the end of the line, e.g.
  ;; comment

- block comment: any block of text surround by '(;' and ';)' pair, nested block comments are allowed, e.g.
  (; block comment ;)
  (; one (; two ;);)

block comment has higher priority than line comment and node comment.

  (; one ;; line comments within the block comment are ignored ;)
  (; one #( node comments within the block comment are ignored) ;)

- node comment: a hash mark at the front of the left parenthesis, nested node comments are allowed, e.g.
  #(add 11 (mul 13 17))
  #(add 11 #(mul 13 17) (mul 19 23))

node comment has the lowest priority.

  #(add (; block comments are still valid ;) 11 13)
  #(add ;; line comments are still valid
    11 13)
  #(add (; note ;; line comments within the block comment are ignored ;) 11 13)

## Tokens

### Identifier

'$' + /a-zA-Z0-9_/+, should not starts with number, e.g.

$add, $some_func, $print2

### Symbol

/a-zA-Z0-9_./+, should not starts with number, e.g.

local, i32, i32.imm, i32.div_s, user

### Number

supportes decimal, binary, hexadecimal and float point numbers e.g.
211, 0x11, 0x11_22, 0b1100, 3.14, 2.99e8, +12, -3.14

invalid number: -0xaabb, -0b1100, 0xaa.bb, 0b10.01

floating point numbers can be written as HEX, it's the little-endian bytes in the memory, do not confuse with the C floating-point literal (https://en.cppreference.com/w/cpp/language/floating_literal)

### String

a char sequence surround by double quotes, multiline is supported. e.g.

- "abc文字😊", "\t\r\n\\\""\u{2d}\u{6587}\0"
- "line 0
   line 1"

new line without 'Lf' and leading space chars.
- "line 0 \
   line 1"

#### raw string

r"line 0
line 1
line2"

#### indented string

"""
line 0
line 1
line 2
"""

        """
        auto trim leading space
        line 0
        line 1
        line 2
        """

supported escape chars:

- \\, escape char itself
- \", doube quote
- \t, horizontal tabulation
- \r, carriage return (CR)
- \n, new line character (line feed, LF)
- \0, null char
- \u{...}, unicode code point, e.g. '\u{2d}', '\u{6587}'

unsupported escape chars:

- \v, vertical tabulation
- \f, page breaking control character
- \x.., byte escape

### Byte data

a char sequence surrounded by char 'd' and double quotes, two hex digital number per byte, chars / -:\t\r\n/ are ignored, e.g.

d"0011aabb",
d"00 11 AA BB",
d"00-11-aa-bb",
d"00:11:aa:bb",
d"00 11 22 33
  44 55 66 77"
