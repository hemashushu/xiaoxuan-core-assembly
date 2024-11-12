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
    Arrow,

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
    // used for object field/key name
    NamePath(String),
    Name(String),
    Keyword(String),
    DataType(String), // e.g. "i64", "i32"

    Number(NumberToken),
    Char(char),
    String(String),
    ByteData(Vec<u8>),

    Comment(Comment),
}

#[derive(Debug, PartialEq)]
pub enum NumberToken {
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
    I16,
    I32,
    I64,
    F32,
    F64,
}

impl NumberType {
    pub fn from_str(s: &str) -> Result<Self, String> {
        let t = match s {
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
