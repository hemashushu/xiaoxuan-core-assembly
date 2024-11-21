// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

pub const LEXER_PEEK_CHAR_MAX_COUNT: usize = 3;

use crate::{
    charposition::{CharWithPosition, CharsWithPositionIter},
    error::Error,
    location::Location,
    peekableiter::PeekableIter,
    token::{NumberToken, NumberType},
};

use super::token::{Comment, Token, TokenWithRange};

pub fn lex_from_str(s: &str) -> Result<Vec<TokenWithRange>, Error> {
    let mut chars = s.chars();
    let mut char_position_iter = CharsWithPositionIter::new(0, &mut chars);
    let mut peekable_char_position_iter =
        PeekableIter::new(&mut char_position_iter, LEXER_PEEK_CHAR_MAX_COUNT);
    let mut lexer = Lexer::new(&mut peekable_char_position_iter);
    lexer.lex()
}

struct Lexer<'a> {
    upstream: &'a mut PeekableIter<'a, CharWithPosition>,
    last_position: Location,
    saved_positions: Vec<Location>,
}

impl<'a> Lexer<'a> {
    fn new(upstream: &'a mut PeekableIter<'a, CharWithPosition>) -> Self {
        Self {
            upstream,
            last_position: Location::new_position(0, 0, 0, 0),
            saved_positions: vec![],
        }
    }

    fn next_char(&mut self) -> Option<char> {
        match self.upstream.next() {
            Some(CharWithPosition {
                character,
                position,
            }) => {
                self.last_position = position;
                Some(character)
            }
            None => None,
        }
    }

    fn peek_char(&self, offset: usize) -> Option<&char> {
        match self.upstream.peek(offset) {
            Some(CharWithPosition { character, .. }) => Some(character),
            None => None,
        }
    }

    fn peek_char_and_equals(&self, offset: usize, expected_char: char) -> bool {
        matches!(
            self.upstream.peek(offset),
            Some(CharWithPosition { character, .. }) if character == &expected_char)
    }

    fn peek_position(&self, offset: usize) -> Option<&Location> {
        match self.upstream.peek(offset) {
            Some(CharWithPosition { position, .. }) => Some(position),
            None => None,
        }
    }

    fn push_peek_position(&mut self) {
        self.saved_positions.push(*self.peek_position(0).unwrap());
    }

    fn pop_saved_position(&mut self) -> Location {
        self.saved_positions.pop().unwrap()
    }
}

