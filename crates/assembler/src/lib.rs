// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::fmt::Display;

pub mod assembler;
pub mod entry;
pub mod object_writer;

// https://doc.rust-lang.org/reference/conditional-compilation.html#debug_assertions
// https://doc.rust-lang.org/reference/conditional-compilation.html#test
#[cfg(debug_assertions)]
pub mod utils;

#[derive(Debug)]
pub struct AssemblerError {
    pub message: String,
}

impl AssemblerError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }
}

impl Display for AssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Assembler error: {}", self.message)
    }
}

impl std::error::Error for AssemblerError {}
