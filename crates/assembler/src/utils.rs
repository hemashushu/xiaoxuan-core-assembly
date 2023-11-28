// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_parser::{lexer::lex, parser::parse, peekable_iterator::PeekableIterator};
use ancvm_program::program_settings::ProgramSettings;

use crate::{
    assembler::assemble_merged_module_node, linker::generate_image_binaries,
    preprocessor::merge_submodule_nodes,
};

pub fn helper_generate_single_module_image_binary_from_assembly(source: &str) -> Vec<Vec<u8>> {
    let mut chars = source.chars();
    let mut char_iter = PeekableIterator::new(&mut chars, 2);
    let mut tokens = lex(&mut char_iter).unwrap().into_iter();
    let mut token_iter = PeekableIterator::new(&mut tokens, 2);

    let module_node = parse(&mut token_iter).unwrap();
    let merged_module_node = merge_submodule_nodes(&[module_node]).unwrap();

    let module_entry = assemble_merged_module_node(&merged_module_node).unwrap();
    let program_settings = ProgramSettings::default();
    generate_image_binaries(&vec![module_entry], &program_settings).unwrap()
}
