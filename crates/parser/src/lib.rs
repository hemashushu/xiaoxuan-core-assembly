// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::location::Location;
use std::fmt::Display;

mod charwithposition;
mod errorprinter;
mod lexer;
mod location;
mod normalizer;
mod peekableiter;
mod token;

pub mod parser;

pub const NAME_PATH_SEPARATOR: &str = "::";

#[derive(Debug, PartialEq, Clone)]
pub enum ParserError {
    Message(String),
    UnexpectedEndOfDocument(String),

    // note that the "index" (and the result of "index+length") may exceed
    // the last index of string, for example, the "char incomplete" error raised by a string `'a`,
    // which index is 2.
    MessageWithLocation(String, Location),
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParserError::Message(msg) => f.write_str(msg),
            ParserError::UnexpectedEndOfDocument(detail) => {
                writeln!(f, "Unexpected to reach the end of document.")?;
                write!(f, "{}", detail)
            }
            ParserError::MessageWithLocation(detail, location) => {
                writeln!(
                    f,
                    "Error at line: {}, column: {}",
                    location.line + 1,
                    location.column + 1
                )?;
                write!(f, "{}", detail)
            }
        }
    }
}

impl std::error::Error for ParserError {}