impl<'a> Lexer<'a> {
    fn lex(&mut self) -> Result<Vec<TokenWithRange>, Error> {
        let mut token_with_ranges = vec![];

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                ' ' | '\t' => {
                    self.next_char(); // consume whitespace
                }
                '\r' if self.peek_char_and_equals(1, '\n') => {
                    self.push_peek_position();

                    self.next_char(); // consume '\r'
                    self.next_char(); // consume '\n'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::NewLine,
                        &self.pop_saved_position(),
                        2,
                    ));
                }
                '\n' => {
                    self.next_char(); // consume '\n'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::NewLine,
                        &self.last_position,
                        1,
                    ));
                }
                '#' if self.peek_char_and_equals(1, '!') => {
                    // shebang
                    self.consume_shebang();
                }
                ',' => {
                    self.next_char(); // consume ','

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Comma,
                        &self.last_position,
                        1,
                    ));
                }
                ':' => {
                    self.next_char(); // consume ':'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Colon,
                        &self.last_position,
                        1,
                    ));
                }
                '=' => {
                    self.next_char(); // consume '='

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Equal,
                        &self.last_position,
                        1,
                    ));
                }
                '-' if self.peek_char_and_equals(1, '>') => {
                    self.push_peek_position();

                    self.next_char(); // consume '-'
                    self.next_char(); // consume '>'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::RightArrow,
                        &self.pop_saved_position(),
                        2,
                    ));
                }
                '-' => {
                    self.next_char(); // consume '-'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Minus,
                        &self.last_position,
                        1,
                    ));
                }
                '+' => {
                    self.next_char(); // consume '+'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::Plus,
                        &self.last_position,
                        1,
                    ));
                }
                '{' => {
                    self.next_char(); // consume '{'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::LeftBrace,
                        &self.last_position,
                        1,
                    ));
                }
                '}' => {
                    self.next_char(); // consume '}'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::RightBrace,
                        &self.last_position,
                        1,
                    ))
                }
                '[' => {
                    self.next_char(); // consume '['

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::LeftBracket,
                        &self.last_position,
                        1,
                    ));
                }
                ']' => {
                    self.next_char(); // consume ']'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::RightBracket,
                        &self.last_position,
                        1,
                    ));
                }
                '(' => {
                    self.next_char(); // consume '('

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::LeftParen,
                        &self.last_position,
                        1,
                    ));
                }
                ')' => {
                    self.next_char(); // consume ')'

                    token_with_ranges.push(TokenWithRange::from_position_and_length(
                        Token::RightParen,
                        &self.last_position,
                        1,
                    ))
                }
                '0'..='9' => {
                    // number
                    token_with_ranges.push(self.lex_number()?);
                }
                'h' if self.peek_char_and_equals(1, '"') => {
                    // hex byte data
                    token_with_ranges.push(self.lex_byte_data_hexadecimal()?);
                }
                'r' if self.peek_char_and_equals(1, '"') => {
                    // raw string
                    token_with_ranges.push(self.lex_raw_string()?);
                }
                'r' if self.peek_char_and_equals(1, '#') && self.peek_char_and_equals(2, '"') => {
                    // raw string with hash symbol
                    token_with_ranges.push(self.lex_raw_string_with_hash_symbol()?);
                }
                '"' => {
                    // string
                    let twr =
                        if self.peek_char_and_equals(1, '"') && self.peek_char_and_equals(2, '"') {
                            // auto-trimmed string
                            self.lex_auto_trimmed_string()?
                        } else {
                            // normal string
                            self.lex_string()?
                        };

                    token_with_ranges.push(twr);
                }
                // '\'' => {
                //     // char
                //     token_with_ranges.push(self.lex_char()?);
                // }
                '/' if self.peek_char_and_equals(1, '/') => {
                    // line comment
                    token_with_ranges.push(self.lex_line_comment()?);
                }
                '/' if self.peek_char_and_equals(1, '*') => {
                    // block comment
                    token_with_ranges.push(self.lex_block_comment()?);
                }
                'a'..='z' | 'A'..='Z' | '_' | '\u{a0}'..='\u{d7ff}' | '\u{e000}'..='\u{10ffff}' => {
                    // identifier/name/keyword
                    token_with_ranges.push(self.lex_identifier()?);
                }
                current_char => {
                    return Err(Error::MessageWithLocation(
                        format!("Unexpected char '{}'.", current_char),
                        *self.peek_position(0).unwrap(),
                    ));
                }
            }
        }

        Ok(token_with_ranges)
    }

    fn consume_shebang(&mut self) {
        // #!...\n?  //
        // ^^     ^__// to here
        // ||________// current char, validated
        //
        // current char = the character of `iter.upstream.peek(0)``

        while let Some(current_char) = self.next_char() {
            if current_char == '\n' {
                break;
            }
        }
    }

    fn lex_identifier(&mut self) -> Result<TokenWithRange, Error> {
        // key_nameT  //
        // ^       ^__// to here
        // |__________// current char, validated
        //
        // current char = the character of `iter.upstream.peek(0)``
        // T = terminator chars || EOF

        let mut name_string = String::new();
        let mut found_double_colon = false; // to indicate whether the variant separator "::" is found

        self.push_peek_position();

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' => {
                    name_string.push(*current_char);
                    self.next_char(); // consume char
                }
                ':' if self.peek_char_and_equals(1, ':') => {
                    found_double_colon = true;
                    name_string.push_str("::");
                    self.next_char(); // consume the 1st ":"
                    self.next_char(); // consume the 2nd ":"
                }
                '\u{a0}'..='\u{d7ff}' | '\u{e000}'..='\u{10ffff}' => {
                    // A char is a ‘Unicode scalar value’, which is any ‘Unicode code point’ other than a surrogate code point.
                    // This has a fixed numerical definition: code points are in the range 0 to 0x10FFFF,
                    // inclusive. Surrogate code points, used by UTF-16, are in the range 0xD800 to 0xDFFF.
                    //
                    // check out:
                    // https://doc.rust-lang.org/std/primitive.char.html
                    //
                    // CJK chars: '\u{4e00}'..='\u{9fff}'
                    // for complete CJK chars, check out Unicode standard
                    // Ch. 18.1 Han CJK Unified Ideographs
                    //
                    // summary:
                    // Block Location Comment
                    // CJK Unified Ideographs 4E00–9FFF Common
                    // CJK Unified Ideographs Extension A 3400–4DBF Rare
                    // CJK Unified Ideographs Extension B 20000–2A6DF Rare, historic
                    // CJK Unified Ideographs Extension C 2A700–2B73F Rare, historic
                    // CJK Unified Ideographs Extension D 2B740–2B81F Uncommon, some in current use
                    // CJK Unified Ideographs Extension E 2B820–2CEAF Rare, historic
                    // CJK Unified Ideographs Extension F 2CEB0–2EBEF Rare, historic
                    // CJK Unified Ideographs Extension G 30000–3134F Rare, historic
                    // CJK Unified Ideographs Extension H 31350–323AF Rare, historic
                    // CJK Compatibility Ideographs F900–FAFF Duplicates, unifiable variants, corporate characters
                    // CJK Compatibility Ideographs Supplement 2F800–2FA1F Unifiable variants
                    //
                    // https://www.unicode.org/versions/Unicode15.0.0/ch18.pdf
                    // https://en.wikipedia.org/wiki/CJK_Unified_Ideographs
                    // https://www.unicode.org/versions/Unicode15.0.0/
                    //
                    // see also
                    // https://www.unicode.org/reports/tr31/tr31-37.html

                    name_string.push(*current_char);
                    self.next_char(); // consume char
                }
                ' ' | '\t' | '\r' | '\n' | ',' | ':' | '=' | '+' | '-' | '{' | '}' | '[' | ']'
                | '(' | ')' | '/' | '"' => {
                    // terminator chars
                    break;
                }
                _ => {
                    return Err(Error::MessageWithLocation(
                        format!("Invalid char '{}' for identifier.", current_char),
                        *self.peek_position(0).unwrap(),
                    ));
                }
            }
        }

        let name_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        let token = if found_double_colon {
            Token::NamePath(name_string)
        } else {
            match name_string.as_str() {
                "use" | "as" | "external" | "fn" | "data" | "pub" | "readonly" | "uninit" | "align" => {
                    Token::Keyword(name_string)
                }
                "i64" | "i32" | "f64" | "f32" | "byte" => Token::DataTypeName(name_string),
                _ => Token::Name(name_string),
            }
        };

        Ok(TokenWithRange::new(token, name_range))
    }

    fn lex_number(&mut self) -> Result<TokenWithRange, Error> {
        // 123456T  //
        // ^     ^__// to here
        // |________// current char, validated
        //
        // T = terminator chars || EOF

        if self.peek_char_and_equals(0, '0') && self.peek_char_and_equals(1, 'b') {
            // '0b...'
            self.lex_number_binary()
        } else if self.peek_char_and_equals(0, '0') && self.peek_char_and_equals(1, 'x') {
            // '0x...'
            self.lex_number_hex()
        } else {
            // '123'
            self.lex_number_decimal()
        }
    }

    fn lex_number_decimal(&mut self) -> Result<TokenWithRange, Error> {
        // 123456T  //
        // ^     ^__// to here
        // |________// current char, validated
        //
        // T = terminator chars || EOF

        let mut num_string = String::new();
        let mut num_type: Option<NumberType> = None; // "_ixx", "_uxx", "_fxx"
        let mut found_point = false; // to indicated whether char '.' is found
        let mut found_e = false; // to indicated whether char 'e' is found

        // samples:
        //
        // 123
        // 3.14
        // 2.99e8
        // 2.99e+8
        // 6.672e-34

        self.push_peek_position();

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                '0'..='9' => {
                    // valid digits for decimal number
                    num_string.push(*current_char);

                    self.next_char(); // consume digit
                }
                '_' => {
                    self.next_char(); // consume '_'
                }
                '.' if !found_point => {
                    found_point = true;
                    num_string.push(*current_char);

                    self.next_char(); // consume '.'
                }
                'e' if !found_e => {
                    found_e = true;

                    // 123e45
                    // 123e+45
                    // 123e-45
                    if self.peek_char_and_equals(1, '-') {
                        num_string.push_str("e-");
                        self.next_char(); // consume 'e'
                        self.next_char(); // consume '-'
                    } else if self.peek_char_and_equals(1, '+') {
                        num_string.push_str("e+");
                        self.next_char(); // consume 'e'
                        self.next_char(); // consume '+'
                    } else {
                        num_string.push(*current_char);
                        self.next_char(); // consume 'e'
                    }
                }
                'i' | 'f' if num_type.is_none() && matches!(self.peek_char(1), Some('0'..='9')) => {
                    let nt = self.lex_number_type_suffix()?;
                    num_type.replace(nt);
                    break;
                }
                ' ' | '\t' | '\r' | '\n' | ',' | ':' | '=' | '+' | '-' | '{' | '}' | '[' | ']'
                | '(' | ')' | '/' | '"' => {
                    // terminator chars
                    break;
                }
                _ => {
                    return Err(Error::MessageWithLocation(
                        format!("Invalid char '{}' for decimal number.", current_char),
                        *self.peek_position(0).unwrap(),
                    ));
                }
            }
        }

        // check syntax
        if num_string.ends_with('.') {
            return Err(Error::MessageWithLocation(
                "Decimal number can not ends with \".\".".to_owned(),
                self.last_position,
            ));
        }

        if num_string.ends_with('e') {
            return Err(Error::MessageWithLocation(
                "Decimal number can not ends with \"e\".".to_owned(),
                self.last_position,
            ));
        }

        let num_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        let num_token: NumberToken = if let Some(nt) = num_type {
            // numbers with explicit type
            match nt {
                NumberType::I8 => {
                    let v = num_string.parse::<u8>().map_err(|_| {
                        Error::MessageWithLocation(
                            format!("Can not convert \"{}\" to i8 integer number.", num_string),
                            num_range,
                        )
                    })?;

                    NumberToken::I8(v)
                }
                NumberType::I16 => {
                    let v = num_string.parse::<u16>().map_err(|_| {
                        Error::MessageWithLocation(
                            format!("Can not convert \"{}\" to i16 integer number.", num_string),
                            num_range,
                        )
                    })?;

                    NumberToken::I16(v)
                }
                NumberType::I32 => {
                    let v = num_string.parse::<u32>().map_err(|_| {
                        Error::MessageWithLocation(
                            format!("Can not convert \"{}\" to i32 integer number.", num_string),
                            num_range,
                        )
                    })?;

                    NumberToken::I32(v)
                }
                NumberType::I64 => {
                    let v = num_string.parse::<u64>().map_err(|_| {
                        Error::MessageWithLocation(
                            format!("Can not convert \"{}\" to i64 integer number.", num_string),
                            num_range,
                        )
                    })?;

                    NumberToken::I64(v)
                }
                NumberType::F32 => {
                    let v = num_string.parse::<f32>().map_err(|_| {
                        Error::MessageWithLocation(
                            format!(
                                "Can not convert \"{}\" to f32 floating-point number.",
                                num_string
                            ),
                            num_range,
                        )
                    })?;

                    // overflow when parsing from string
                    if v.is_infinite() {
                        return Err(Error::MessageWithLocation(
                            format!("F32 floating point number \"{}\" is overflow.", num_string),
                            num_range,
                        ));
                    }

                    NumberToken::F32(v)
                }
                NumberType::F64 => {
                    let v = num_string.parse::<f64>().map_err(|_| {
                        Error::MessageWithLocation(
                            format!(
                                "Can not convert \"{}\" to f64 floating-point number.",
                                num_string
                            ),
                            num_range,
                        )
                    })?;

                    // overflow when parsing from string
                    if v.is_infinite() {
                        return Err(Error::MessageWithLocation(
                            format!("F64 floating point number \"{}\" is overflow.", num_string),
                            num_range,
                        ));
                    }

                    NumberToken::F64(v)
                }
            }
        } else if found_point || found_e {
            // the default floating-point number type is f64

            let v = num_string.parse::<f64>().map_err(|_| {
                Error::MessageWithLocation(
                    format!(
                        "Can not convert \"{}\" to f64 floating-point number.",
                        num_string
                    ),
                    num_range,
                )
            })?;

            // overflow when parsing from string
            if v.is_infinite() {
                return Err(Error::MessageWithLocation(
                    format!("F64 floating point number \"{}\" is overflow.", num_string),
                    num_range,
                ));
            }

            NumberToken::F64(v)
        } else {
            // the default integer number type is i32

            let v = num_string.parse::<u32>().map_err(|_| {
                Error::MessageWithLocation(
                    format!("Can not convert \"{}\" to i32 integer number.", num_string,),
                    num_range,
                )
            })?;

            NumberToken::I32(v)
        };

        Ok(TokenWithRange::new(Token::Number(num_token), num_range))
    }

    fn lex_number_type_suffix(&mut self) -> Result<NumberType, Error> {
        // iddT  //
        // ^^ ^__// to here
        // ||____// d = 0..9, validated
        // |_____// current char, validated
        //
        // i = i/f
        // d = 0..=9
        // T = terminator chars || EOF

        self.push_peek_position();

        let first_char = self.next_char().unwrap(); // consume char 'i/u/f'

        let mut type_name = String::new();
        type_name.push(first_char);

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                '0'..='9' => {
                    // valid char for type name
                    type_name.push(*current_char);

                    // consume digit
                    self.next_char();
                }
                _ => {
                    break;
                }
            }
        }

        let type_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        let nt = NumberType::from_str(&type_name)
            .map_err(|msg| Error::MessageWithLocation(msg, type_range))?;

        Ok(nt)
    }

    fn lex_number_hex(&mut self) -> Result<TokenWithRange, Error> {
        // 0xaabbT  //
        // ^^    ^__// to here
        // ||_______// validated
        // |________// current char, validated
        //
        // T = terminator chars || EOF

        self.push_peek_position();

        self.next_char(); // consume '0'
        self.next_char(); // consume 'x'

        let mut num_string = String::new();
        let mut num_type: Option<NumberType> = None; // "_ixx"

        let mut found_point: bool = false; // to indicated whether char '.' is found
        let mut found_p: bool = false; // to indicated whether char 'p' is found

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                'f' if num_type.is_none()
                    && found_p
                    && matches!(self.peek_char(1), Some('0'..='9')) =>
                {
                    // 'f' is allowed only in the hex floating point literal mode, (i.e. the
                    //  character 'p' should be detected first)
                    let nt = self.lex_number_type_suffix()?;
                    num_type.replace(nt);
                    break;
                }
                '0'..='9' | 'a'..='f' | 'A'..='F' => {
                    // valid digits for hex number
                    num_string.push(*current_char);

                    self.next_char(); // consume digit
                }
                '_' => {
                    self.next_char(); // consume '_'
                }
                '.' if !found_point && !found_p => {
                    // going to be hex floating point literal mode
                    found_point = true;

                    num_string.push(*current_char);

                    self.next_char(); // consume '.'
                }
                'p' | 'P' if !found_p => {
                    // hex floating point literal mode
                    found_p = true;

                    // 0x0.123p45
                    // 0x0.123p+45
                    // 0x0.123p-45
                    if self.peek_char_and_equals(1, '-') {
                        num_string.push_str("p-");
                        self.next_char(); // consume 'p'
                        self.next_char(); // consume '-'
                    } else if self.peek_char_and_equals(1, '+') {
                        num_string.push_str("p+");
                        self.next_char(); // consume 'p'
                        self.next_char(); // consume '+'
                    } else {
                        num_string.push(*current_char);
                        self.next_char(); // consume 'p'
                    }
                }
                'i' if num_type.is_none()
                    && !found_point
                    && !found_p
                    && matches!(self.peek_char(1), Some('0'..='9')) =>
                {
                    // only 'i' and 'u' are allowed for hexadecimal integer numbers,
                    // and 'f' is a ordinary hex digit.
                    let nt = self.lex_number_type_suffix()?;
                    num_type.replace(nt);

                    break;
                }
                ' ' | '\t' | '\r' | '\n' | ',' | ':' | '=' | '+' | '-' | '{' | '}' | '[' | ']'
                | '(' | ')' | '/' | '"' => {
                    // terminator chars
                    break;
                }
                _ => {
                    return Err(Error::MessageWithLocation(
                        format!("Invalid char '{}' for hexadecimal number.", current_char),
                        *self.peek_position(0).unwrap(),
                    ));
                }
            }
        }

        let num_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        if num_string.is_empty() {
            return Err(Error::MessageWithLocation(
                "Empty hexadecimal number".to_owned(),
                num_range,
            ));
        }

        if found_point && !found_p {
            return Err(Error::MessageWithLocation(
                format!(
                    "Hexadecimal floating point number \"{}\" is missing the exponent.",
                    num_string
                ),
                num_range,
            ));
        }

        let num_token = if found_p {
            // the default type for floating-point is f64
            let mut to_f64 = true;

            if let Some(nt) = num_type {
                match nt {
                    NumberType::F32 => {
                        to_f64 = false;
                    }
                    NumberType::F64 => {
                        to_f64 = true;
                    }
                    _ => {
                        return Err(Error::MessageWithLocation(format!(
                                "Invalid type \"{}\" for hexadecimal floating-point numbers, only type \"f32\" and \"f64\" are allowed.",
                                nt
                            ),
                            num_range
                        ));
                    }
                }
            };

            num_string.insert_str(0, "0x");

            if to_f64 {
                let v = hexfloat2::parse::<f64>(&num_string).map_err(|_| {
                    // there is no detail message provided by `hexfloat2::parse`.
                    Error::MessageWithLocation(
                        format!(
                            "Can not convert \"{}\" to f64 floating-point number.",
                            num_string
                        ),
                        num_range,
                    )
                })?;

                NumberToken::F64(v)
            } else {
                let v = hexfloat2::parse::<f32>(&num_string).map_err(|_| {
                    // there is no detail message provided by `hexfloat2::parse`.
                    Error::MessageWithLocation(
                        format!(
                            "Can not convert \"{}\" to f32 floating-point number.",
                            num_string
                        ),
                        num_range,
                    )
                })?;

                NumberToken::F32(v)
            }
        } else if let Some(nt) = num_type {
            match nt {
                NumberType::I8 => {
                    let v = u8::from_str_radix(&num_string, 16).map_err(|_| {
                        Error::MessageWithLocation(
                            format!("Can not convert \"{}\" to i8 integer number.", num_string),
                            num_range,
                        )
                    })?;

                    NumberToken::I8(v)
                }
                NumberType::I16 => {
                    let v = u16::from_str_radix(&num_string, 16).map_err(|_| {
                        Error::MessageWithLocation(
                            format!("Can not convert \"{}\" to i16 integer number.", num_string),
                            num_range,
                        )
                    })?;

                    NumberToken::I16(v)
                }
                NumberType::I32 => {
                    let v = u32::from_str_radix(&num_string, 16).map_err(|_| {
                        Error::MessageWithLocation(
                            format!("Can not convert \"{}\" to i32 integer number.", num_string),
                            num_range,
                        )
                    })?;

                    NumberToken::I32(v)
                }
                NumberType::I64 => {
                    let v = u64::from_str_radix(&num_string, 16).map_err(|_| {
                        Error::MessageWithLocation(
                            format!("Can not convert \"{}\" to i64 integer number.", num_string),
                            num_range,
                        )
                    })?;

                    NumberToken::I64(v)
                }
                NumberType::F32 | NumberType::F64 => {
                    // '0x..f32' and '0x..f64' would only be parsed
                    // as ordinary hex digits
                    unreachable!()
                }
            }
        } else {
            // default
            // convert to i32
            let v = u32::from_str_radix(&num_string, 16).map_err(|_| {
                Error::MessageWithLocation(
                    format!("Can not convert \"{}\" to i32 integer number.", num_string),
                    num_range,
                )
            })?;

            NumberToken::I32(v)
        };

        Ok(TokenWithRange::new(Token::Number(num_token), num_range))
    }

    fn lex_number_binary(&mut self) -> Result<TokenWithRange, Error> {
        // 0b1010T  //
        // ^^    ^__// to here
        // ||_______// validated
        // |________// current char, validated
        //
        // T = terminator chars || EOF

        self.push_peek_position();

        self.next_char(); // consume '0'
        self.next_char(); // consume 'b'

        let mut num_string = String::new();
        let mut num_type: Option<NumberType> = None;

        while let Some(current_char) = self.peek_char(0) {
            match current_char {
                '0' | '1' => {
                    // valid digits for binary number
                    num_string.push(*current_char);

                    self.next_char(); // consume digit
                }
                '_' => {
                    self.next_char(); // consume '_'
                }
                // binary form only supports integer numbers, does not support floating-point numbers
                'i' if num_type.is_none() && matches!(self.peek_char(1), Some('0'..='9')) => {
                    let nt = self.lex_number_type_suffix()?;
                    num_type.replace(nt);
                    break;
                }
                ' ' | '\t' | '\r' | '\n' | ',' | ':' | '=' | '+' | '-' | '{' | '}' | '[' | ']'
                | '(' | ')' | '/' | '"' => {
                    // terminator chars
                    break;
                }
                _ => {
                    return Err(Error::MessageWithLocation(
                        format!("Invalid char '{}' for binary number.", current_char),
                        *self.peek_position(0).unwrap(),
                    ));
                }
            }
        }

        let num_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        if num_string.is_empty() {
            return Err(Error::MessageWithLocation(
                "Empty binary number.".to_owned(),
                num_range,
            ));
        }

        let num_token = if let Some(nt) = num_type {
            match nt {
                NumberType::I8 => {
                    let v = u8::from_str_radix(&num_string, 2).map_err(|_| {
                        Error::MessageWithLocation(
                            format!("Can not convert \"{}\" to i8 integer number.", num_string,),
                            num_range,
                        )
                    })?;

                    NumberToken::I8(v)
                }
                NumberType::I16 => {
                    let v = u16::from_str_radix(&num_string, 2).map_err(|_| {
                        Error::MessageWithLocation(
                            format!("Can not convert \"{}\" to i16 integer number.", num_string,),
                            num_range,
                        )
                    })?;

                    NumberToken::I16(v)
                }
                NumberType::I32 => {
                    let v = u32::from_str_radix(&num_string, 2).map_err(|_| {
                        Error::MessageWithLocation(
                            format!("Can not convert \"{}\" to i32 integer number.", num_string),
                            num_range,
                        )
                    })?;

                    NumberToken::I32(v)
                }
                NumberType::I64 => {
                    let v = u64::from_str_radix(&num_string, 2).map_err(|_| {
                        Error::MessageWithLocation(
                            format!("Can not convert \"{}\" to i64 integer number.", num_string),
                            num_range,
                        )
                    })?;

                    NumberToken::I64(v)
                }
                NumberType::F32 | NumberType::F64 => {
                    unreachable!()
                }
            }
        } else {
            // default
            // convert to i32

            let v = u32::from_str_radix(&num_string, 2).map_err(|_| {
                Error::MessageWithLocation(
                    format!("Can not convert \"{}\" to i32 integer number.", num_string),
                    num_range,
                )
            })?;

            NumberToken::I32(v)
        };

        Ok(TokenWithRange::new(Token::Number(num_token), num_range))
    }

    //     fn lex_char(&mut self) -> Result<TokenWithRange, Error> {
    //         // 'a'?  //
    //         // ^  ^__// to here
    //         // |_____// current char, validated
    //
    //         self.push_peek_position();
    //
    //         self.next_char(); // consume "'"
    //
    //         let character = match self.next_char() {
    //             Some(previous_previous_char) => {
    //                 match previous_previous_char {
    //                     '\\' => {
    //                         // escape chars
    //                         match self.next_char() {
    //                             Some(previous_char) => {
    //                                 match previous_char {
    //                                     '\\' => '\\',
    //                                     '\'' => '\'',
    //                                     '"' => {
    //                                         // double quote does not necessary to be escaped for char
    //                                         // however, it is still supported for consistency between chars and strings.
    //                                         '"'
    //                                     }
    //                                     't' => {
    //                                         // horizontal tabulation
    //                                         '\t'
    //                                     }
    //                                     'r' => {
    //                                         // carriage return (CR, ascii 13)
    //                                         '\r'
    //                                     }
    //                                     'n' => {
    //                                         // new line character (line feed, LF, ascii 10)
    //                                         '\n'
    //                                     }
    //                                     '0' => {
    //                                         // null char
    //                                         '\0'
    //                                     }
    //                                     'u' => {
    //                                         if self.peek_char_and_equals(0, '{') {
    //                                             // unicode code point, e.g. '\u{2d}', '\u{6587}'
    //                                             self.unescape_unicode()?
    //                                         } else {
    //                                             return Err(Error::MessageWithLocation(
    //                                                 "Missing the brace for unicode escape sequence."
    //                                                     .to_owned(),
    //                                                 self.last_position.move_position_forward(),
    //                                             ));
    //                                         }
    //                                     }
    //                                     _ => {
    //                                         return Err(Error::MessageWithLocation(
    //                                             format!("Unexpected escape char '{}'.", previous_char),
    //                                             // self.last_position,
    //                                             Location::from_position_and_length(
    //                                                 &self.last_position.move_position_backward(),
    //                                                 2,
    //                                             ),
    //                                         ));
    //                                     }
    //                                 }
    //                             }
    //                             None => {
    //                                 // `\` + EOF
    //                                 return Err(Error::UnexpectedEndOfDocument(
    //                                     "Incomplete escape character sequence.".to_owned(),
    //                                 ));
    //                             }
    //                         }
    //                     }
    //                     '\'' => {
    //                         // `''`
    //                         return Err(Error::MessageWithLocation(
    //                             "Empty char.".to_owned(),
    //                             Location::from_position_pair_with_end_included(
    //                                 &self.pop_saved_position(),
    //                                 &self.last_position,
    //                             ),
    //                         ));
    //                     }
    //                     _ => {
    //                         // ordinary char
    //                         previous_previous_char
    //                     }
    //                 }
    //             }
    //             None => {
    //                 // `'EOF`
    //                 return Err(Error::UnexpectedEndOfDocument(
    //                     "Incomplete character.".to_owned(),
    //                 ));
    //             }
    //         };
    //
    //         // consume the right single quote
    //         match self.next_char() {
    //             Some('\'') => {
    //                 // Ok
    //             }
    //             Some(_) => {
    //                 // `'a?`
    //                 return Err(Error::MessageWithLocation(
    //                     "Expected a closing single quote for char".to_owned(),
    //                     self.last_position,
    //                 ));
    //             }
    //             None => {
    //                 // `'aEOF`
    //                 return Err(Error::UnexpectedEndOfDocument(
    //                     "Incomplete character.".to_owned(),
    //                 ));
    //             }
    //         }
    //
    //         let character_range = Location::from_position_pair_with_end_included(
    //             &self.pop_saved_position(),
    //             &self.last_position,
    //         );
    //         Ok(TokenWithRange::new(Token::Char(character), character_range))
    //     }

    fn unescape_unicode(&mut self) -> Result<char, Error> {
        // \u{6587}?  //
        //   ^     ^__// to here
        //   |________// current char, validated

        self.push_peek_position();

        self.next_char(); // comsume char '{'

        let mut codepoint_string = String::new();

        loop {
            match self.next_char() {
                Some(previous_char) => match previous_char {
                    '}' => break,
                    '0'..='9' | 'a'..='f' | 'A'..='F' => codepoint_string.push(previous_char),
                    _ => {
                        return Err(Error::MessageWithLocation(
                            format!(
                                "Invalid character '{}' for unicode escape sequence.",
                                previous_char
                            ),
                            self.last_position,
                        ));
                    }
                },
                None => {
                    // EOF
                    return Err(Error::UnexpectedEndOfDocument(
                        "Incomplete unicode escape sequence.".to_owned(),
                    ));
                }
            }

            if codepoint_string.len() > 6 {
                break;
            }
        }

        let codepoint_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        if codepoint_string.len() > 6 {
            return Err(Error::MessageWithLocation(
                "Unicode point code exceeds six digits.".to_owned(),
                codepoint_range,
            ));
        }

        if codepoint_string.is_empty() {
            return Err(Error::MessageWithLocation(
                "Empty unicode code point.".to_owned(),
                codepoint_range,
            ));
        }

        let codepoint = u32::from_str_radix(&codepoint_string, 16).unwrap();

        if let Some(c) = char::from_u32(codepoint) {
            // valid code point:
            // 0 to 0x10FFFF, inclusive
            //
            // ref:
            // https://doc.rust-lang.org/std/primitive.char.html
            Ok(c)
        } else {
            Err(Error::MessageWithLocation(
                "Invalid unicode code point.".to_owned(),
                codepoint_range,
            ))
        }
    }

    fn lex_string(&mut self) -> Result<TokenWithRange, Error> {
        // "abc"?  //
        // ^    ^__// to here
        // |_______// current char, validated

        self.push_peek_position();

        self.next_char(); // consume '"'

        let mut final_string = String::new();

        loop {
            match self.next_char() {
                Some(previous_previous_char) => {
                    match previous_previous_char {
                        '\\' => {
                            // escape chars
                            match self.next_char() {
                                Some(previous_char) => {
                                    match previous_char {
                                        '\\' => {
                                            final_string.push('\\');
                                        }
                                        '\'' => {
                                            // single quote does not necessary to be escaped for string
                                            // however, it is still supported for consistency between chars and strings.
                                            final_string.push('\'');
                                        }
                                        '"' => {
                                            final_string.push('"');
                                        }
                                        't' => {
                                            // horizontal tabulation
                                            final_string.push('\t');
                                        }
                                        'r' => {
                                            // carriage return (CR, ascii 13)
                                            final_string.push('\r');
                                        }
                                        'n' => {
                                            // new line character (line feed, LF, ascii 10)
                                            final_string.push('\n');
                                        }
                                        '0' => {
                                            // null char
                                            final_string.push('\0');
                                        }
                                        'u' => {
                                            if self.peek_char_and_equals(0, '{') {
                                                // unicode code point, e.g. '\u{2d}', '\u{6587}'
                                                let ch = self.unescape_unicode()?;
                                                final_string.push(ch);
                                            } else {
                                                return Err(Error::MessageWithLocation(
                                                    "Missing the brace for unicode escape sequence.".to_owned(),
                                                    self.last_position.move_position_forward()
                                                ));
                                            }
                                        }
                                        '\r' if self.peek_char_and_equals(0, '\n') => {
                                            // (single line) long string

                                            self.next_char(); // consume '\n'
                                            self.consume_all_leading_whitespaces()?;
                                        }
                                        '\n' => {
                                            // (single line) long string
                                            self.consume_all_leading_whitespaces()?;
                                        }
                                        _ => {
                                            return Err(Error::MessageWithLocation(
                                                format!(
                                                    "Unsupported escape char '{}'.",
                                                    previous_char
                                                ),
                                                Location::from_position_and_length(
                                                    &self.last_position.move_position_backward(),
                                                    2,
                                                ),
                                            ));
                                        }
                                    }
                                }
                                None => {
                                    // `\` + EOF
                                    return Err(Error::UnexpectedEndOfDocument(
                                        "Incomplete character escape sequence.".to_owned(),
                                    ));
                                }
                            }
                        }
                        '"' => {
                            // end of the string
                            break;
                        }
                        _ => {
                            // ordinary char
                            final_string.push(previous_previous_char);
                        }
                    }
                }
                None => {
                    // `"...EOF`
                    return Err(Error::UnexpectedEndOfDocument(
                        "Incomplete string.".to_owned(),
                    ));
                }
            }
        }

        let final_string_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        Ok(TokenWithRange::new(
            Token::String(final_string),
            final_string_range,
        ))
    }

    fn consume_all_leading_whitespaces(&mut self) -> Result<(), Error> {
        // \nssssS  //
        //   ^   ^__// to here ('s' = whitespace, 'S' = not whitespace)
        //   |______// current char, UNVALIDATED

        loop {
            match self.peek_char(0) {
                Some(current_char) => {
                    match current_char {
                        ' ' | '\t' => {
                            self.next_char(); // consume ' ' or '\t'
                        }
                        _ => {
                            break;
                        }
                    }
                }
                None => {
                    // EOF
                    return Err(Error::UnexpectedEndOfDocument(
                        "Incomplete string.".to_owned(),
                    ));
                }
            }
        }

        Ok(())
    }

    fn lex_raw_string(&mut self) -> Result<TokenWithRange, Error> {
        // r"abc"?  //
        // ^^    ^__// to here
        // ||_______// validated
        // |________// current char, validated

        self.push_peek_position();

        self.next_char(); // consume char 'r'
        self.next_char(); // consume the '"'

        let mut final_string = String::new();

        loop {
            match self.next_char() {
                Some(previous_char) => {
                    match previous_char {
                        '"' => {
                            // end of the string
                            break;
                        }
                        _ => {
                            // ordinary char
                            final_string.push(previous_char);
                        }
                    }
                }
                None => {
                    // `r"...EOF`
                    return Err(Error::UnexpectedEndOfDocument(
                        "Incomplete string.".to_owned(),
                    ));
                }
            }
        }

        let final_string_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        Ok(TokenWithRange::new(
            Token::String(final_string),
            final_string_range,
        ))
    }

    fn lex_raw_string_with_hash_symbol(&mut self) -> Result<TokenWithRange, Error> {
        // r#"abc"#?  //
        // ^^^     ^__// to here
        // |||________// validated
        // ||_________// validated
        // |__________// current char, validated

        // hash symbol = '#', i.e. the pound sign

        self.push_peek_position();

        self.next_char(); // consume 'r'
        self.next_char(); // consume '#'
        self.next_char(); // consume '"'

        let mut final_string = String::new();

        loop {
            match self.next_char() {
                Some(previous_char) => {
                    match previous_char {
                        '"' if self.peek_char_and_equals(0, '#') => {
                            // it is the end of the string
                            self.next_char(); // consume '#'
                            break;
                        }
                        _ => {
                            // ordinary char
                            final_string.push(previous_char);
                        }
                    }
                }
                None => {
                    // `r#"...EOF`
                    return Err(Error::UnexpectedEndOfDocument(
                        "Incomplete string.".to_owned(),
                    ));
                }
            }
        }

        let final_string_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        Ok(TokenWithRange::new(
            Token::String(final_string),
            final_string_range,
        ))
    }

    fn lex_auto_trimmed_string(&mut self) -> Result<TokenWithRange, Error> {
        // """\n                    //
        // ^^^  auto-trimmed string //
        // |||  ...\n               //
        // |||  """?                //
        // |||     ^________________// to here ('?' = any chars or EOF)
        // |||______________________// validated
        // ||_______________________// validated
        // |________________________// current char, validated

        // note:
        // - the '\n' of the first line is necessary.
        // - the closed `"""` must be started with a new line.

        self.push_peek_position();

        self.next_char(); // consume the 1st '"'
        self.next_char(); // consume the 2nd '"'
        self.next_char(); // consume the 3rd '"'

        if self.peek_char_and_equals(0, '\n') {
            self.next_char(); // consume '\n'
        } else if self.peek_char_and_equals(0, '\r') && self.peek_char_and_equals(1, '\n') {
            self.next_char(); // consume '\r'
            self.next_char(); // consume '\n'
        } else {
            return Err(Error::MessageWithLocation(
                "The content of auto-trimmed string should start on a new line.".to_owned(),
                self.last_position.move_position_forward(),
            ));
        }

        let mut lines = vec![]; // String::new();
        let mut current_line = vec![]; //String::new();

        loop {
            match self.next_char() {
                Some(previous_char) => {
                    match previous_char {
                        '\n' => {
                            current_line.push('\n');
                            lines.push(current_line);

                            current_line = vec![];
                        }
                        '\r' if self.peek_char_and_equals(0, '\n') => {
                            self.next_char(); // consume '\n'

                            current_line.push('\r');
                            current_line.push('\n');
                            lines.push(current_line);

                            current_line = vec![];
                        }
                        '"' if current_line.iter().all(|&c| c == ' ' || c == '\t')
                            && self.peek_char_and_equals(0, '"')
                            && self.peek_char_and_equals(1, '"') =>
                        {
                            // it is the end of string
                            self.next_char(); // consume '"'
                            self.next_char(); // consume '"'
                            break;
                        }
                        _ => {
                            // ordinary char
                            current_line.push(previous_char);
                        }
                    }
                }
                None => {
                    // `"""\n...EOF`
                    return Err(Error::UnexpectedEndOfDocument(
                        "Incomplete string.".to_owned(),
                    ));
                }
            }
        }

        let range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        if lines.is_empty() {
            return Ok(TokenWithRange::new(Token::String(String::new()), range));
        }

        // calculate leading spaces of each line
        //
        // the empty lines would be excluded
        let spaces: Vec<usize> = lines
            .iter()
            .filter(|line| {
                let is_empty = (line.len() == 1 && line[0] == '\n')
                    || (line.len() == 2 && line[0] == '\r' && line[1] == '\n');
                !is_empty
            })
            .map(|line| {
                let mut count = 0;
                while count < line.len() {
                    if !(line[count] == ' ' || line[count] == '\t') {
                        break;
                    }
                    count += 1;
                }
                count
            })
            .collect();

        let min = *spaces.iter().min().unwrap_or(&0);

        // trim leading spaces
        lines
            .iter_mut()
            .filter(|line| {
                let is_empty = (line.len() == 1 && line[0] == '\n')
                    || (line.len() == 2 && line[0] == '\r' && line[1] == '\n');
                !is_empty
            })
            .for_each(|line| {
                line.drain(0..min);
            });

        // trim the ending '\n' or "\r\n"
        let last_index = lines.len() - 1;
        let last_line = &mut lines[last_index];
        if matches!(last_line.last(), Some('\n')) {
            last_line.pop();
        }
        if matches!(last_line.last(), Some('\r')) {
            last_line.pop();
        }

        let content = lines
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join("");

        Ok(TokenWithRange::new(Token::String(content), range))
    }

    fn lex_byte_data_hexadecimal(&mut self) -> Result<TokenWithRange, Error> {
        // h"00 11 aa bb"?  //
        // ^^            ^__// to here
        // ||_______________// validated
        // |________________// current char, validated

        let consume_zero_or_more_whitespaces = |iter: &mut Lexer| -> Result<usize, Error> {
            // exit when encounting non-whitespaces or EOF
            let mut amount: usize = 0;

            while let Some(' ' | '\t' | '\r' | '\n') = iter.peek_char(0) {
                amount += 1;
                iter.next_char();
            }

            Ok(amount)
        };

        let consume_one_or_more_whitespaces = |iter: &mut Lexer| -> Result<usize, Error> {
            let mut amount: usize = 0;

            loop {
                match iter.peek_char(0) {
                    Some(current_char) => {
                        match current_char {
                            ' ' | '\t' | '\r' | '\n' => {
                                // consume whitespace
                                iter.next_char();
                                amount += 1;
                            }
                            _ => {
                                if amount > 0 {
                                    break;
                                } else {
                                    return Err(Error::MessageWithLocation(
                                            "Expect a whitespace between the hexadecimal byte data digits."
                                                .to_owned(),
                                            iter.last_position.move_position_forward()
                                        ));
                                }
                            }
                        }
                    }
                    None => {
                        // h"...EOF
                        return Err(Error::UnexpectedEndOfDocument(
                            "Incomplete hexadecimal byte data.".to_owned(),
                        ));
                    }
                }
            }

            Ok(amount)
        };

        self.push_peek_position();

        self.next_char(); // consume char 'h'
        self.next_char(); // consume quote '"'

        let mut bytes: Vec<u8> = Vec::new();
        let mut chars: [char; 2] = ['0', '0'];

        consume_zero_or_more_whitespaces(self)?;

        loop {
            if self.peek_char_and_equals(0, '"') {
                break;
            }

            for c in &mut chars {
                match self.next_char() {
                    Some(previous_char) => match previous_char {
                        'a'..='f' | 'A'..='F' | '0'..='9' => {
                            *c = previous_char;
                        }
                        _ => {
                            return Err(Error::MessageWithLocation(
                                format!(
                                    "Invalid digit '{}' for hexadecimal byte data.",
                                    previous_char
                                ),
                                self.last_position,
                            ));
                        }
                    },
                    None => {
                        return Err(Error::UnexpectedEndOfDocument(
                            "Incomplete hexadecimal byte data.".to_owned(),
                        ))
                    }
                }
            }

            let byte_string = String::from_iter(chars);
            let byte_number = u8::from_str_radix(&byte_string, 16).unwrap();
            bytes.push(byte_number);

            if self.peek_char_and_equals(0, '"') {
                break;
            }

            // consume at lease one whitespace
            consume_one_or_more_whitespaces(self)?;
        }

        self.next_char(); // consume '"'

        let bytes_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        Ok(TokenWithRange::new(Token::HexByteData(bytes), bytes_range))
    }

    fn lex_line_comment(&mut self) -> Result<TokenWithRange, Error> {
        // xx...[\r]\n?  //
        // ^^         ^__// to here ('?' = any char or EOF)
        // ||____________// validated
        // |_____________// current char, validated
        //
        // x = '/'

        self.push_peek_position();

        self.next_char(); // consume the 1st '/'
        self.next_char(); // consume the 2nd '/'

        let mut comment_string = String::new();

        while let Some(current_char) = self.peek_char(0) {
            // ignore all chars except '\n' or '\r\n'
            // note that the "line comment token" does not include the trailing new line chars (\n or \r\n),

            match current_char {
                '\n' => {
                    break;
                }
                '\r' if self.peek_char_and_equals(0, '\n') => {
                    break;
                }
                _ => {
                    comment_string.push(*current_char);

                    self.next_char(); // consume char
                }
            }
        }

        let comment_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        Ok(TokenWithRange::new(
            Token::Comment(Comment::Line(comment_string)),
            comment_range,
        ))
    }

    fn lex_block_comment(&mut self) -> Result<TokenWithRange, Error> {
        // /*...*/?  //
        // ^^     ^__// to here
        // ||________// validated
        // |_________// current char, validated

        self.push_peek_position();

        self.next_char(); // consume '/'
        self.next_char(); // consume '*'

        let mut comment_string = String::new();
        let mut depth = 1; // nested depth

        loop {
            match self.next_char() {
                Some(previous_char) => {
                    match previous_char {
                        '/' if self.peek_char_and_equals(0, '*') => {
                            // nested block comment
                            comment_string.push_str("/*");

                            self.next_char(); // consume '*'

                            // increase depth
                            depth += 1;
                        }
                        '*' if self.peek_char_and_equals(0, '/') => {
                            self.next_char(); // consume '/'

                            // decrease depth
                            depth -= 1;

                            // check pairs
                            if depth == 0 {
                                break;
                            } else {
                                comment_string.push_str("*/");
                            }
                        }
                        _ => {
                            // ignore all chars except "/*" and "*/"
                            // note that line comments within block comments are ignored also.
                            comment_string.push(previous_char);
                        }
                    }
                }
                None => {
                    let msg = if depth > 1 {
                        "Incomplete nested block comment.".to_owned()
                    } else {
                        "Incomplete block comment.".to_owned()
                    };

                    return Err(Error::UnexpectedEndOfDocument(msg));
                }
            }
        }

        let comment_range = Location::from_position_pair_with_end_included(
            &self.pop_saved_position(),
            &self.last_position,
        );

        Ok(TokenWithRange::new(
            Token::Comment(Comment::Block(comment_string)),
            comment_range,
        ))
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        error::Error,
        location::Location,
        token::{Comment, NumberToken, Token, TokenWithRange},
    };

    use super::lex_from_str;

    impl Token {
        pub fn new_namepath(s: &str) -> Self {
            Token::NamePath(s.to_owned())
        }

        pub fn new_name(s: &str) -> Self {
            Token::Name(s.to_owned())
        }

        pub fn new_keyword(s: &str) -> Self {
            Token::Keyword(s.to_owned())
        }

        pub fn new_datatype_name(s: &str) -> Self {
            Token::DataTypeName(s.to_owned())
        }

        pub fn new_string(s: &str) -> Self {
            Token::String(s.to_owned())
        }
    }

    fn lex_from_str_without_location(s: &str) -> Result<Vec<Token>, Error> {
        let tokens = lex_from_str(s)?
            .into_iter()
            .map(|e| e.token)
            .collect::<Vec<Token>>();
        Ok(tokens)
    }

    #[test]
    fn test_lex_whitespaces() {
        assert_eq!(lex_from_str_without_location("  ").unwrap(), vec![]);

        assert_eq!(
            lex_from_str_without_location("()").unwrap(),
            vec![Token::LeftParen, Token::RightParen]
        );

        assert_eq!(
            lex_from_str_without_location("(  )").unwrap(),
            vec![Token::LeftParen, Token::RightParen]
        );

        assert_eq!(
            lex_from_str_without_location("(\t\r\n\n\n)").unwrap(),
            vec![
                Token::LeftParen,
                Token::NewLine,
                Token::NewLine,
                Token::NewLine,
                Token::RightParen,
            ]
        );

        // location

        assert_eq!(lex_from_str("  ").unwrap(), vec![]);

        assert_eq!(
            lex_from_str("()").unwrap(),
            vec![
                TokenWithRange::new(Token::LeftParen, Location::new_range(0, 0, 0, 0, 1)),
                TokenWithRange::new(Token::RightParen, Location::new_range(0, 1, 0, 1, 1)),
            ]
        );

        assert_eq!(
            lex_from_str("(  )").unwrap(),
            vec![
                TokenWithRange::new(Token::LeftParen, Location::new_range(0, 0, 0, 0, 1)),
                TokenWithRange::new(Token::RightParen, Location::new_range(0, 3, 0, 3, 1)),
            ]
        );

        // "(\t\r\n\n\n)"
        //  _--____--__-
        //  0  2   4 5 6    // index
        //  0  0   1 2 3    // line
        //  0  2   0 0 1    // column
        //  1  2   1 1 1    // length

        assert_eq!(
            lex_from_str("(\t\r\n\n\n)").unwrap(),
            vec![
                TokenWithRange::new(Token::LeftParen, Location::new_range(0, 0, 0, 0, 1)),
                TokenWithRange::new(Token::NewLine, Location::new_range(0, 2, 0, 2, 2,)),
                TokenWithRange::new(Token::NewLine, Location::new_range(0, 4, 1, 0, 1,)),
                TokenWithRange::new(Token::NewLine, Location::new_range(0, 5, 2, 0, 1,)),
                TokenWithRange::new(Token::RightParen, Location::new_range(0, 6, 3, 0, 1)),
            ]
        );
    }

    #[test]
    fn test_lex_punctuations() {
        assert_eq!(
            lex_from_str_without_location(",:=->+-{}[]()").unwrap(),
            vec![
                Token::Comma,
                Token::Colon,
                Token::Equal,
                Token::RightArrow,
                Token::Plus,
                Token::Minus,
                Token::LeftBrace,
                Token::RightBrace,
                Token::LeftBracket,
                Token::RightBracket,
                Token::LeftParen,
                Token::RightParen,
            ]
        );

        // location

        assert_eq!(
            lex_from_str("-->").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Minus,
                    &Location::new_position(0, 0, 0, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::RightArrow,
                    &Location::new_position(0, 1, 0, 1),
                    2
                ),
            ]
        );
    }

    #[test]
    fn test_lex_identifier() {
        assert_eq!(
            lex_from_str_without_location("name").unwrap(),
            vec![Token::new_name("name")]
        );

        assert_eq!(
            lex_from_str_without_location("(name)").unwrap(),
            vec![Token::LeftParen, Token::new_name("name"), Token::RightParen,]
        );

        assert_eq!(
            lex_from_str_without_location("( a )").unwrap(),
            vec![Token::LeftParen, Token::new_name("a"), Token::RightParen,]
        );

        assert_eq!(
            lex_from_str_without_location("a__b__c").unwrap(),
            vec![Token::new_name("a__b__c")]
        );

        assert_eq!(
            lex_from_str_without_location("foo bar").unwrap(),
            vec![Token::new_name("foo"), Token::new_name("bar")]
        );

        assert_eq!(
            lex_from_str_without_location("αβγ 文字 🍞🥛").unwrap(),
            vec![
                Token::new_name("αβγ"),
                Token::new_name("文字"),
                Token::new_name("🍞🥛"),
            ]
        );

        // location

        assert_eq!(
            lex_from_str("hello ASON").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_name("hello"),
                    &Location::new_position(0, 0, 0, 0),
                    5
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_name("ASON"),
                    &Location::new_position(0, 6, 0, 6),
                    4
                )
            ]
        );

        // err: invalid char
        assert!(matches!(
            lex_from_str_without_location("abc&xyz"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 3,
                    line: 0,
                    column: 3,
                    length: 0
                }
            ))
        ));
    }

    #[test]
    fn test_lex_namepath() {
        assert_eq!(
            lex_from_str("foo::bar a::bc::def a:b").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_namepath("foo::bar"),
                    &Location::new_position(0, 0, 0, 0),
                    8
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_namepath("a::bc::def"),
                    &Location::new_position(0, 9, 0, 9),
                    10
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_name("a"),
                    &Location::new_position(0, 20, 0, 20),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Colon,
                    &Location::new_position(0, 21, 0, 21),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_name("b"),
                    &Location::new_position(0, 22, 0, 22),
                    1
                )
            ]
        );
    }

    #[test]
    fn test_lex_keywords() {
        assert_eq!(
            lex_from_str_without_location("use as external fn data pub readonly uninit").unwrap(),
            vec![
                Token::new_keyword("use"),
                Token::new_keyword("as"),
                Token::new_keyword("external"),
                Token::new_keyword("fn"),
                Token::new_keyword("data"),
                Token::new_keyword("pub"),
                Token::new_keyword("readonly"),
                Token::new_keyword("uninit"),
            ]
        );

        assert_eq!(
            lex_from_str_without_location("i64 i32 f64 f32 byte").unwrap(),
            vec![
                Token::new_datatype_name("i64"),
                Token::new_datatype_name("i32"),
                Token::new_datatype_name("f64"),
                Token::new_datatype_name("f32"),
                Token::new_datatype_name("byte"),
            ]
        );

        // location
        assert_eq!(
            lex_from_str("data foo:i32").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_keyword("data"),
                    &Location::new_position(0, 0, 0, 0),
                    4
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_name("foo"),
                    &Location::new_position(0, 5, 0, 5),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Colon,
                    &Location::new_position(0, 8, 0, 8),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_datatype_name("i32"),
                    &Location::new_position(0, 9, 0, 9),
                    3
                ),
            ]
        );
    }

    #[test]
    fn test_lex_multiline_location() {
        // "[\n  use\n    data\n]"
        //  01 234567 890123456 7   // index
        //  00 111111 222222222 3   // line
        //  01 012345 012345678 0   // column
        //  11   3  1     4   1 1   // length

        assert_eq!(
            lex_from_str("[\n  use\n    data\n]").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::LeftBracket,
                    &Location::new_position(0, 0, 0, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::NewLine,
                    &Location::new_position(0, 1, 0, 1),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_keyword("use"),
                    &Location::new_position(0, 4, 1, 2),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::NewLine,
                    &Location::new_position(0, 7, 1, 5),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_keyword("data"),
                    &Location::new_position(0, 12, 2, 4),
                    4
                ),
                TokenWithRange::from_position_and_length(
                    Token::NewLine,
                    &Location::new_position(0, 16, 2, 8),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::RightBracket,
                    &Location::new_position(0, 17, 3, 0),
                    1
                ),
            ]
        )
    }

    #[test]
    fn test_lex_shebang() {
        assert_eq!(
            lex_from_str("#!/bin/bash\nabc").unwrap(),
            vec![TokenWithRange::from_position_and_length(
                Token::new_name("abc"),
                &Location::new_position(0, 12, 1, 0),
                3
            )]
        );
    }

    #[test]
    fn test_lex_decimal_number() {
        assert_eq!(
            lex_from_str_without_location("(211)").unwrap(),
            vec![
                Token::LeftParen,
                Token::Number(NumberToken::I32(211)),
                Token::RightParen,
            ]
        );

        assert_eq!(
            lex_from_str_without_location("211").unwrap(),
            vec![Token::Number(NumberToken::I32(211))]
        );

        assert_eq!(
            lex_from_str_without_location("-2017").unwrap(),
            vec![Token::Minus, Token::Number(NumberToken::I32(2017))]
        );

        assert_eq!(
            lex_from_str_without_location("+2024").unwrap(),
            vec![Token::Plus, Token::Number(NumberToken::I32(2024))]
        );

        assert_eq!(
            lex_from_str_without_location("223_211").unwrap(),
            vec![Token::Number(NumberToken::I32(223_211))]
        );

        assert_eq!(
            lex_from_str_without_location("223 211").unwrap(),
            vec![
                Token::Number(NumberToken::I32(223)),
                Token::Number(NumberToken::I32(211)),
            ]
        );

        // location

        assert_eq!(
            lex_from_str("223 211").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Number(NumberToken::I32(223)),
                    &Location::new_position(0, 0, 0, 0,),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Number(NumberToken::I32(211)),
                    &Location::new_position(0, 4, 0, 4,),
                    3
                ),
            ]
        );

        // err: invalid char for decimal number
        assert!(matches!(
            lex_from_str_without_location("12x34"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 2,
                    line: 0,
                    column: 2,
                    length: 0
                }
            ))
        ));

        // err: integer number overflow
        assert!(matches!(
            lex_from_str_without_location("4_294_967_296"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 0,
                    line: 0,
                    column: 0,
                    length: 13
                }
            ))
        ));
    }

    #[allow(clippy::approx_constant)]
    #[test]
    fn test_lex_decimal_number_floating_point() {
        assert_eq!(
            lex_from_str_without_location("3.14").unwrap(),
            vec![Token::Number(NumberToken::F64(3.14))]
        );

        assert_eq!(
            lex_from_str_without_location("+1.414").unwrap(),
            vec![Token::Plus, Token::Number(NumberToken::F64(1.414))]
        );

        assert_eq!(
            lex_from_str_without_location("-2.718").unwrap(),
            vec![Token::Minus, Token::Number(NumberToken::F64(2.718))]
        );

        assert_eq!(
            lex_from_str_without_location("2.998e8").unwrap(),
            vec![Token::Number(NumberToken::F64(2.998e8))]
        );

        assert_eq!(
            lex_from_str_without_location("2.998e+8").unwrap(),
            vec![Token::Number(NumberToken::F64(2.998e+8))]
        );

        assert_eq!(
            lex_from_str_without_location("6.626e-34").unwrap(),
            vec![Token::Number(NumberToken::F64(6.626e-34))]
        );

        // err: incomplete floating point number since ends with '.'
        assert!(matches!(
            lex_from_str_without_location("123."),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 3,
                    line: 0,
                    column: 3,
                    length: 0
                }
            ))
        ));

        // err: incomplete floating point number since ends with 'e'
        assert!(matches!(
            lex_from_str_without_location("123e"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 3,
                    line: 0,
                    column: 3,
                    length: 0
                }
            ))
        ));

        // err: multiple '.' (point)
        assert!(matches!(
            lex_from_str_without_location("1.23.456"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 4,
                    line: 0,
                    column: 4,
                    length: 0
                }
            ))
        ));

        // err: multiple 'e' (exponent)
        assert!(matches!(
            lex_from_str_without_location("1e23e456"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 4,
                    line: 0,
                    column: 4,
                    length: 0
                }
            ))
        ));

        // err: unsupports start with dot
        assert!(matches!(
            lex_from_str_without_location(".123"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 0,
                    line: 0,
                    column: 0,
                    length: 0
                }
            ))
        ));
    }

    #[test]
    fn test_lex_decimal_number_with_explicit_type() {
        // general
        {
            assert_eq!(
                lex_from_str_without_location("11i32").unwrap(),
                vec![Token::Number(NumberToken::I32(11))]
            );

            assert_eq!(
                lex_from_str_without_location("11_i32").unwrap(),
                vec![Token::Number(NumberToken::I32(11))]
            );

            assert_eq!(
                lex_from_str_without_location("11__i32").unwrap(),
                vec![Token::Number(NumberToken::I32(11))]
            );

            // location

            // "101_i16 103_u32" // text
            //  012345678901234  // index
            assert_eq!(
                lex_from_str("101_i16 103_i32").unwrap(),
                vec![
                    TokenWithRange::from_position_and_length(
                        Token::Number(NumberToken::I16(101)),
                        &Location::new_position(0, 0, 0, 0),
                        7
                    ),
                    TokenWithRange::from_position_and_length(
                        Token::Number(NumberToken::I32(103)),
                        &Location::new_position(0, 8, 0, 8),
                        7
                    ),
                ]
            );
        }

        // byte
        {
            assert_eq!(
                lex_from_str_without_location("127_i8").unwrap(),
                vec![Token::Number(NumberToken::I8(127))]
            );

            assert_eq!(
                lex_from_str_without_location("255_i8").unwrap(),
                vec![Token::Number(NumberToken::I8(255))]
            );

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str_without_location("256_i8"),
                Err(Error::MessageWithLocation(
                    _,
                    Location {
                        unit: 0,
                        index: 0,
                        line: 0,
                        column: 0,
                        length: 6
                    }
                ))
            ));
        }

        // short
        {
            assert_eq!(
                lex_from_str_without_location("32767_i16").unwrap(),
                vec![Token::Number(NumberToken::I16(32767))]
            );

            assert_eq!(
                lex_from_str_without_location("65535_i16").unwrap(),
                vec![Token::Number(NumberToken::I16(65535))]
            );

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str_without_location("65536_i16"),
                Err(Error::MessageWithLocation(
                    _,
                    Location {
                        unit: 0,
                        index: 0,
                        line: 0,
                        column: 0,
                        length: 9
                    }
                ))
            ));
        }

        // int
        {
            assert_eq!(
                lex_from_str_without_location("2_147_483_647_i32").unwrap(),
                vec![Token::Number(NumberToken::I32(2_147_483_647i32 as u32))]
            );

            assert_eq!(
                lex_from_str_without_location("4_294_967_295_i32").unwrap(),
                vec![Token::Number(NumberToken::I32(u32::MAX))]
            );

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str_without_location("4_294_967_296_i32"),
                Err(Error::MessageWithLocation(
                    _,
                    Location {
                        unit: 0,
                        index: 0,
                        line: 0,
                        column: 0,
                        length: 17
                    }
                ))
            ));
        }

        // long
        {
            assert_eq!(
                lex_from_str_without_location("9_223_372_036_854_775_807_i64").unwrap(),
                vec![Token::Number(NumberToken::I64(
                    9_223_372_036_854_775_807i64 as u64
                )),]
            );

            assert_eq!(
                lex_from_str_without_location("18_446_744_073_709_551_615_i64").unwrap(),
                vec![Token::Number(NumberToken::I64(u64::MAX))]
            );

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str_without_location("18_446_744_073_709_551_616_i64"),
                Err(Error::MessageWithLocation(
                    _,
                    Location {
                        unit: 0,
                        index: 0,
                        line: 0,
                        column: 0,
                        length: 30
                    }
                ))
            ));
        }

        // float
        {
            assert_eq!(
                lex_from_str_without_location("3.402_823_5e+38_f32").unwrap(),
                vec![Token::Number(NumberToken::F32(3.402_823_5e38f32))]
            );

            assert_eq!(
                lex_from_str_without_location("1.175_494_4e-38_f32").unwrap(),
                vec![Token::Number(NumberToken::F32(1.175_494_4e-38f32))]
            );

            // err: overflow
            assert!(matches!(
                lex_from_str_without_location("3.4e39_f32"),
                Err(Error::MessageWithLocation(
                    _,
                    Location {
                        unit: 0,
                        index: 0,
                        line: 0,
                        column: 0,
                        length: 10
                    }
                ))
            ));
        }

        // double
        {
            assert_eq!(
                lex_from_str_without_location("1.797_693_134_862_315_7e+308_f64").unwrap(),
                vec![Token::Number(NumberToken::F64(
                    1.797_693_134_862_315_7e308_f64
                )),]
            );

            assert_eq!(
                lex_from_str_without_location("2.2250738585072014e-308_f64").unwrap(),
                vec![Token::Number(NumberToken::F64(2.2250738585072014e-308f64)),]
            );

            // err: overflow
            assert!(matches!(
                lex_from_str_without_location("1.8e309_f64"),
                Err(Error::MessageWithLocation(
                    _,
                    Location {
                        unit: 0,
                        index: 0,
                        line: 0,
                        column: 0,
                        length: 11
                    }
                ))
            ));
        }
    }

    #[test]
    fn test_lex_hex_number() {
        assert_eq!(
            lex_from_str_without_location("0xabcd").unwrap(),
            vec![Token::Number(NumberToken::I32(0xabcd))]
        );

        assert_eq!(
            lex_from_str_without_location("-0xaabb").unwrap(),
            vec![Token::Minus, Token::Number(NumberToken::I32(0xaabb))]
        );

        assert_eq!(
            lex_from_str_without_location("+0xccdd").unwrap(),
            vec![Token::Plus, Token::Number(NumberToken::I32(0xccdd))]
        );

        // location

        assert_eq!(
            lex_from_str("0xab 0xdef").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Number(NumberToken::I32(0xab)),
                    &Location::new_position(0, 0, 0, 0),
                    4
                ),
                TokenWithRange::from_position_and_length(
                    Token::Number(NumberToken::I32(0xdef)),
                    &Location::new_position(0, 5, 0, 5,),
                    5
                ),
            ]
        );

        // err: invalid char for hex number
        assert!(matches!(
            lex_from_str_without_location("0x1234xyz"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 6,
                    line: 0,
                    column: 6,
                    length: 0
                }
            ))
        ));

        // err: hex number overflow
        assert!(matches!(
            lex_from_str_without_location("0x1_0000_0000"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 0,
                    line: 0,
                    column: 0,
                    length: 13
                }
            ))
        ));

        // err: empty hex number
        assert!(matches!(
            lex_from_str_without_location("0x"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 0,
                    line: 0,
                    column: 0,
                    length: 2
                }
            ))
        ));
    }

    #[test]
    fn test_lex_hex_number_with_explicit_type() {
        // general
        {
            assert_eq!(
                lex_from_str_without_location("0x11i32").unwrap(),
                vec![Token::Number(NumberToken::I32(0x11))]
            );

            assert_eq!(
                lex_from_str_without_location("0x11_i32").unwrap(),
                vec![Token::Number(NumberToken::I32(0x11))]
            );

            assert_eq!(
                lex_from_str_without_location("0x11__i32").unwrap(),
                vec![Token::Number(NumberToken::I32(0x11))]
            );

            // location

            // "0x101_i16 0x103_u32" // text
            //  0123456789012345678  // index
            assert_eq!(
                lex_from_str("0x101_i16 0x103_i32").unwrap(),
                vec![
                    TokenWithRange::from_position_and_length(
                        Token::Number(NumberToken::I16(0x101)),
                        &Location::new_position(0, 0, 0, 0),
                        9
                    ),
                    TokenWithRange::from_position_and_length(
                        Token::Number(NumberToken::I32(0x103)),
                        &Location::new_position(0, 10, 0, 10),
                        9
                    ),
                ]
            );
        }

        // byte
        {
            assert_eq!(
                lex_from_str_without_location("0x7f_i8").unwrap(),
                vec![Token::Number(NumberToken::I8(0x7f_i8 as u8))]
            );

            assert_eq!(
                lex_from_str_without_location("0xff_i8").unwrap(),
                vec![Token::Number(NumberToken::I8(0xff_u8))]
            );

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str_without_location("0x1_ff_i8"),
                Err(Error::MessageWithLocation(
                    _,
                    Location {
                        unit: 0,
                        index: 0,
                        line: 0,
                        column: 0,
                        length: 9
                    }
                ))
            ));
        }

        // short
        {
            assert_eq!(
                lex_from_str_without_location("0x7fff_i16").unwrap(),
                vec![Token::Number(NumberToken::I16(0x7fff_i16 as u16))]
            );

            assert_eq!(
                lex_from_str_without_location("0xffff_i16").unwrap(),
                vec![Token::Number(NumberToken::I16(0xffff_u16))]
            );

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str_without_location("0x1_ffff_i16"),
                Err(Error::MessageWithLocation(
                    _,
                    Location {
                        unit: 0,
                        index: 0,
                        line: 0,
                        column: 0,
                        length: 12
                    }
                ))
            ));
        }

        // int
        {
            assert_eq!(
                lex_from_str_without_location("0x7fff_ffff_i32").unwrap(),
                vec![Token::Number(NumberToken::I32(0x7fff_ffff_i32 as u32))]
            );

            assert_eq!(
                lex_from_str_without_location("0xffff_ffff_i32").unwrap(),
                vec![Token::Number(NumberToken::I32(0xffff_ffff_u32))]
            );

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str_without_location("0x1_ffff_ffff_i32"),
                Err(Error::MessageWithLocation(
                    _,
                    Location {
                        unit: 0,
                        index: 0,
                        line: 0,
                        column: 0,
                        length: 17
                    }
                ))
            ));
        }

        // long
        {
            assert_eq!(
                lex_from_str_without_location("0x7fff_ffff_ffff_ffff_i64").unwrap(),
                vec![Token::Number(NumberToken::I64(
                    0x7fff_ffff_ffff_ffff_i64 as u64
                ))]
            );

            assert_eq!(
                lex_from_str_without_location("0xffff_ffff_ffff_ffff_i64").unwrap(),
                vec![Token::Number(NumberToken::I64(0xffff_ffff_ffff_ffff_u64))]
            );

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str_without_location("0x1_ffff_ffff_ffff_ffff_i64"),
                Err(Error::MessageWithLocation(
                    _,
                    Location {
                        unit: 0,
                        index: 0,
                        line: 0,
                        column: 0,
                        length: 27
                    }
                ))
            ));
        }

        // hex decimal
        {
            // note: this is not a hex floating pointer number
            assert_eq!(
                lex_from_str_without_location("0xaa_f32").unwrap(),
                vec![Token::Number(NumberToken::I32(0xaaf32))]
            );

            // note: this is not a hex floating pointer number
            assert_eq!(
                lex_from_str_without_location("0xbb_f64").unwrap(),
                vec![Token::Number(NumberToken::I32(0xbbf64))]
            );
        }
    }

    #[test]
    fn test_lex_hex_number_floating_point() {
        // default type is f64
        assert_eq!(
            lex_from_str_without_location("0x1.4p3").unwrap(),
            vec![Token::Number(NumberToken::F64(10f64))]
        );

        // 3.1415927f32
        assert_eq!(
            lex_from_str_without_location("0x1.921fb6p1f32").unwrap(),
            vec![Token::Number(NumberToken::F32(std::f32::consts::PI))]
        );

        // 2.718281828459045f64
        assert_eq!(
            lex_from_str_without_location("0x1.5bf0a8b145769p+1_f64").unwrap(),
            vec![Token::Number(NumberToken::F64(std::f64::consts::E))]
        );

        // https://observablehq.com/@jrus/hexfloat
        assert_eq!(
            lex_from_str_without_location("0x1.62e42fefa39efp-1_f64").unwrap(),
            vec![Token::Number(NumberToken::F64(std::f64::consts::LN_2))]
        );

        // location

        assert_eq!(
            lex_from_str("0x1.4p3").unwrap(),
            vec![TokenWithRange::from_position_and_length(
                Token::Number(NumberToken::F64(10f64)),
                &Location::new_position(0, 0, 0, 0),
                7
            )]
        );

        // err: missing the exponent
        assert!(matches!(
            lex_from_str_without_location("0x1.23"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 0,
                    line: 0,
                    column: 0,
                    length: 6
                }
            ))
        ));

        // err: multiple '.' (point)
        assert!(matches!(
            lex_from_str_without_location("0x1.2.3"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 5,
                    line: 0,
                    column: 5,
                    length: 0
                }
            ))
        ));

        // err: multiple 'p' (exponent)
        assert!(matches!(
            lex_from_str_without_location("0x1.2p3p4"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 7,
                    line: 0,
                    column: 7,
                    length: 0
                }
            ))
        ));

        // err: incorrect type (invalid dot '.' after 'p')
        assert!(matches!(
            lex_from_str_without_location("0x1.23p4.5"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 8,
                    line: 0,
                    column: 8,
                    length: 0
                }
            ))
        ));

        // err: incorrect type (invalid char 'i' after 'p')
        assert!(matches!(
            lex_from_str_without_location("0x1.23p4_i32"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 9,
                    line: 0,
                    column: 9,
                    length: 0
                }
            ))
        ));
    }

    #[test]
    fn test_lex_binary_number() {
        assert_eq!(
            lex_from_str_without_location("0b1100").unwrap(),
            vec![Token::Number(NumberToken::I32(0b1100))]
        );

        assert_eq!(
            lex_from_str_without_location("-0b1010").unwrap(),
            vec![Token::Minus, Token::Number(NumberToken::I32(0b1010))]
        );

        assert_eq!(
            lex_from_str_without_location("+0b0101").unwrap(),
            vec![Token::Plus, Token::Number(NumberToken::I32(0b0101))]
        );

        // location

        assert_eq!(
            lex_from_str("0b10 0b0101").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Number(NumberToken::I32(0b10)),
                    &Location::new_position(0, 0, 0, 0,),
                    4
                ),
                TokenWithRange::from_position_and_length(
                    Token::Number(NumberToken::I32(0b0101)),
                    &Location::new_position(0, 5, 0, 5,),
                    6
                ),
            ]
        );

        // err: does not support binary floating point
        assert!(matches!(
            lex_from_str_without_location("0b11.10"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 4,
                    line: 0,
                    column: 4,
                    length: 0
                }
            ))
        ));

        // err: binary number overflow
        assert!(matches!(
            lex_from_str_without_location("0b1_0000_0000_0000_0000_0000_0000_0000_0000"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 0,
                    line: 0,
                    column: 0,
                    length: 43
                }
            ))
        ));

        // err: invalid char for binary number
        assert!(matches!(
            lex_from_str_without_location("0b101xyz"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 5,
                    line: 0,
                    column: 5,
                    length: 0
                }
            ))
        ));

        // err: empty binary number
        assert!(matches!(
            lex_from_str_without_location("0b"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 0,
                    line: 0,
                    column: 0,
                    length: 2
                }
            ))
        ));
    }

    #[test]
    fn test_lex_binary_number_with_explicit_type() {
        // general
        {
            assert_eq!(
                lex_from_str_without_location("0b11i32").unwrap(),
                vec![Token::Number(NumberToken::I32(0b11))]
            );

            assert_eq!(
                lex_from_str_without_location("0b11_i32").unwrap(),
                vec![Token::Number(NumberToken::I32(0b11))]
            );

            assert_eq!(
                lex_from_str_without_location("0b11__i32").unwrap(),
                vec![Token::Number(NumberToken::I32(0b11))]
            );

            // location

            // "0b101_i16 0b1010_u32"
            //  01234567890123456789  // index
            assert_eq!(
                lex_from_str("0b101_i16 0b1010_i32").unwrap(),
                vec![
                    TokenWithRange::from_position_and_length(
                        Token::Number(NumberToken::I16(0b101)),
                        &Location::new_position(0, 0, 0, 0),
                        9
                    ),
                    TokenWithRange::from_position_and_length(
                        Token::Number(NumberToken::I32(0b1010)),
                        &Location::new_position(0, 10, 0, 10),
                        10
                    ),
                ]
            );
        }

        // byte
        {
            assert_eq!(
                lex_from_str_without_location("0b0111_1111_i8").unwrap(),
                vec![Token::Number(NumberToken::I8(0x7f_i8 as u8))]
            );

            assert_eq!(
                lex_from_str_without_location("0b1111_1111_i8").unwrap(),
                vec![Token::Number(NumberToken::I8(0xff_u8))]
            );

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str_without_location("0b1_1111_1111_i8"),
                Err(Error::MessageWithLocation(
                    _,
                    Location {
                        unit: 0,
                        index: 0,
                        line: 0,
                        column: 0,
                        length: 16
                    }
                ))
            ));
        }

        // short
        {
            assert_eq!(
                lex_from_str_without_location("0b0111_1111_1111_1111_i16").unwrap(),
                vec![Token::Number(NumberToken::I16(0x7fff_i16 as u16))]
            );

            assert_eq!(
                lex_from_str_without_location("0b1111_1111_1111_1111_i16").unwrap(),
                vec![Token::Number(NumberToken::I16(0xffff_u16))]
            );

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str_without_location("0b1_1111_1111_1111_1111_i16"),
                Err(Error::MessageWithLocation(
                    _,
                    Location {
                        unit: 0,
                        index: 0,
                        line: 0,
                        column: 0,
                        length: 27
                    }
                ))
            ));
        }

        // int
        {
            assert_eq!(
                lex_from_str_without_location("0b0111_1111_1111_1111__1111_1111_1111_1111_i32")
                    .unwrap(),
                vec![Token::Number(NumberToken::I32(0x7fff_ffff_i32 as u32))]
            );

            assert_eq!(
                lex_from_str_without_location("0b1111_1111_1111_1111__1111_1111_1111_1111_i32")
                    .unwrap(),
                vec![Token::Number(NumberToken::I32(0xffff_ffff_u32))]
            );

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str_without_location("0b1_1111_1111_1111_1111__1111_1111_1111_1111_i32"),
                Err(Error::MessageWithLocation(
                    _,
                    Location {
                        unit: 0,
                        index: 0,
                        line: 0,
                        column: 0,
                        length: 48
                    }
                ))
            ));
        }

        // long
        {
            assert_eq!(
                lex_from_str_without_location("0b0111_1111_1111_1111__1111_1111_1111_1111__1111_1111_1111_1111__1111_1111_1111_1111_i64").unwrap(),
                vec![Token::Number(NumberToken::I64(0x7fff_ffff_ffff_ffff_i64 as u64))]
            );

            assert_eq!(
                lex_from_str_without_location("0b1111_1111_1111_1111__1111_1111_1111_1111__1111_1111_1111_1111__1111_1111_1111_1111_i64").unwrap(),
                vec![Token::Number(NumberToken::I64(0xffff_ffff_ffff_ffff_u64))]
            );

            // err: unsigned overflow
            assert!(matches!(
                lex_from_str_without_location("0b1_1111_1111_1111_1111__1111_1111_1111_1111__1111_1111_1111_1111__1111_1111_1111_1111_i64"),
                Err(Error::MessageWithLocation(
                    _,
                    Location {
                        unit: 0,
                        index: 0,
                        line: 0,
                        column: 0,
                        length: 90
                    }
                ))
            ));
        }

        // err: does not support binary floating pointer number (invalid char 'f' for binary number)
        assert!(matches!(
            lex_from_str_without_location("0b11_f32"),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 5,
                    line: 0,
                    column: 5,
                    length: 0
                }
            ))
        ));
    }

    //     #[test]
    //     fn test_lex_char() {
    //         assert_eq!(
    //             lex_from_str_without_location("'a'").unwrap(),
    //             vec![Token::Char('a')]
    //         );
    //
    //         assert_eq!(
    //             lex_from_str_without_location("('a')").unwrap(),
    //             vec![Token::LeftParen, Token::Char('a'), Token::RightParen]
    //         );
    //
    //         assert_eq!(
    //             lex_from_str_without_location("'a' 'z'").unwrap(),
    //             vec![Token::Char('a'), Token::Char('z')]
    //         );
    //
    //         // CJK
    //         assert_eq!(
    //             lex_from_str_without_location("'文'").unwrap(),
    //             vec![Token::Char('文')]
    //         );
    //
    //         // emoji
    //         assert_eq!(
    //             lex_from_str_without_location("'😊'").unwrap(),
    //             vec![Token::Char('😊')]
    //         );
    //
    //         // escape char `\\`
    //         assert_eq!(
    //             lex_from_str_without_location("'\\\\'").unwrap(),
    //             vec![Token::Char('\\')]
    //         );
    //
    //         // escape char `\'`
    //         assert_eq!(
    //             lex_from_str_without_location("'\\\''").unwrap(),
    //             vec![Token::Char('\'')]
    //         );
    //
    //         // escape char `"`
    //         assert_eq!(
    //             lex_from_str_without_location("'\\\"'").unwrap(),
    //             vec![Token::Char('"')]
    //         );
    //
    //         // escape char `\t`
    //         assert_eq!(
    //             lex_from_str_without_location("'\\t'").unwrap(),
    //             vec![Token::Char('\t')]
    //         );
    //
    //         // escape char `\r`
    //         assert_eq!(
    //             lex_from_str_without_location("'\\r'").unwrap(),
    //             vec![Token::Char('\r')]
    //         );
    //
    //         // escape char `\n`
    //         assert_eq!(
    //             lex_from_str_without_location("'\\n'").unwrap(),
    //             vec![Token::Char('\n')]
    //         );
    //
    //         // escape char `\0`
    //         assert_eq!(
    //             lex_from_str_without_location("'\\0'").unwrap(),
    //             vec![Token::Char('\0')]
    //         );
    //
    //         // escape char, unicode
    //         assert_eq!(
    //             lex_from_str_without_location("'\\u{2d}'").unwrap(),
    //             vec![Token::Char('-')]
    //         );
    //
    //         // escape char, unicode
    //         assert_eq!(
    //             lex_from_str_without_location("'\\u{6587}'").unwrap(),
    //             vec![Token::Char('文')]
    //         );
    //
    //         // location
    //
    //         assert_eq!(
    //             lex_from_str("'a' '文'").unwrap(),
    //             vec![
    //                 TokenWithRange::from_position_and_length(
    //                     Token::Char('a'),
    //                     &Location::new_position(0, 0, 0, 0),
    //                     3
    //                 ),
    //                 TokenWithRange::from_position_and_length(
    //                     Token::Char('文'),
    //                     &Location::new_position(0, 4, 0, 4),
    //                     3
    //                 )
    //             ]
    //         );
    //
    //         assert_eq!(
    //             lex_from_str("'\\t'").unwrap(),
    //             vec![TokenWithRange::from_position_and_length(
    //                 Token::Char('\t'),
    //                 &Location::new_position(0, 0, 0, 0),
    //                 4
    //             )]
    //         );
    //
    //         assert_eq!(
    //             lex_from_str("'\\u{6587}'").unwrap(),
    //             vec![TokenWithRange::from_position_and_length(
    //                 Token::Char('文'),
    //                 &Location::new_position(0, 0, 0, 0),
    //                 10
    //             )]
    //         );
    //
    //         // err: empty char
    //         assert!(matches!(
    //             lex_from_str_without_location("''"),
    //             Err(Error::MessageWithLocation(
    //                 _,
    //                 Location {
    //                     unit: 0,
    //                     index: 0,
    //                     line: 0,
    //                     column: 0,
    //                     length: 2
    //                 }
    //             ))
    //         ));
    //
    //         // err: empty char, missing the char
    //         assert!(matches!(
    //             lex_from_str_without_location("'"),
    //             Err(Error::UnexpectedEndOfDocument(_))
    //         ));
    //
    //         // err: incomplete char, missing the right quote, encounter EOF
    //         assert!(matches!(
    //             lex_from_str_without_location("'a"),
    //             Err(Error::UnexpectedEndOfDocument(_))
    //         ));
    //
    //         // err: invalid char, expect the right quote, encounter another char
    //         assert!(matches!(
    //             lex_from_str_without_location("'ab"),
    //             Err(Error::MessageWithLocation(
    //                 _,
    //                 Location {
    //                     unit: 0,
    //                     index: 2,
    //                     line: 0,
    //                     column: 2,
    //                     length: 0
    //                 }
    //             ))
    //         ));
    //
    //         // err: invalid char, expect the right quote, encounter another char
    //         assert!(matches!(
    //             lex_from_str_without_location("'ab'"),
    //             Err(Error::MessageWithLocation(
    //                 _,
    //                 Location {
    //                     unit: 0,
    //                     index: 2,
    //                     line: 0,
    //                     column: 2,
    //                     length: 0
    //                 }
    //             ))
    //         ));
    //
    //         // err: unsupported escape char \v
    //         assert!(matches!(
    //             lex_from_str_without_location(r#"'\v'"#),
    //             Err(Error::MessageWithLocation(
    //                 _,
    //                 Location {
    //                     unit: 0,
    //                     index: 1,
    //                     line: 0,
    //                     column: 1,
    //                     length: 2
    //                 }
    //             ))
    //         ));
    //
    //         // err: unsupported hex escape "\x.."
    //         assert!(matches!(
    //             lex_from_str_without_location(r#"'\x33'"#),
    //             Err(Error::MessageWithLocation(
    //                 _,
    //                 Location {
    //                     unit: 0,
    //                     index: 1,
    //                     line: 0,
    //                     column: 1,
    //                     length: 2
    //                 }
    //             ))
    //         ));
    //
    //         // err: empty unicode escape string
    //         // "'\\u{}'"
    //         //  01 2345     // index
    //         assert!(matches!(
    //             lex_from_str_without_location("'\\u{}'"),
    //             Err(Error::MessageWithLocation(
    //                 _,
    //                 Location {
    //                     unit: 0,
    //                     index: 3,
    //                     line: 0,
    //                     column: 3,
    //                     length: 2
    //                 }
    //             ))
    //         ));
    //
    //         // err: invalid unicode code point, digits too much
    //         // "'\\u{1000111}'"
    //         //  01 234567890    // index
    //         assert!(matches!(
    //             lex_from_str_without_location("'\\u{1000111}'"),
    //             Err(Error::MessageWithLocation(
    //                 _,
    //                 Location {
    //                     unit: 0,
    //                     index: 3,
    //                     line: 0,
    //                     column: 3,
    //                     length: 8
    //                 }
    //             ))
    //         ));
    //
    //         // err: invalid unicode code point, code point out of range
    //         // "'\\u{123456}'"
    //         //  01 2345678901
    //         assert!(matches!(
    //             lex_from_str_without_location("'\\u{123456}'"),
    //             Err(Error::MessageWithLocation(
    //                 _,
    //                 Location {
    //                     unit: 0,
    //                     index: 3,
    //                     line: 0,
    //                     column: 3,
    //                     length: 8
    //                 }
    //             ))
    //         ));
    //
    //         // err: invalid char in the unicode escape sequence
    //         assert!(matches!(
    //             lex_from_str_without_location("'\\u{12mn}''"),
    //             Err(Error::MessageWithLocation(
    //                 _,
    //                 Location {
    //                     unit: 0,
    //                     index: 6,
    //                     line: 0,
    //                     column: 6,
    //                     length: 0
    //                 }
    //             ))
    //         ));
    //
    //         // err: missing the closed brace for unicode escape sequence
    //         assert!(matches!(
    //             lex_from_str_without_location("'\\u{1234'"),
    //             Err(Error::MessageWithLocation(
    //                 _,
    //                 Location {
    //                     unit: 0,
    //                     index: 8,
    //                     line: 0,
    //                     column: 8,
    //                     length: 0
    //                 }
    //             ))
    //         ));
    //
    //         // err: incomplete unicode escape sequence, encounter EOF
    //         assert!(matches!(
    //             lex_from_str_without_location("'\\u{1234"),
    //             Err(Error::UnexpectedEndOfDocument(_))
    //         ));
    //
    //         // err: missing left brace for unicode escape sequence
    //         assert!(matches!(
    //             lex_from_str_without_location("'\\u1234}'"),
    //             Err(Error::MessageWithLocation(
    //                 _,
    //                 Location {
    //                     unit: 0,
    //                     index: 3,
    //                     line: 0,
    //                     column: 3,
    //                     length: 0
    //                 }
    //             ))
    //         ));
    //     }

    #[test]
    fn test_lex_string() {
        assert_eq!(
            lex_from_str_without_location(r#""abc""#).unwrap(),
            vec![Token::new_string("abc")]
        );

        assert_eq!(
            lex_from_str_without_location(r#"("abc")"#).unwrap(),
            vec![
                Token::LeftParen,
                Token::new_string("abc"),
                Token::RightParen,
            ]
        );

        assert_eq!(
            lex_from_str_without_location(r#""abc" "xyz""#).unwrap(),
            vec![Token::new_string("abc"), Token::new_string("xyz")]
        );

        assert_eq!(
            lex_from_str_without_location("\"abc\"\n\n\"xyz\"").unwrap(),
            vec![
                Token::new_string("abc"),
                Token::NewLine,
                Token::NewLine,
                Token::new_string("xyz"),
            ]
        );

        // unicode
        assert_eq!(
            lex_from_str_without_location(
                r#"
                "abc文字😊"
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("abc文字😊"),
                Token::NewLine,
            ]
        );

        // empty string
        assert_eq!(
            lex_from_str_without_location("\"\"").unwrap(),
            vec![Token::new_string("")]
        );

        // escape chars
        assert_eq!(
            lex_from_str_without_location(
                r#"
                "\\\'\"\t\r\n\0\u{2d}\u{6587}"
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("\\\'\"\t\r\n\0-文"),
                Token::NewLine,
            ]
        );

        // location
        // "abc" "文字😊"
        // 01234567 8 9 0

        assert_eq!(
            lex_from_str(r#""abc" "文字😊""#).unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_string("abc"),
                    &Location::new_position(0, 0, 0, 0),
                    5
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_string("文字😊"),
                    &Location::new_position(0, 6, 0, 6),
                    5
                ),
            ]
        );

        // err: incomplete string, missing the closed quote
        assert!(matches!(
            lex_from_str_without_location("\"abc"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete string, missing the closed quote, ends with \n
        assert!(matches!(
            lex_from_str_without_location("\"abc\n"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete string, missing the closed quote, ends with whitespaces/other chars
        assert!(matches!(
            lex_from_str_without_location("\"abc\n   "),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: unsupported escape char \v
        assert!(matches!(
            lex_from_str_without_location(r#""abc\vxyz""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 4,
                    line: 0,
                    column: 4,
                    length: 2
                }
            ))
        ));

        // err: unsupported hex escape "\x.."
        assert!(matches!(
            lex_from_str_without_location(r#""abc\x33xyz""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 4,
                    line: 0,
                    column: 4,
                    length: 2
                }
            ))
        ));

        // err: empty unicode escape string
        // "abc\u{}"
        // 012345678    // index
        assert!(matches!(
            lex_from_str_without_location(r#""abc\u{}xyz""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 6,
                    line: 0,
                    column: 6,
                    length: 2
                }
            ))
        ));

        // err: invalid unicode code point, too much digits
        // "abc\u{1000111}xyz"
        // 0123456789023456789    // index
        assert!(matches!(
            lex_from_str_without_location(r#""abc\u{1000111}xyz""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 6,
                    line: 0,
                    column: 6,
                    length: 8
                }
            ))
        ));

        // err: invalid unicode code point, code point out of range
        // "abc\u{123456}xyz"
        // 012345678901234567
        assert!(matches!(
            lex_from_str_without_location(r#""abc\u{123456}xyz""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 6,
                    line: 0,
                    column: 6,
                    length: 8
                }
            ))
        ));

        // err: invalid char in the unicode escape sequence
        assert!(matches!(
            lex_from_str_without_location(r#""abc\u{12mn}xyz""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 9,
                    line: 0,
                    column: 9,
                    length: 0
                }
            ))
        ));

        // err: missing the right brace for unicode escape sequence
        assert!(matches!(
            lex_from_str_without_location(r#""abc\u{1234""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 11,
                    line: 0,
                    column: 11,
                    length: 0
                }
            ))
        ));

        // err: incomplete unicode escape sequence, encounter EOF
        assert!(matches!(
            lex_from_str_without_location(r#""abc\u{1234"#),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: missing left brace for unicode escape sequence
        assert!(matches!(
            lex_from_str_without_location(r#""abc\u1234}xyz""#),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 6,
                    line: 0,
                    column: 6,
                    length: 0
                }
            ))
        ));
    }

    #[test]
    fn test_lex_multiple_line_string() {
        assert_eq!(
            lex_from_str_without_location("\"abc\n    \n\n    \n\"").unwrap(),
            vec![Token::new_string("abc\n    \n\n    \n")]
        );

        assert_eq!(
            lex_from_str_without_location("\"abc\ndef\n    uvw\r\n\t  \txyz\"").unwrap(),
            vec![Token::new_string("abc\ndef\n    uvw\r\n\t  \txyz")]
        );

        // location

        assert_eq!(
            lex_from_str("\"abc\n    xyz\n\" \"foo\nbar\"").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_string("abc\n    xyz\n"),
                    &Location::new_position(0, 0, 0, 0),
                    14
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_string("foo\nbar"),
                    &Location::new_position(0, 15, 2, 2),
                    9
                )
            ]
        );

        // err: incomplete string, missing the closed quote, ends with \n
        assert!(matches!(
            lex_from_str_without_location("\"abc\n    \n\n    \n"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete string, missing the closed quote, whitespaces/other chars
        assert!(matches!(
            lex_from_str_without_location("\"abc\n    \n\n    \n   "),
            Err(Error::UnexpectedEndOfDocument(_))
        ));
    }

    #[test]
    fn test_lex_long_string() {
        // the tailing '\' should escapes the new-line chars
        assert_eq!(
            lex_from_str_without_location("\"abc\\\ndef\\\n    opq\\\r\n\t  \txyz\"").unwrap(),
            vec![Token::new_string("abcdefopqxyz")]
        );

        // the tailing '\' should escapes the new-line chars and trim the leading white-spaces
        assert_eq!(
            lex_from_str_without_location("\"\\\n  \t  \"").unwrap(),
            vec![Token::new_string("")]
        );

        // location

        assert_eq!(
            lex_from_str("\"abc\\\n\\\n    xyz\" \"\\\n\"").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_string("abcxyz"),
                    &Location::new_position(0, 0, 0, 0),
                    16
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_string(""),
                    &Location::new_position(0, 17, 2, 9),
                    4
                )
            ]
        );

        // err: incomplete string, missing the right quote, ends with \n
        assert!(matches!(
            lex_from_str_without_location("\"abc\\\n"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete string, missing the right quote, ends with whitespaces/other chars
        assert!(matches!(
            lex_from_str_without_location("\"abc\\\n    "),
            Err(Error::UnexpectedEndOfDocument(_))
        ));
    }

    #[test]
    fn test_lex_raw_string() {
        assert_eq!(
            lex_from_str_without_location(
                "r\"abc\ndef\n    uvw\r\n\t escape: \\r\\n\\t\\\\ unicode: \\u{1234} xyz\""
            )
            .unwrap(),
            vec![Token::new_string(
                "abc\ndef\n    uvw\r\n\t escape: \\r\\n\\t\\\\ unicode: \\u{1234} xyz"
            )]
        );

        // location

        assert_eq!(
            lex_from_str("r\"abc\n    xyz\" r\"foo\\nbar\"").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_string("abc\n    xyz"),
                    &Location::new_position(0, 0, 0, 0),
                    14
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_string("foo\\nbar"),
                    &Location::new_position(0, 15, 1, 9),
                    11
                )
            ]
        );

        // err: incomplete string, missing the right quote
        assert!(matches!(
            lex_from_str_without_location("r\"abc    "),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete string, missing the right quote, ends with \n
        assert!(matches!(
            lex_from_str_without_location("r\"abc\n"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete string, missing the right quote, ends with whitespaces/other chars
        assert!(matches!(
            lex_from_str_without_location("r\"abc\n   "),
            Err(Error::UnexpectedEndOfDocument(_))
        ));
    }

    #[test]
    fn test_lex_raw_string_with_hash_symbol() {
        assert_eq!(
            lex_from_str_without_location(
                "r#\"abc\ndef\n    uvw\r\n\t escape: \\r\\n\\t\\\\ unicode: \\u{1234} xyz quote: \"foo\"\"#"
            ).unwrap(),
            vec![Token::new_string(
                "abc\ndef\n    uvw\r\n\t escape: \\r\\n\\t\\\\ unicode: \\u{1234} xyz quote: \"foo\""
            )]
        );

        // location

        assert_eq!(
            lex_from_str("r#\"abc\n    xyz\"# r#\"foo\\nbar\"#").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_string("abc\n    xyz"),
                    &Location::new_position(0, 0, 0, 0),
                    16
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_string("foo\\nbar"),
                    &Location::new_position(0, 17, 1, 10),
                    13
                )
            ]
        );

        // err: incomplete string, missing the closed hash
        assert!(matches!(
            lex_from_str_without_location("r#\"abc    \""),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete string, missing the closed quote, ends with \n
        assert!(matches!(
            lex_from_str_without_location("r#\"abc\n"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete string, missing the closed quote, ends with whitespace/other chars
        assert!(matches!(
            lex_from_str_without_location("r#\"abc\nxyz"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));
    }

    #[test]
    fn test_lex_auto_trimmed_string() {
        assert_eq!(
            lex_from_str_without_location(
                r#"
                """
                one
                  two
                    three
                end
                """
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("one\n  two\n    three\nend"),
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str_without_location(
                r#"
                """
                one
              two
            three
                end
                """
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("    one\n  two\nthree\n    end"),
                Token::NewLine,
            ]
        );

        // contains empty lines
        assert_eq!(
            lex_from_str_without_location(
                r#"
                """
                    one\\\"\t\r\n\u{1234}

                    end
                """
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("one\\\\\\\"\\t\\r\\n\\u{1234}\n\nend"),
                Token::NewLine,
            ]
        );

        // including (""")
        assert_eq!(
            lex_from_str_without_location(
                r#"
                """
                    one"""
                    two
                """
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("one\"\"\"\ntwo"),
                Token::NewLine,
            ]
        );

        // inline
        assert_eq!(
            lex_from_str_without_location(
                r#"
                11 """
                    abc
                """ 13
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(NumberToken::I32(11)),
                Token::new_string("abc"),
                Token::Number(NumberToken::I32(13)),
                Token::NewLine,
            ]
        );

        // location

        assert_eq!(
            lex_from_str(
                r#"["""
    foo
    bar
""", """
    hello
    world
"""]"#
            )
            .unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::LeftBracket,
                    &Location::new_position(0, 0, 0, 0),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_string("foo\nbar"),
                    &Location::new_position(0, 1, 0, 1),
                    23
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comma,
                    &Location::new_position(0, 24, 3, 3),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_string("hello\nworld"),
                    &Location::new_position(0, 26, 3, 5),
                    27
                ),
                TokenWithRange::from_position_and_length(
                    Token::RightBracket,
                    &Location::new_position(0, 53, 6, 3),
                    1
                ),
            ]
        );

        // err: the content does not start on a new line
        assert!(matches!(
            lex_from_str_without_location(
                r#"
"""hello
"""
"#
            ),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 4,
                    line: 1,
                    column: 3,
                    length: 0
                }
            ))
        ));

        // err: missing the ending marker (the ending marker does not start on a new line)
        assert!(matches!(
            lex_from_str_without_location(
                r#"
"""
hello"""
"#
            ),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: missing the ending marker
        assert!(matches!(
            lex_from_str_without_location(
                r#"
"""
hello
world"#
            ),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: missing the ending marker, ends with \n
        assert!(matches!(
            lex_from_str_without_location(
                r#"
"""
hello
"#
            ),
            Err(Error::UnexpectedEndOfDocument(_))
        ));
    }

    #[test]
    fn test_lex_hex_byte_data() {
        assert_eq!(
            lex_from_str_without_location(
                r#"
                    h""
                    "#
            )
            .unwrap(),
            vec![Token::NewLine, Token::HexByteData(vec![]), Token::NewLine]
        );

        assert_eq!(
            lex_from_str_without_location(
                r#"
                    h"11"
                    "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::HexByteData(vec![0x11]),
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str_without_location(
                r#"
                    h"11 13 17 19"
                    "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::HexByteData(vec![0x11, 0x13, 0x17, 0x19]),
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str_without_location(
                "
                    h\"  11\t  13\r17\r\n  19\n  \"
                    "
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::HexByteData(vec![0x11, 0x13, 0x17, 0x19]),
                Token::NewLine,
            ]
        );

        // location

        assert_eq!(
            lex_from_str("h\"11 13\"").unwrap(),
            vec![TokenWithRange::from_position_and_length(
                Token::HexByteData(vec![0x11, 0x13]),
                &Location::new_position(0, 0, 0, 0),
                8
            )]
        );

        assert_eq!(
            lex_from_str("h\"11\" h\"13\"").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::HexByteData(vec![0x11]),
                    &Location::new_position(0, 0, 0, 0),
                    5
                ),
                TokenWithRange::from_position_and_length(
                    Token::HexByteData(vec![0x13]),
                    &Location::new_position(0, 6, 0, 6),
                    5
                )
            ]
        );

        // err: not enough digits
        assert!(matches!(
            lex_from_str_without_location("h\"11 1\""),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 6,
                    line: 0,
                    column: 6,
                    length: 0
                }
            ))
        ));

        // err: too much digits | no whitespace between two bytes
        assert!(matches!(
            lex_from_str_without_location("h\"11 1317\""),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 7,
                    line: 0,
                    column: 7,
                    length: 0
                }
            ))
        ));

        // err: invalid char for byte string
        assert!(matches!(
            lex_from_str_without_location("h\"11 1x\""),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 6,
                    line: 0,
                    column: 6,
                    length: 0
                }
            ))
        ));

        // err: invalid separator
        assert!(matches!(
            lex_from_str_without_location("h\"11-13\""),
            Err(Error::MessageWithLocation(
                _,
                Location {
                    unit: 0,
                    index: 4,
                    line: 0,
                    column: 4,
                    length: 0
                }
            ))
        ));

        // err: missing the close quote
        assert!(matches!(
            lex_from_str_without_location("h\"11 13"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: missing the close quote, ends with \n
        assert!(matches!(
            lex_from_str_without_location("h\"11 13\n"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: missing the close quote, ends with whitespaces/other chars
        assert!(matches!(
            lex_from_str_without_location("h\"11 13\n    "),
            Err(Error::UnexpectedEndOfDocument(_))
        ));
    }

    #[test]
    fn test_lex_line_comment() {
        assert_eq!(
            lex_from_str_without_location(
                r#"
                7 //11
                13 17// 19 23
                //  29
                31//    37
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(NumberToken::I32(7)),
                Token::Comment(Comment::Line("11".to_owned())),
                Token::NewLine,
                Token::Number(NumberToken::I32(13)),
                Token::Number(NumberToken::I32(17)),
                Token::Comment(Comment::Line(" 19 23".to_owned())),
                Token::NewLine,
                Token::Comment(Comment::Line("  29".to_owned())),
                Token::NewLine,
                Token::Number(NumberToken::I32(31)),
                Token::Comment(Comment::Line("    37".to_owned())),
                Token::NewLine,
            ]
        );

        // location

        assert_eq!(
            lex_from_str("foo // bar").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_name("foo"),
                    &Location::new_position(0, 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comment(Comment::Line(" bar".to_owned())),
                    &Location::new_position(0, 4, 0, 4),
                    6
                ),
            ]
        );

        assert_eq!(
            lex_from_str("abc // def\n// xyz\n").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_name("abc"),
                    &Location::new_position(0, 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comment(Comment::Line(" def".to_owned())),
                    &Location::new_position(0, 4, 0, 4),
                    6
                ),
                TokenWithRange::from_position_and_length(
                    Token::NewLine,
                    &Location::new_position(0, 10, 0, 10),
                    1
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comment(Comment::Line(" xyz".to_owned())),
                    &Location::new_position(0, 11, 1, 0),
                    6
                ),
                TokenWithRange::from_position_and_length(
                    Token::NewLine,
                    &Location::new_position(0, 17, 1, 6),
                    1
                ),
            ]
        );
    }

    #[test]
    fn test_lex_block_comment() {
        assert_eq!(
            lex_from_str_without_location(
                r#"
                7 /* 11 13 */ 17
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(NumberToken::I32(7)),
                Token::Comment(Comment::Block(" 11 13 ".to_owned())),
                Token::Number(NumberToken::I32(17)),
                Token::NewLine,
            ]
        );

        // nested block comment
        assert_eq!(
            lex_from_str_without_location(
                r#"
                7 /* 11 /* 13 */ 17 */ 19
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(NumberToken::I32(7)),
                Token::Comment(Comment::Block(" 11 /* 13 */ 17 ".to_owned())),
                Token::Number(NumberToken::I32(19)),
                Token::NewLine,
            ]
        );

        // line comment chars "//" within the block comment
        assert_eq!(
            lex_from_str_without_location(
                r#"
                7 /* 11 // 13 17 */ 19
                "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(NumberToken::I32(7)),
                Token::Comment(Comment::Block(" 11 // 13 17 ".to_owned())),
                Token::Number(NumberToken::I32(19)),
                Token::NewLine,
            ]
        );

        // location

        assert_eq!(
            lex_from_str("foo /* hello */ bar").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::new_name("foo"),
                    &Location::new_position(0, 0, 0, 0),
                    3
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comment(Comment::Block(" hello ".to_owned())),
                    &Location::new_position(0, 4, 0, 4),
                    11
                ),
                TokenWithRange::from_position_and_length(
                    Token::new_name("bar"),
                    &Location::new_position(0, 16, 0, 16),
                    3
                ),
            ]
        );

        assert_eq!(
            lex_from_str("/* abc\nxyz */ /* hello */").unwrap(),
            vec![
                TokenWithRange::from_position_and_length(
                    Token::Comment(Comment::Block(" abc\nxyz ".to_owned())),
                    &Location::new_position(0, 0, 0, 0),
                    13
                ),
                TokenWithRange::from_position_and_length(
                    Token::Comment(Comment::Block(" hello ".to_owned())),
                    &Location::new_position(0, 14, 1, 7),
                    11
                ),
            ]
        );

        // err: incomplete, missing "*/"
        assert!(matches!(
            lex_from_str_without_location("7 /* 11"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete, missing "*/", ends with \n
        assert!(matches!(
            lex_from_str_without_location("7 /* 11\n"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete, unpaired, missing "*/"
        assert!(matches!(
            lex_from_str_without_location("a /* b /* c */"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));

        // err: incomplete, unpaired, missing "*/", ends with \n
        assert!(matches!(
            lex_from_str_without_location("a /* b /* c */\n"),
            Err(Error::UnexpectedEndOfDocument(_))
        ));
    }
}
