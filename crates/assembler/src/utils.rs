// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancasm_parser::{lexer::lex, parser::parse, peekable_iterator::PeekableIterator};

use crate::{
    assembler::assemble_merged_module_node, binarygen::generate_module_image_binary, linker::link,
    preprocessor::merge_and_canonicalize_submodule_nodes,
};

pub fn helper_generate_module_image_binary_from_str(source: &str) -> Vec<u8> {
    let mut chars = source.chars();
    let mut char_iter = PeekableIterator::new(&mut chars, 2);
    let mut tokens = lex(&mut char_iter).unwrap().into_iter();
    let mut token_iter = PeekableIterator::new(&mut tokens, 2);

    let module_node = parse(&mut token_iter).unwrap();
    let merged_module_node = merge_and_canonicalize_submodule_nodes(&[module_node]).unwrap();

    let module_entry = assemble_merged_module_node(&merged_module_node).unwrap();
    let module_entries = vec![&module_entry];

    // let program_settings = ProgramSettings::default();
    let index_entry = link(&module_entries).unwrap();
    generate_module_image_binary(&module_entry, Some(&index_entry)).unwrap()
}
