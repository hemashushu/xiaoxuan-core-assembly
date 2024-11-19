// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use anc_isa::DataSectionType;

use crate::{
    ast::{
        DataNode, DeclareDataType, ExternalDataNode, ExternalDataType, ExternalFunctionNode,
        ExternalNode, FunctionDataType, FunctionNode, ModuleNode, UseNode,
    },
    error::Error,
    lexer::lex_from_str,
    location::Location,
    normalizer::{clean, normalize},
    peekableiter::PeekableIter,
    token::{NumberToken, Token, TokenWithRange},
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

    fn consume_new_line_or_eof(&mut self) -> Result<(), Error> {
        match self.peek_token(0) {
            Some(Token::NewLine) => {
                self.next_token();
                Ok(())
            }
            Some(_) => Err(Error::MessageWithLocation(
                "Expect a new-line.".to_owned(),
                self.peek_range(0).unwrap().get_position_by_range_start(),
            )),
            None => Ok(()),
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

    fn consume_number_i32(&mut self) -> Result<u32, Error> {
        match self.next_token() {
            Some(Token::Number(NumberToken::I32(n))) => Ok(n),
            Some(_) => Err(Error::MessageWithLocation(
                "Expect an i32 number.".to_owned(),
                self.last_range.get_position_by_range_start(),
            )),
            None => Err(Error::UnexpectedEndOfDocument(
                "Expect an i32 number.".to_owned(),
            )),
        }
    }

    // '('
    fn consume_left_paren(&mut self) -> Result<(), Error> {
        self.consume_token(&Token::LeftParen, "left parenthese")
    }

    // ')'
    fn consume_right_paren(&mut self) -> Result<(), Error> {
        self.consume_token(&Token::RightParen, "right parenthese")
    }

    // '['
    fn consume_left_bracket(&mut self) -> Result<(), Error> {
        self.consume_token(&Token::LeftBracket, "left bracket")
    }

    // ']'
    fn consume_right_bracket(&mut self) -> Result<(), Error> {
        self.consume_token(&Token::RightBracket, "right parenthese")
    }

    // '='
    fn consume_equal(&mut self) -> Result<(), Error> {
        self.consume_token(&Token::Equal, "equal sign")
    }

    // ':'
    fn consume_colon(&mut self) -> Result<(), Error> {
        self.consume_token(&Token::Colon, "colon sign")
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
            uses,
            externals,
            datas,
            functions,
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

        // consume trail new line
        self.consume_new_line_or_eof()?;

        let node = UseNode {
            name_path,
            alias_name,
        };
        Ok(node)
    }

    fn parse_external_node(&mut self) -> Result<ExternalNode, Error> {
        // external {fn|data} ... ?  //
        // ^                      ^__// to here
        // |-------------------------// current token, validated

        self.next_token(); // consume 'external'

        match self.peek_token(0) {
            Some(Token::Keyword(n)) if n == "fn" => {
                // external fn ...
                let function_node = self.parse_external_function_node()?;
                Ok(ExternalNode::Function(function_node))
            }
            Some(Token::Keyword(n)) if n == "data" => {
                // external data ...
                let data_node = self.parse_external_data_node()?;
                Ok(ExternalNode::Data(data_node))
            }
            Some(_) => {
                return Err(Error::MessageWithLocation(
                    "Expect external \"fn\" or \"data\".".to_owned(),
                    self.peek_range(0).unwrap().get_position_by_range_start(),
                ))
            }
            None => {
                return Err(Error::UnexpectedEndOfDocument(
                    "Expect external \"fn\" or \"data\".".to_owned(),
                ))
            }
        }
    }

    fn parse_external_function_node(&mut self) -> Result<ExternalFunctionNode, Error> {
        // fn name_path ()->() [as ...] ?  //
        // ^                            ^__// to here
        // |-------------------------------// current token, validated

        self.next_token(); // consume 'fn'
        self.consume_new_line_if_exist();

        let name_path = self.consume_name_path()?;

        // parse the parameters

        self.consume_left_paren()?; // consume '('
        self.consume_new_line_if_exist();

        let mut params: Vec<FunctionDataType> = vec![];
        while let Some(token) = self.peek_token(0) {
            if token == &Token::RightParen {
                break;
            }

            let data_type = self.continue_parse_function_data_type()?;
            params.push(data_type);

            let found_sep = self.consume_new_line_or_comma_if_exist();
            if !found_sep {
                break;
            }
        }

        self.consume_right_paren()?; // consume ')'

        // parse the return data type

        let return_: Option<FunctionDataType> = if self.expect_token(0, &Token::RightArrow) {
            self.next_token(); // consume '->'
            self.consume_new_line_if_exist();

            if self.expect_token(0, &Token::LeftParen) {
                self.next_token(); // consume '('
                self.consume_right_paren()?; // consume ')'
                None
            } else {
                let data_type = self.continue_parse_function_data_type()?;
                Some(data_type)
            }
        } else {
            None
        };

        // parse the 'as' part
        let alias_name = if self.expect_keyword(0, "as") {
            self.next_token(); // consume 'as'
            self.consume_new_line_if_exist();

            let name = self.consume_name()?;
            Some(name)
        } else {
            None
        };

        // consume trail new line
        self.consume_new_line_or_eof()?;

        let node = ExternalFunctionNode {
            name_path,
            params,
            return_,
            alias_name,
        };

        Ok(node)
    }

    fn continue_parse_function_data_type(&mut self) -> Result<FunctionDataType, Error> {
        // i32 ?  //
        // ^   ^__// to here
        // |------// current token, validated
        //
        // also:
        // i64, f32, f64

        let token = self.peek_token(0).unwrap();
        let data_type = match token {
            // the name of data type.
            // e.g. "i64", "i32", "byte"
            // it does not include the type details, such as
            // the length and alignment of byte, e.g. "byte[1024, align=8]".
            Token::DataTypeName(dt) => {
                match dt.as_str() {
                    "i64" => {
                        self.next_token(); // consume i64
                        FunctionDataType::I64
                    }
                    "i32" => {
                        self.next_token(); // consume i32
                        FunctionDataType::I32
                    }
                    "f64" => {
                        self.next_token(); // consume f64
                        FunctionDataType::F64
                    }
                    "f32" => {
                        self.next_token(); // consume f32
                        FunctionDataType::F32
                    }
                    _ => {
                        return Err(Error::MessageWithLocation(
                            "Unsupported data type for function parameters.".to_owned(),
                            self.peek_range(0).unwrap().get_position_by_range_start(),
                        ));
                    }
                }
            }
            _ => {
                return Err(Error::MessageWithLocation(
                    "Expect a data type".to_owned(),
                    self.peek_range(0).unwrap().get_position_by_range_start(),
                ));
            }
        };

        Ok(data_type)
    }

    fn parse_external_data_node(&mut self) -> Result<ExternalDataNode, Error> {
        // data name_path:data_type [as ...] ?  //
        // ^                                 ^__// to here
        // |------------------------------------// current token, validated
        self.next_token(); // consume 'data'
        self.consume_new_line_if_exist();

        let name_path = self.consume_name_path()?;

        self.consume_colon()?; // consume ':'
        self.consume_new_line_if_exist();

        let data_type = self.continue_parse_external_data_type()?;

        let alias_name = if self.expect_keyword(0, "as") {
            self.next_token(); // consume 'as'
            self.consume_new_line_if_exist();

            let name = self.consume_name()?;
            Some(name)
        } else {
            None
        };

        self.consume_new_line_or_eof()?;

        let node = ExternalDataNode {
            name_path,
            data_type,
            alias_name,
        };

        Ok(node)
    }

    fn continue_parse_external_data_type(&mut self) -> Result<ExternalDataType, Error> {
        // i32 ?  //
        // ^   ^__// to here
        // |------// current token, validated
        //
        // also:
        // - i64, f32, f64, byte[]

        let token = self.peek_token(0).unwrap();
        let data_type = match token {
            // the name of data type.
            // e.g. "i64", "i32", "byte"
            // it does not include the type details, such as
            // the length and alignment of byte, e.g. "byte[1024, align=8]".
            Token::DataTypeName(dt) => {
                match dt.as_str() {
                    "i64" => {
                        self.next_token(); // consume i64
                        ExternalDataType::I64
                    }
                    "i32" => {
                        self.next_token(); // consume i32
                        ExternalDataType::I32
                    }
                    "f64" => {
                        self.next_token(); // consume f64
                        ExternalDataType::F64
                    }
                    "f32" => {
                        self.next_token(); // consume f32
                        ExternalDataType::F32
                    }
                    "byte" => {
                        self.next_token(); // consume 'byte'
                        self.consume_left_bracket()?; // consule '['
                        self.consume_right_bracket()?;

                        ExternalDataType::Bytes
                    }
                    _ => {
                        return Err(Error::MessageWithLocation(
                            "Unsupported data type for external data.".to_owned(),
                            self.peek_range(0).unwrap().get_position_by_range_start(),
                        ));
                    }
                }
            }
            _ => {
                return Err(Error::MessageWithLocation(
                    "Expect a data type".to_owned(),
                    self.peek_range(0).unwrap().get_position_by_range_start(),
                ));
            }
        };

        Ok(data_type)
    }

    fn parse_data_node(
        &mut self,
        is_public: bool,
        data_section_type: DataSectionType,
    ) -> Result<DataNode, Error> {
        todo!()
    }

    fn continue_parse_declare_data_type(&mut self) -> Result<DeclareDataType, Error> {
        // i32 ?  //
        // ^   ^__// to here
        // |------// current token, validated
        //
        // also:
        // - i64, f32, f64
        // - byte[], byte[1024]
        // - byte[align=4], byte[1024, align=8]

        let token = self.peek_token(0).unwrap();
        let data_type = match token {
            // the name of data type.
            // e.g. "i64", "i32", "byte"
            // it does not include the type details, such as
            // the length and alignment of byte, e.g. "byte[1024, align=8]".
            Token::DataTypeName(dt) => {
                match dt.as_str() {
                    "i64" => {
                        self.next_token(); // consume i64
                        DeclareDataType::I64
                    }
                    "i32" => {
                        self.next_token(); // consume i32
                        DeclareDataType::I32
                    }
                    "f64" => {
                        self.next_token(); // consume f64
                        DeclareDataType::F64
                    }
                    "f32" => {
                        self.next_token(); // consume f32
                        DeclareDataType::F32
                    }
                    "byte" => {
                        self.next_token(); // consume 'byte'
                        self.consume_left_bracket()?; // consule '['
                        self.consume_new_line_if_exist();

                        if matches!(self.peek_token(0), Some(Token::Number(_))) {
                            // fixed size byte array

                            let length = self.consume_number_i32()? as usize; // consume i32

                            let found_sep = self.consume_new_line_or_comma_if_exist();
                            let align = if found_sep && self.expect_keyword(0, "align") {
                                self.next_token(); // consume 'align'
                                self.consume_equal()?; // consume '='
                                self.consume_new_line_if_exist();

                                let align = self.consume_number_i32()? as usize; // consume i32
                                self.consume_new_line_if_exist();

                                Some(align)
                            } else {
                                None
                            };

                            self.consume_right_bracket()?; //  consume ']'

                            DeclareDataType::FixedBytes(length, align)
                        } else if self.expect_keyword(0, "align") {
                            // variable size byte array
                            self.next_token(); // consume 'align'
                            self.consume_equal()?; // consume '='
                            self.consume_new_line_if_exist();

                            let align = self.consume_number_i32()? as usize; // consume i32
                            self.consume_new_line_if_exist();

                            self.consume_right_bracket()?; //  consume ']'

                            DeclareDataType::Bytes(Some(align))
                        } else {
                            // variable size byte array

                            self.consume_right_bracket()?; //  consume ']'

                            DeclareDataType::Bytes(None)
                        }
                    }
                    _ => {
                        return Err(Error::MessageWithLocation(
                            "Unsupported data type for data.".to_owned(),
                            self.peek_range(0).unwrap().get_position_by_range_start(),
                        ));
                    }
                }
            }
            _ => {
                return Err(Error::MessageWithLocation(
                    "Expect a data type".to_owned(),
                    self.peek_range(0).unwrap().get_position_by_range_start(),
                ));
            }
        };

        Ok(data_type)
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
        assert_eq!(format("use std::memory::copy"), "use std::memory::copy\n\n");

        // test 'as'
        assert_eq!(
            format("use parent::sub_sub_module::some_data as other_data"),
            "use parent::sub_sub_module::some_data as other_data\n\n"
        );

        // test line break
        assert_eq!(
            format(
                "use
std::memory::copy as
mem_copy"
            ),
            "use std::memory::copy as mem_copy\n\n"
        );

        // test multiple items
        assert_eq!(
            format(
                "use module::sub_module::some_func
use self::sub_module::some_func"
            ),
            "use module::sub_module::some_func
use self::sub_module::some_func\n\n"
        );
    }

    #[test]
    fn test_parse_external_fn_statement() {
        assert_eq!(
            format("external fn libfoo::bar()->()"),
            "external fn libfoo::bar() -> ()\n\n"
        );

        // test omit 'return'
        assert_eq!(
            format("external fn libfoo::bar()"),
            "external fn libfoo::bar() -> ()\n\n"
        );

        // test with params
        assert_eq!(
            format("external fn libfoo::add(i32,i32)->i32"),
            "external fn libfoo::add(i32, i32) -> i32\n\n"
        );

        // test 'as'
        assert_eq!(
            format("external fn libfoo::bar() as baz"),
            "external fn libfoo::bar() -> () as baz\n\n"
        );

        assert_eq!(
            format("external fn libfoo::add(i32,i32)->i32 as add_i32"),
            "external fn libfoo::add(i32, i32) -> i32 as add_i32\n\n"
        );

        // test line break
        assert_eq!(
            format(
                "external fn
libfoo::add(
i32
i32
)->
i32 as
add_i32"
            ),
            "external fn libfoo::add(i32, i32) -> i32 as add_i32\n\n"
        );

        // test multiple items
        assert_eq!(
            format(
                "external fn libfoo::bar()
external fn libfoo::add(i32,i32)->i32"
            ),
            "external fn libfoo::bar() -> ()
external fn libfoo::add(i32, i32) -> i32\n\n"
        );
    }

    #[test]
    fn test_parse_external_data_statement() {
        assert_eq!(
            format("external data libfoo::PI:f32"),
            "external data libfoo::PI:f32\n\n"
        );

        // test 'as'
        assert_eq!(
            format("external data libfoo::bar:byte[] as baz"),
            "external data libfoo::bar:byte[] as baz\n\n"
        );

        // test line break
        assert_eq!(
            format(
                "external data
libfoo::bar :
byte[] as
baz"
            ),
            "external data libfoo::bar:byte[] as baz\n\n"
        );

        // test multiple items
        assert_eq!(
            format(
                "external data libfoo::PI:f32
external data libfoo::bar:byte[] as baz"
            ),
            "external data libfoo::PI:f32
external data libfoo::bar:byte[] as baz\n\n"
        );
    }

    #[test]
    fn test_parse_data_statement() {
        // todo
    }
}
