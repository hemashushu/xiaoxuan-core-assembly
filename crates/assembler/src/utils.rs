// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancasm_parser::{
    lexer::{filter, lex},
    parser::parse,
    peekable_iterator::PeekableIterator,
};

use crate::{
    assembler::assemble_merged_module_node, imagegenerator::generate_module_image_binary, linker::link,
    preprocessor::merge_and_canonicalize_submodule_nodes,
};

pub fn helper_generate_module_image_binary_from_str(source: &str) -> Vec<u8> {
    let mut chars = source.chars();
    let mut char_iter = PeekableIterator::new(&mut chars, 3);
    let all_tokens = lex(&mut char_iter).unwrap();
    let effective_tokens = filter(&all_tokens);
    let mut token_iter = effective_tokens.into_iter();
    let mut peekable_token_iter = PeekableIterator::new(&mut token_iter, 2);

    let module_node = parse(&mut peekable_token_iter, None).unwrap();
    let merged_module_node =
        merge_and_canonicalize_submodule_nodes(&[module_node], None, None).unwrap();

    let (module_entry, _) = assemble_merged_module_node(&merged_module_node).unwrap();
    let module_entries = vec![&module_entry];

    // let program_settings = ProgramSettings::default();
    let index_entry = link(&module_entries, 0).unwrap();
    generate_module_image_binary(&module_entry, Some(&index_entry)).unwrap()
}
