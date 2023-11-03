// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use ancvm_assembler::{assembler::assemble_module_node, linker::generate_image_binaries};
use ancvm_assembly_parser::{
    instruction_kind::init_instruction_kind_table, lexer::lex, parser::parse,
    peekable_iterator::PeekableIterator,
};
use ancvm_program::program_settings::ProgramSettings;

pub fn assemble_single_module(source: &str) -> Vec<Vec<u8>> {
    init_instruction_kind_table();

    let mut chars = source.chars();
    let mut char_iter = PeekableIterator::new(&mut chars, 2);
    let mut tokens = lex(&mut char_iter).unwrap().into_iter();
    let mut token_iter = PeekableIterator::new(&mut tokens, 2);
    let module_node = parse(&mut token_iter).unwrap();

    let module_entry = assemble_module_node(&module_node).unwrap();
    let program_settings = ProgramSettings::default();
    generate_image_binaries(&vec![module_entry], &program_settings).unwrap()
}
