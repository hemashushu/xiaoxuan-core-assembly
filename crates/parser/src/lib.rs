// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

pub mod ast;
mod charposition;
mod error;
mod errorprinter;
mod lexer;
mod location;
mod peekableiter;
mod token;
// pub mod parser;

pub const NAME_PATH_SEPARATOR: &str = "::";

#[derive(Debug)]
pub struct ParserError {
    pub message: String,
}

impl ParserError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error: {}", self.message)
    }
}

impl std::error::Error for ParserError {}
