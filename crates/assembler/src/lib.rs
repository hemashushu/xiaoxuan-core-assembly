// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use std::{any::Any, fmt::Display};

pub mod assembler;
pub mod binarygen;
pub mod linker;
pub mod preprocessor;
pub mod utils;

pub const UNREACHABLE_CODE_NO_DEFAULT_ARM: u32 = 0x100;

#[derive(Debug)]
pub struct AssembleError {
    pub message: String,
}

impl AssembleError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }
}

impl Display for AssembleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Assemble error: {}", self.message)
    }
}

impl std::error::Error for AssembleError {}
