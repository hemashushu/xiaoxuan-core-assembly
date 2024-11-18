// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{
    ast::{DataNode, ExternalNode, FunctionNode, ModuleNode, UseNode},
    error::Error,
    location::Location,
    peekableiter::PeekableIter,
    token::{Token, TokenWithRange},
};

pub const PARSER_PEEK_TOKEN_MAX_COUNT: usize = 4;

pub struct Parser<'a> {
    upstream: &'a mut PeekableIter<'a, TokenWithRange>,
    last_range: Location,
}

impl<'a> Parser<'a> {
    fn new(upstream: &'a mut PeekableIter<'a, TokenWithRange>) -> Self {
        Self {
            upstream,
            last_range: Location::new_range(0, 0, 0, 0, 0),
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        match self.upstream.next() {
            Some(TokenWithRange { token, range }) => {
                self.last_range = range;
                Some(token)
            }
            None => None,
        }
    }

    fn peek_range(&self, offset: usize) -> Option<&Location> {
        match self.upstream.peek(offset) {
            Some(TokenWithRange { range, .. }) => Some(range),
            None => None,
        }
    }

    fn peek_token(&self, offset: usize) -> Option<&Token> {
        match self.upstream.peek(offset) {
            Some(TokenWithRange { token, .. }) => Some(token),
            None => None,
        }
    }

    fn peek_token_and_equals(&self, offset: usize, expected_token: &Token) -> bool {
        matches!(
            self.upstream.peek(offset),
            Some(TokenWithRange { token, .. }) if token == expected_token)
    }

    // consume '\n' if it exists.
    fn consume_new_line_if_exist(&mut self) -> bool {
        match self.peek_token(0) {
            Some(Token::NewLine) => {
                self.next_token();
                true
            }
            _ => false,
        }
    }

    // consume '\n' or ',' if they exist.
    fn consume_new_line_or_comma_if_exist(&mut self) -> bool {
        match self.peek_token(0) {
            Some(Token::NewLine | Token::Comma) => {
                self.next_token();
                true
            }
            _ => false,
        }
    }

    fn expect_token(
        &mut self,
        expected_token: &Token,
        token_description: &str,
    ) -> Result<(), Error> {
        match self.next_token() {
            Some(token) => {
                if &token == expected_token {
                    Ok(())
                } else {
                    Err(Error::MessageWithLocation(
                        format!("Expect token: {}.", token_description),
                        self.last_range.get_position_by_range_start(),
                    ))
                }
            }
            None => Err(Error::UnexpectedEndOfDocument(format!(
                "Expect token: {}.",
                token_description
            ))),
        }
    }

    //     fn expect_identifier(&mut self) -> Result<String, Error> {
    //         match self.peek_token(0) {
    //             Some(Token::Identifier(s)) => {
    //                 let id = s.to_owned();
    //                 self.next_token();
    //                 Ok(id)
    //             }
    //             Some(_) => Err(Error::MessageWithLocation(
    //                 "Expect an identifier.".to_owned(),
    //                 self.last_range.get_position_by_range_start(),
    //             )),
    //             None => Err(Error::UnexpectedEndOfDocument(
    //                 "Expect an identifier.".to_owned(),
    //             )),
    //         }
    //     }
    //
    //     fn expect_number(&mut self) -> Result<usize, Error> {
    //         match self.peek_token(0) {
    //             Some(Token::Number(n)) => {
    //                 let num = *n;
    //                 self.next_token();
    //                 Ok(num)
    //             }
    //             Some(_) => Err(Error::MessageWithLocation(
    //                 "Expect a number.".to_owned(),
    //                 self.last_range.get_position_by_range_start(),
    //             )),
    //             None => Err(Error::UnexpectedEndOfDocument(
    //                 "Expect a number.".to_owned(),
    //             )),
    //         }
    //     }
}

impl<'a> Parser<'a> {
    pub fn parse_module(&mut self, name_path: &str) -> Result<ModuleNode, Error> {
        let mut uses: Vec<UseNode> = vec![];
        let mut externals: Vec<ExternalNode> = vec![];
        let mut datas: Vec<DataNode> = vec![];
        let mut functions: Vec<FunctionNode> = vec![];

        while let Some(token) = self.peek_token(0) {
            match token {
                Token::NewLine => todo!(),
                Token::Comma => todo!(),
                Token::Colon => todo!(),
                Token::Equal => todo!(),
                Token::Arrow => todo!(),
                Token::Plus => todo!(),
                Token::Minus => todo!(),
                Token::LeftBrace => todo!(),
                Token::RightBrace => todo!(),
                Token::LeftBracket => todo!(),
                Token::RightBracket => todo!(),
                Token::LeftParen => todo!(),
                Token::RightParen => todo!(),
                Token::NamePath(_) => todo!(),
                Token::Name(_) => todo!(),
                Token::Keyword(_) => todo!(),
                Token::DataType(_) => todo!(),
                Token::Number(number_token) => todo!(),
                Token::String(_) => todo!(),
                Token::Comment(comment) => todo!(),
            }
        }

        let module_node = ModuleNode {
            name_path: name_path.to_owned(),
            uses: vec![],
            externals: vec![],
            datas: vec![],
            functions: vec![],
        };

        Ok(module_node)
    }
}
