// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use anc_isa::DataSectionType;

use crate::{
    ast::{DataNode, ExternalNode, FunctionNode, ModuleNode, UseNode},
    error::Error,
    lexer::lex_from_str,
    location::Location,
    normalizer::{clean, normalize},
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

    fn expect_token(&self, offset: usize, expected_token: &Token) -> bool {
        matches!(
            self.peek_token(offset),
            Some(token) if token == expected_token)
    }

    fn expect_keyword(&self, offset: usize, expected_keyword: &str) -> bool {
        matches!(self.peek_token(offset), Some(Token::Keyword(keyword)) if keyword == expected_keyword )
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

    fn consume_new_line_or_comma(&mut self) -> Result<(), Error> {
        match self.peek_token(0) {
            Some(Token::NewLine | Token::Comma) => {
                self.next_token();
                Ok(())
            }
            Some(_) => Err(Error::MessageWithLocation(
                "Expect a comma or new-line.".to_owned(),
                self.peek_range(0).unwrap().get_position_by_range_start(),
            )),
            None => Err(Error::UnexpectedEndOfDocument(
                "Expect a comma or new-line.".to_owned(),
            )),
        }
    }

    //     fn consume_identifier(&mut self) -> Result<String, Error> {
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

    fn consume_token(
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

    fn consume_name_path(&mut self) -> Result<String, Error> {
        match self.next_token() {
            Some(Token::NamePath(s)) => Ok(s),
            Some(_) => Err(Error::MessageWithLocation(
                "Expect a name path.".to_owned(),
                self.last_range.get_position_by_range_start(),
            )),
            None => Err(Error::UnexpectedEndOfDocument(
                "Expect a name path.".to_owned(),
            )),
        }
    }

    fn consume_name(&mut self) -> Result<String, Error> {
        match self.next_token() {
            Some(Token::Name(s)) => Ok(s),
            Some(_) => Err(Error::MessageWithLocation(
                "Expect a name.".to_owned(),
                self.last_range.get_position_by_range_start(),
            )),
            None => Err(Error::UnexpectedEndOfDocument("Expect a name.".to_owned())),
        }
    }
}

impl<'a> Parser<'a> {
    pub fn parse_module_node(&mut self, name_path: &str) -> Result<ModuleNode, Error> {
        let mut uses: Vec<UseNode> = vec![];
        let mut externals: Vec<ExternalNode> = vec![];
        let mut datas: Vec<DataNode> = vec![];
        let mut functions: Vec<FunctionNode> = vec![];

        while let Some(token) = self.peek_token(0) {
            match token {
                Token::Keyword(keyword) if keyword == "use" => {
                    // use statement
                    uses.push(self.parse_use_node()?);
                }
                Token::Keyword(keyword) if keyword == "external" => {
                    // external statement
                    externals.push(self.parse_external_node()?);
                }
                Token::Keyword(keyword) if keyword == "data" => {
                    // private read-write data statement
                    datas.push(self.parse_data_node(false, DataSectionType::ReadWrite)?);
                }
                Token::Keyword(keyword)
                    if (keyword == "readonly" || keyword == "uninit")
                        && self.expect_keyword(1, "data") =>
                {
                    // private {readonly|uninit} data statement
                    let data_section_type = if keyword == "readonly" {
                        DataSectionType::ReadOnly
                    } else {
                        DataSectionType::Uninit
                    };

                    self.next_token(); // consume 'readonly' or 'uninit'

                    datas.push(self.parse_data_node(false, data_section_type)?);
                }
                Token::Keyword(keyword) if keyword == "pub" && self.expect_keyword(1, "data") => {
                    // public read-write data statement
                    self.next_token(); // consume 'pub'

                    datas.push(self.parse_data_node(true, DataSectionType::ReadWrite)?);
                }
                Token::Keyword(keyword)
                    if keyword == "pub"
                        && (self.expect_keyword(1, "readonly")
                            || self.expect_keyword(1, "uninit"))
                        && self.expect_keyword(2, "data") =>
                {
                    // public {readonly|uninit} data statement
                    let data_section_type = if keyword == "readonly" {
                        DataSectionType::ReadOnly
                    } else {
                        DataSectionType::Uninit
                    };

                    self.next_token(); // consume 'pub'
                    self.next_token(); // consume 'readonly' or 'uninit'

                    datas.push(self.parse_data_node(true, data_section_type)?);
                }
                Token::Keyword(keyword) if keyword == "fn" => {
                    // private fn statement
                    functions.push(self.parse_fn_node(false)?);
                }
                Token::Keyword(keyword) if keyword == "pub" && self.expect_keyword(1, "fn") => {
                    // public fn statement
                    self.next_token(); // consume 'pub'

                    functions.push(self.parse_fn_node(true)?);
                }
                _ => {
                    return Err(Error::MessageWithLocation(
                        "Unexpected token.".to_owned(),
                        self.peek_range(0).unwrap().get_position_by_range_start(),
                    ));
                }
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

    fn parse_use_node(&mut self) -> Result<UseNode, Error> {
        // use ... [as ...] ?  //
        // ^                ^__// to here
        // |-------------------// current token, validated

        self.next_token(); // consume 'use'
        self.consume_new_line_if_exist();

        let name_path = self.consume_name_path()?;

        let alias_name = if self.expect_keyword(0, "as") {
            self.next_token(); // consume 'as'
            self.consume_new_line_if_exist();

            let name = self.consume_name()?;
            Some(name)
        } else {
            None
        };

        Ok(UseNode {
            name_path,
            alias_name,
        })
    }

    fn parse_external_node(&mut self) -> Result<ExternalNode, Error> {
        todo!()
    }

    fn parse_data_node(
        &mut self,
        is_public: bool,
        data_section_type: DataSectionType,
    ) -> Result<DataNode, Error> {
        todo!()
    }

    fn parse_fn_node(&mut self, is_public: bool) -> Result<FunctionNode, Error> {
        todo!()
    }
}

pub fn parse_from_str(source_code: &str, name_path: &str) -> Result<ModuleNode, Error> {
    let tokens = lex_from_str(source_code)?;
    let clean_tokens = clean(tokens);
    let normalized_tokens = normalize(clean_tokens)?;
    let mut token_iter = normalized_tokens.into_iter();
    let mut peekable_token_iter = PeekableIter::new(&mut token_iter, PARSER_PEEK_TOKEN_MAX_COUNT);
    let mut parser = Parser::new(&mut peekable_token_iter);
    parser.parse_module_node(name_path)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::printer::print_to_string;

    use super::parse_from_str;

    fn format(s: &str) -> String {
        let module_node = parse_from_str(s, "").unwrap();
        print_to_string(&module_node)
    }

    #[test]
    fn test_parse_use_statement() {

        // todo
    }
}
