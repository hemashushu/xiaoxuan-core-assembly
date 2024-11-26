// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

use crate::location::Location;

#[derive(Debug, PartialEq)]
pub enum Token {
    // includes `\n` and `\r\n`
    NewLine,

    // `,`
    Comma,

    // `:`
    Colon,

    // `=`
    Equal,

    // "->"
    RightArrow,

    // `+`, for positive numbers
    Plus,
    // `-`, for negative numbers
    Minus,

    // {
    LeftBrace,
    // }
    RightBrace,
    // [
    LeftBracket,
    // ]
    RightBracket,
    // (
    LeftParen,
    // )
    RightParen,

    // [a-zA-Z0-9_] and '\u{a0}' - '\u{d7ff}' and '\u{e000}' - '\u{10ffff}'
    // e.g. "foo", "bar", "data0", "data_"
    Name(String),

    // name with "::"
    // e.g. "std::memory::copy"
    FullName(String),

    // e.g. "pub", "data", "readonly", "fn"
    Keyword(String),

    // the name of data type.
    // e.g. "i64", "i32", "byte"
    // it does not include the type details, such as
    // the length and alignment of byte, e.g. "byte[1024, align=8]".
    DataTypeName(String),

    Number(NumberToken),
    String(String),
    HexByteData(Vec<u8>),
    Comment(Comment),
}

#[derive(Debug, PartialEq)]
pub enum NumberToken {
    I8(u8),
    I16(u16),
    I32(u32),
    I64(u64),
    F32(f32),
    F64(f64),
}

#[derive(Debug, PartialEq)]
pub enum Comment {
    // `//...`
    // note that the trailing '\n' or '\r\n' does not belong to line comment
    Line(String),

    // `/*...*/`
    Block(String),
}

#[derive(Debug, PartialEq)]
pub enum NumberType {
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
}

impl NumberType {
    pub fn from_str(s: &str) -> Result<Self, String> {
        let t = match s {
            "i8" => NumberType::I8,
            "i16" => NumberType::I16,
            "i32" => NumberType::I32,
            "i64" => NumberType::I64,
            "f32" => NumberType::F32,
            "f64" => NumberType::F64,
            _ => {
                return Err(format!("Invalid number type \"{}\".", s));
            }
        };

        Ok(t)
    }
}

impl Display for NumberType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumberType::I8 => write!(f, "i8"),
            NumberType::I16 => write!(f, "i16"),
            NumberType::I32 => write!(f, "i32"),
            NumberType::I64 => write!(f, "i64"),
            NumberType::F32 => write!(f, "f32"),
            NumberType::F64 => write!(f, "f64"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct TokenWithRange {
    pub token: Token,
    pub range: Location,
}

impl TokenWithRange {
    pub fn new(token: Token, range: Location) -> Self {
        Self { token, range }
    }

    pub fn from_position_and_length(token: Token, position: &Location, length: usize) -> Self {
        Self {
            token,
            range: Location::from_position_and_length(position, length),
        }
    }
}
