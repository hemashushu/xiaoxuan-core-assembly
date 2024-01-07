// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::{peekable_iterator::PeekableIterator, ParseError};

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    LeftParen,
    RightParen,
    NewLine,
    Identifier(String),
    Number(NumberToken),
    String_(String),
    ByteData(Vec<u8>),
    Symbol(String),
    Shebang(String),
    Comment(CommentToken),
}

#[derive(Debug, PartialEq, Clone)]
pub enum NumberToken {
    Decimal(String),
    Hex(String),
    HexFloat(String),
    Binary(String),
}

#[derive(Debug, PartialEq, Clone)]
pub enum CommentToken {
    Line(String),
    Block(String),
    // Node(String),
    Document(String),
}

pub fn filter(tokens: &[Token]) -> Vec<Token> {
    tokens
        .iter()
        .filter(|token| {
            !matches!(
                token,
                Token::Shebang(_) | Token::NewLine | Token::Comment(_)
            )
        })
        .map(|token| token.to_owned())
        .collect::<Vec<Token>>()
}

pub fn lex(iter: &mut PeekableIterator<char>) -> Result<Vec<Token>, ParseError> {
    let mut tokens: Vec<Token> = vec![];

    // skip the shebang (https://en.wikipedia.org/wiki/Shebang_(Unix))
    if iter.look_ahead_equals(0, &'#') && iter.look_ahead_equals(1, &'!') {
        iter.next(); // consume '#'
        iter.next(); // consume '!'

        let mut ss = String::new();
        while let Some(previous_char) = iter.next() {
            if previous_char == '\n' {
                tokens.push(Token::Shebang(ss));
                break;
            } else if previous_char == '\r' && iter.look_ahead_equals(0, &'\n') {
                tokens.push(Token::Shebang(ss));
                iter.next(); // consume '\n'
                break;
            }

            ss.push(previous_char);
        }
    }

    while let Some(current_char) = iter.peek(0) {
        match current_char {
            ' ' | '\t' => {
                // white space
                iter.next();
            }
            '\r' => {
                // \r\n or \r
                if iter.look_ahead_equals(1, &'\n') {
                    iter.next();
                }

                iter.next();
                tokens.push(Token::NewLine);
            }
            '\n' => {
                // \n
                iter.next();
                tokens.push(Token::NewLine);
            }
            '$' => {
                // identifier
                tokens.push(lex_identifier(iter)?);
            }
            '0'..='9' | '+' | '-' => {
                // number
                tokens.push(lex_number(iter)?);
            }
            'h' if iter.look_ahead_equals(1, &'"') => {
                // bytes data
                tokens.push(lex_bytes_data(iter)?);
            }
            'r' if iter.look_ahead_equals(1, &'"') => {
                // raw string
                tokens.push(lex_raw_string(iter)?);
            }
            'r' if iter.look_ahead_equals(1, &'#') && iter.look_ahead_equals(2, &'"') => {
                // raw string variant
                tokens.push(lex_raw_string_variant(iter)?);
            }
            'r' if iter.look_ahead_equals(1, &'|') && iter.look_ahead_equals(2, &'"') => {
                // auto-trimmed string
                tokens.push(lex_auto_trimmed_string(iter)?);
            }
            '"' => {
                if iter.look_ahead_equals(1, &'"') && iter.look_ahead_equals(2, &'"') {
                    // document comment
                    tokens.push(lex_document_comment(iter)?);
                } else {
                    // string
                    tokens.push(lex_string(iter)?);
                }
            }
            // '|' if iter.look_ahead_equals(1, &'"') => {
            //     // auto-trimmed string
            //     tokens.push(lex_auto_trimmed_string(iter)?);
            // }
            '(' => {
                // if iter.look_ahead_equals(1, &';') {
                //     // block comment
                //     tokens.push(lex_block_comment(iter)?);
                // } else {
                // left parenthese
                tokens.push(Token::LeftParen);
                iter.next();
                // }
            }
            ')' => {
                // right parenthese
                tokens.push(Token::RightParen);
                iter.next();
            }
            '/' if iter.look_ahead_equals(1, &'/') => {
                // line comment
                tokens.push(lex_line_comment(iter)?);
            }
            '/' if iter.look_ahead_equals(1, &'*') => {
                // block comment
                tokens.push(lex_block_comment(iter)?);
            }
            // ';' if iter.look_ahead_equals(1, &';') => {
            //     // line comment
            //     tokens.push(lex_line_comment(iter)?);
            // }
            // '#' if iter.look_ahead_equals(1, &'(') => {
            //     // node comment
            //     comsume_node_comment(iter)?;
            // }
            'a'..='z' | 'A'..='Z' | '_' => {
                // symbol
                tokens.push(lex_symbol(iter)?);
            }
            _ => {
                return Err(ParseError::new(&format!(
                    "Unexpected char: {}",
                    current_char
                )))
            }
        }
    }

    Ok(tokens)
}

fn lex_identifier(iter: &mut PeekableIterator<char>) -> Result<Token, ParseError> {
    // $nameT  //
    // ^    ^__// to here
    // |_______// current char, i.e. the value of 'iter.peek(0)'
    //
    // T = terminator chars

    iter.next(); // consume char '$'

    // identifier should not starts with numbers [0-9]
    if matches!(iter.peek(0), Some(current_char) if *current_char >= '0' && *current_char <= '9') {
        return Err(ParseError::new(
            "Identifier should not start with a number.",
        ));
    }

    let mut id_string = String::new();

    while let Some(current_char) = iter.peek(0) {
        match *current_char {
            '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' => {
                id_string.push(*current_char);
                iter.next();
            }
            ':' if iter.look_ahead_equals(1, &':') => {
                id_string.push_str("::");
                iter.next();
                iter.next();
            }
            ' ' | '\t' | '\r' | '\n' | '(' | ')' | '/' | '"' => {
                // terminator chars
                break;
            }
            _ => {
                return Err(ParseError::new(&format!(
                    "Invalid char for identifier: {}",
                    *current_char
                )))
            }
        }
    }

    if id_string.is_empty() {
        Err(ParseError::new("Empty identifier."))
    } else {
        Ok(Token::Identifier(id_string))
    }
}

fn lex_number(iter: &mut PeekableIterator<char>) -> Result<Token, ParseError> {
    // 123456T  //
    // ^     ^__// to here
    // |________// current char
    //
    //
    // the number may also be hex or binary, e.g.
    // 0xaabb
    // 0b1100

    if let Some('+') = iter.peek(0) {
        // consume the plugs sign '+'
        iter.next();
    }

    let is_neg = if let Some('-') = iter.peek(0) {
        // consume the minus sign '-'
        iter.next();
        true
    } else {
        false
    };

    if iter.look_ahead_equals(0, &'0') && iter.look_ahead_equals(1, &'b') {
        // '0b...'
        lex_number_binary(iter, is_neg)
    } else if iter.look_ahead_equals(0, &'0') && iter.look_ahead_equals(1, &'x') {
        // '0x...'
        lex_number_hex(iter, is_neg)
    } else {
        lex_number_decimal(iter, is_neg)
    }
}

fn lex_number_decimal(
    iter: &mut PeekableIterator<char>,
    is_neg: bool,
) -> Result<Token, ParseError> {
    // 123456T  //
    // ^     ^__// to here
    // |________// current char
    //
    // T = terminator chars

    let mut num_string = String::new();

    if is_neg {
        num_string.push('-');
    }

    let mut found_point = false;
    let mut found_e = false;

    while let Some(current_char) = iter.peek(0) {
        match *current_char {
            '0'..='9' | '_' => {
                // valid digits for decimal number
                num_string.push(*current_char);
                iter.next();
            }
            '.' if !found_point => {
                found_point = true;
                num_string.push(*current_char);
                iter.next();
            }
            'e' if !found_e => {
                found_e = true;
                if iter.look_ahead_equals(1, &'-') {
                    num_string.push_str("e-");
                    iter.next();
                    iter.next();
                } else {
                    num_string.push(*current_char);
                    iter.next();
                }
            }
            ' ' | '\t' | '\r' | '\n' | '(' | ')' | '/' | '"' => {
                // terminator chars
                break;
            }
            _ => {
                return Err(ParseError::new(&format!(
                    "Invalid char for decimal number: {}",
                    *current_char
                )))
            }
        }
    }

    Ok(Token::Number(NumberToken::Decimal(num_string)))
}

fn lex_number_binary(iter: &mut PeekableIterator<char>, is_neg: bool) -> Result<Token, ParseError> {
    // 0b1010T  //
    // ^     ^__// to here
    // |________// current char
    //
    // T = terminator chars

    // consume '0b'
    iter.next();
    iter.next();

    let mut num_string = String::new();

    if is_neg {
        num_string.push('-');
    }

    while let Some(current_char) = iter.peek(0) {
        match *current_char {
            '0' | '1' | '_' => {
                // valid digits for binary number
                num_string.push(*current_char);
                iter.next();
            }
            ' ' | '\t' | '\r' | '\n' | '(' | ')' | '/' | '"' => {
                // terminator chars
                break;
            }
            _ => {
                return Err(ParseError::new(&format!(
                    "Invalid char for binary number: {}",
                    *current_char
                )))
            }
        }
    }

    if num_string.is_empty() {
        Err(ParseError::new("Incomplete binary number"))
    } else {
        Ok(Token::Number(NumberToken::Binary(num_string)))
    }
}

fn lex_number_hex(iter: &mut PeekableIterator<char>, is_neg: bool) -> Result<Token, ParseError> {
    // 0xaabbT  //
    // ^     ^__// to here
    // |________// current char
    //
    // T = terminator chars

    // consume '0x'
    iter.next();
    iter.next();

    let mut is_floating_point_number: bool = false;
    let mut num_string = String::new();

    while let Some(current_char) = iter.peek(0) {
        match *current_char {
            '0'..='9' | 'a'..='f' | 'A'..='F' | '_' => {
                // valid digits for hex number
                num_string.push(*current_char);
                iter.next();
            }
            '.' | 'p' => {
                // it is hex floating point literal
                is_floating_point_number = true;
                num_string.push(*current_char);
                iter.next();
            }
            '+' | '-' if is_floating_point_number => {
                num_string.push(*current_char);
                iter.next();
            }
            ' ' | '\t' | '\r' | '\n' | '(' | ')' | '/' | '"' => {
                // terminator chars
                break;
            }
            _ => {
                return Err(ParseError::new(&format!(
                    "Invalid char for hexadecimal number: {}",
                    *current_char
                )))
            }
        }
    }

    if num_string.is_empty() {
        Err(ParseError::new("Incomplete hex number"))
    } else {
        #[allow(clippy::collapsible_else_if)]
        let num = if is_floating_point_number {
            if is_neg {
                NumberToken::HexFloat(format!("-0x{}", num_string))
            } else {
                NumberToken::HexFloat(format!("0x{}", num_string))
            }
        } else {
            if is_neg {
                NumberToken::Hex(format!("-{}", num_string))
            } else {
                NumberToken::Hex(num_string)
            }
        };
        Ok(Token::Number(num))
    }
}

fn lex_string(iter: &mut PeekableIterator<char>) -> Result<Token, ParseError> {
    // "abc"?  //
    // ^    ^__// to here
    // |_______// current char

    iter.next(); // consume the quote

    let mut ss = String::new();

    loop {
        match iter.next() {
            Some(previous_char) => match previous_char {
                '\\' => {
                    // escape chars
                    match iter.next() {
                        Some(current_char) => {
                            match current_char {
                                '\\' => {
                                    ss.push('\\');
                                }
                                '"' => {
                                    ss.push('"');
                                }
                                't' => {
                                    // horizontal tabulation
                                    ss.push('\t');
                                }
                                'r' => {
                                    // carriage return (CR)
                                    ss.push('\r');
                                }
                                'n' => {
                                    // new line character (line feed, LF)
                                    ss.push('\n');
                                }
                                '0' => {
                                    // null char
                                    ss.push('\0');
                                }
                                'u' => {
                                    // unicode code point, e.g. '\u{2d}', '\u{6587}'
                                    ss.push(lex_string_unescape_unicode(iter)?);
                                }
                                '\n' => {
                                    // multiple-line string
                                    let _ = consume_leading_whitespaces(iter)?;
                                }
                                '\r' if iter.look_ahead_equals(0, &'\n') => {
                                    // multiple-line string
                                    iter.next();
                                    let _ = consume_leading_whitespaces(iter)?;
                                }
                                _ => {
                                    return Err(ParseError::new(&format!(
                                        "Unsupported escape char: \"{}\"",
                                        current_char
                                    )))
                                }
                            }
                        }
                        None => return Err(ParseError::new("Incomplete escape char.")),
                    }
                }
                '"' => {
                    // end of the string
                    break;
                }
                _ => {
                    // ordinary char
                    ss.push(previous_char);
                }
            },
            None => return Err(ParseError::new("Missing end quote for string.")),
        }
    }

    Ok(Token::String_(ss))
}

// return the amount of leading whitespaces
fn consume_leading_whitespaces(iter: &mut PeekableIterator<char>) -> Result<usize, ParseError> {
    // \nssssS  //
    //   ^   ^__// to here ('s' = whitespace, i.e. [ \t], 'S' = not whitespace)
    //   |______// current char

    let mut count = 0;
    loop {
        match iter.peek(0) {
            Some(next_char) if next_char == &' ' || next_char == &'\t' => {
                count += 1;
                iter.next();
            }
            None => return Err(ParseError::new("Expect the string content.")),
            _ => break,
        }
    }

    Ok(count)
}

fn skip_leading_whitespaces(iter: &mut PeekableIterator<char>, whitespaces: usize) {
    for _ in 0..whitespaces {
        match iter.peek(0) {
            Some(next_char) if next_char == &' ' || next_char == &'\t' => {
                iter.next();
            }
            _ => break,
        }
    }
}

fn lex_string_unescape_unicode(iter: &mut PeekableIterator<char>) -> Result<char, ParseError> {
    // \u{6587}?  //
    //   ^     ^__// to here
    //   |________// current char

    // comsume char '{'
    if !matches!(iter.next(), Some(c) if c == '{') {
        return Err(ParseError::new(
            "Missing left brace for unicode escape sequence.",
        ));
    }

    let mut codepoint_string = String::new();

    loop {
        match iter.next() {
            Some(previous_char) => match previous_char {
                '}' => break,
                '0'..='9' | 'a'..='f' | 'A'..='F' => codepoint_string.push(previous_char),
                _ => {
                    return Err(ParseError::new(&format!(
                        "Invalid character for unicode escape sequence: {}",
                        previous_char
                    )))
                }
            },
            None => {
                return Err(ParseError::new(
                    "Missing right brace for unicode escape sequence.",
                ))
            }
        }

        if codepoint_string.len() > 5 {
            return Err(ParseError::new(
                "The value of unicode point code is to large.",
            ));
        }
    }

    let codepoint = u32::from_str_radix(&codepoint_string, 16).unwrap();

    if let Some(unic) = char::from_u32(codepoint) {
        // valid code point:
        // 0 to 0x10FFFF, inclusive
        //
        // ref:
        // https://doc.rust-lang.org/std/primitive.char.html
        Ok(unic)
    } else {
        Err(ParseError::new("Invalid unicode code point."))
    }
}

fn lex_raw_string(iter: &mut PeekableIterator<char>) -> Result<Token, ParseError> {
    // r"abc"?  //
    // ^     ^__// to here
    // |________// current char

    iter.next(); // consume char 'r'
    iter.next(); // consume the quote

    let mut ss = String::new();

    loop {
        match iter.next() {
            Some(previous_char) => match previous_char {
                '"' => {
                    // end of the string
                    break;
                }
                _ => {
                    // ordinary char
                    ss.push(previous_char);
                }
            },
            None => return Err(ParseError::new("Missing end quote for string.")),
        }
    }

    Ok(Token::String_(ss))
}

fn lex_raw_string_variant(iter: &mut PeekableIterator<char>) -> Result<Token, ParseError> {
    // r#"abc"#?  //
    // ^       ^__// to here
    // |__________// current char

    iter.next(); // consume char 'r'
    iter.next(); // consume the hash
    iter.next(); // consume the quote

    let mut ss = String::new();

    loop {
        match iter.next() {
            Some(previous_char) => match previous_char {
                '"' if iter.look_ahead_equals(0, &'#') => {
                    // end of the string
                    iter.next(); // consume the hash
                    break;
                }
                _ => {
                    // ordinary char
                    ss.push(previous_char);
                }
            },
            None => return Err(ParseError::new("Missing end quote for string.")),
        }
    }

    Ok(Token::String_(ss))
}

fn lex_auto_trimmed_string(iter: &mut PeekableIterator<char>) -> Result<Token, ParseError> {
    // r|"                    //
    // ^  auto-trimmed string //
    // |  "|\n?               //
    // |      ^_______________// to here ('?' = any chars or EOF)
    // |______________________// current char

    iter.next(); // consume char r
    iter.next(); // consume char |
    iter.next(); // consume char "

    if iter.look_ahead_equals(0, &'\n') {
        iter.next();
    } else if iter.look_ahead_equals(0, &'\r') && iter.look_ahead_equals(1, &'\n') {
        iter.next();
        iter.next();
    } else {
        return Err(ParseError::new(
            "The content of auto-trimmed string should start on a new line.",
        ));
    }

    let leading_whitespaces = consume_leading_whitespaces(iter)?;
    let mut ss = String::new();
    let mut line_leading = String::new();

    loop {
        match iter.next() {
            Some(previous_char) => {
                match previous_char {
                    '\n' => {
                        ss.push('\n');
                        line_leading.clear();
                        skip_leading_whitespaces(iter, leading_whitespaces);
                    }
                    '\r' if iter.look_ahead_equals(0, &'\n') => {
                        iter.next(); // consume '\n'

                        ss.push_str("\r\n");
                        line_leading.clear();
                        skip_leading_whitespaces(iter, leading_whitespaces);
                    }
                    '"' if line_leading.trim().is_empty() && iter.look_ahead_equals(0, &'|') => {
                        iter.next(); // consume '|'
                        break;

                        //                         // only ("|) which occupies a single line, is considered to be
                        //                         // the ending mark.
                        //                         // note that the ending marker includes the new line symbol (\n or \r\n),
                        //                         // i.e., the ("|\n) or ("|\r\n), so there is NO `Token::NewLine` follows
                        //                         // the ending marker.
                        //
                        //                         if iter.look_ahead_equals(0, &'\n') {
                        //                             iter.next();
                        //                             break;
                        //                         } else if iter.look_ahead_equals(0, &'\r')
                        //                             && iter.look_ahead_equals(1, &'\n')
                        //                         {
                        //                             iter.next();
                        //                             iter.next();
                        //                             break;
                        //                         } else {
                        //                             // it's not a valid ending mark.
                        //                             ss.push_str("\"|");
                        //                         }
                    }
                    _ => {
                        ss.push(previous_char);
                        line_leading.push(previous_char);
                    }
                }
            }
            None => {
                return Err(ParseError::new(
                    "Missing the ending marker for the auto-trimmed string.",
                ))
            }
        }
    }

    Ok(Token::String_(ss.trim_end().to_owned()))
}

fn lex_document_comment(iter: &mut PeekableIterator<char>) -> Result<Token, ParseError> {
    // """                  //
    // ^  document comment  //
    // |  """\n?            //
    // |       ^____________// to here ('?' = any chars or EOF)
    // |____________________// current char

    // consume 3 chars (""")
    iter.next();
    iter.next();
    iter.next();

    if iter.look_ahead_equals(0, &'\n') {
        iter.next();
    } else if iter.look_ahead_equals(0, &'\r') && iter.look_ahead_equals(1, &'\n') {
        iter.next();
        iter.next();
    } else {
        return Err(ParseError::new(
            "The content of document comment should start on a new line.",
        ));
    }

    let leading_whitespaces = consume_leading_whitespaces(iter)?;
    let mut ss = String::new();
    let mut line_leading = String::new();

    loop {
        match iter.next() {
            Some(previous_char) => {
                match previous_char {
                    '\n' => {
                        ss.push('\n');
                        line_leading.clear();
                        skip_leading_whitespaces(iter, leading_whitespaces);
                    }
                    '\r' if iter.look_ahead_equals(0, &'\n') => {
                        iter.next(); // consume '\n'

                        ss.push_str("\r\n");
                        line_leading.clear();
                        skip_leading_whitespaces(iter, leading_whitespaces);
                    }
                    '"' if line_leading.trim().is_empty()
                        && iter.look_ahead_equals(0, &'"')
                        && iter.look_ahead_equals(1, &'"') =>
                    {
                        iter.next(); // consume '"'
                        iter.next(); // consume '"'

                        // only (""") which occupies a single line, is considered to be
                        // the ending mark of a paragraph string.
                        // note that the ending marker includes the new line symbol (\n or \r\n),
                        // i.e., the ("""\n) or ("""\r\n), so there is NO `Token::NewLine` follows
                        // the ending marker.
                        if iter.look_ahead_equals(0, &'\n') {
                            iter.next();
                            break;
                        } else if iter.look_ahead_equals(0, &'\r')
                            && iter.look_ahead_equals(1, &'\n')
                        {
                            iter.next();
                            iter.next();
                            break;
                        } else {
                            // it's not a valid ending mark.
                            ss.push_str("\"\"\"");
                        }
                    }
                    _ => {
                        ss.push(previous_char);
                        line_leading.push(previous_char);
                    }
                }
            }
            None => {
                return Err(ParseError::new(
                    "Missing the ending marker for the paragraph string.",
                ))
            }
        }
    }

    Ok(Token::Comment(CommentToken::Document(
        ss.trim_end().to_owned(),
    )))
}

fn lex_bytes_data(iter: &mut PeekableIterator<char>) -> Result<Token, ParseError> {
    // h"0011aabb"?  //
    // ^          ^__// to here
    // |_____________// current char

    let mut bytes: Vec<u8> = Vec::new();
    let mut byte_buf = String::with_capacity(2);

    iter.next(); // consume char 'h'
    iter.next(); // consume quote '"'

    loop {
        match iter.next() {
            Some(previous_char) => {
                match previous_char {
                    ' ' | '\t' | '\r' | '\n' | '-' | ':' => {
                        // ignore the separator and whitespace chars
                    }
                    '"' => {
                        if !byte_buf.is_empty() {
                            return Err(ParseError::new("Incomplete byte string."));
                        } else {
                            break;
                        }
                    }
                    'a'..='f' | 'A'..='F' | '0'..='9' => {
                        byte_buf.push(previous_char);

                        if byte_buf.len() == 2 {
                            let byte = u8::from_str_radix(&byte_buf, 16).unwrap();
                            bytes.push(byte);
                            byte_buf.clear();
                        }
                    }
                    _ => {
                        return Err(ParseError::new(&format!(
                            "Invalid char for byte string: {}",
                            previous_char
                        )))
                    }
                }
            }
            None => return Err(ParseError::new("Missing end quote for byte string.")),
        }
    }

    Ok(Token::ByteData(bytes))
}

fn lex_line_comment(iter: &mut PeekableIterator<char>) -> Result<Token, ParseError> {
    // xx...[\r]\n?  //
    // ^          ^__// to here ('?' = any char or EOF)
    // |_____________// current char
    //
    // x = '/'

    iter.next(); // consume char '/'
    iter.next(); // consume char '/'

    let mut ss = String::new();

    while let Some(previous_char) = iter.next() {
        // ignore all chars except '\n' or '\r\n'
        // note that the line comment includes the ending new line symbol (\n or \r\n),
        // so there is NO `Token::NewLine` follows the line comment.

        if previous_char == '\n' {
            break;
        } else if previous_char == '\r' && iter.look_ahead_equals(0, &'\n') {
            iter.next(); // consume char '\n'
            break;
        }

        ss.push(previous_char);
    }

    Ok(Token::Comment(CommentToken::Line(ss)))
}

fn lex_block_comment(iter: &mut PeekableIterator<char>) -> Result<Token, ParseError> {
    // x*...*x?  //
    // ^      ^__// to here
    // |_________// current char
    //
    // x = '/'

    iter.next(); // consume char '/'
    iter.next(); // consume char '*'

    let mut ss = String::new();
    let mut pairs = 1;

    loop {
        match iter.next() {
            Some(previous_char) => match previous_char {
                // '(' if iter.look_ahead_equals(0, &';') => {
                '/' if iter.look_ahead_equals(0, &'*') => {
                    // nested block comment
                    // ss.push_str("(;");
                    ss.push_str("/*");
                    iter.next();
                    pairs += 1;
                }
                // ';' if iter.look_ahead_equals(0, &')') => {
                '*' if iter.look_ahead_equals(0, &'/') => {
                    iter.next();
                    pairs -= 1;

                    // check pairs
                    if pairs == 0 {
                        break;
                    } else {
                        // ss.push_str(";)");
                        ss.push_str("*/");
                    }
                }
                _ => {
                    // ignore all chars except "(;" and ";)"
                    // note that line comments within block comments are ignored.
                    ss.push(previous_char);
                }
            },
            None => return Err(ParseError::new("Incomplete block comment.")),
        }
    }

    Ok(Token::Comment(CommentToken::Block(ss)))
}

// fn comsume_node_comment(iter: &mut PeekableIterator<char>) -> Result<(), ParseError> {
//     // #(comment ...)?  //
//     // ^             ^__// to here
//     // |________________// current char
//
//     iter.next(); // consume char '#'
//     iter.next(); // consume char '('
//
//     let mut pairs = 1;
//
//     loop {
//         match iter.next() {
//             Some(current_char) => match current_char {
//                 '(' => {
//                     if iter.look_ahead_equals(0, &';') {
//                         // nested block comment "(;..."
//                         lex_block_comment(iter)?;
//                     } else {
//                         // nested node comment
//                         pairs += 1;
//                     }
//                 }
//                 ')' => {
//                     pairs -= 1;
//
//                     if pairs == 0 {
//                         break;
//                     }
//                 }
//                 ';' => {
//                     if iter.look_ahead_equals(0, &';') {
//                         // nested line comment "//..."
//                         comsume_line_comment(iter)?;
//                     } else if iter.look_ahead_equals(0, &')') {
//                         return Err(ParseError::new("Unpaired block comment."));
//                     } else {
//                         return Err(ParseError::new("Unexpected char: \";\""));
//                     }
//                 }
//                 _ => {
//                     // ignore all chars except paired ')'
//                 }
//             },
//             None => return Err(ParseError::new("Incomplete node comment.")),
//         }
//     }
//
//     Ok(())
// }

fn lex_symbol(iter: &mut PeekableIterator<char>) -> Result<Token, ParseError> {
    // localT  //
    // ^    ^__// to here
    // |_______// current char
    //
    // T = terminator chars

    let mut sym_string = String::new();

    while let Some(current_char) = iter.peek(0) {
        match *current_char {
            '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' | '.' => {
                sym_string.push(*current_char);
                iter.next();
            }
            ' ' | '\t' | '\r' | '\n' | '(' | ')' | '/' | '"' => {
                // terminator chars
                break;
            }
            _ => {
                return Err(ParseError::new(&format!(
                    "Invalid char for symbol: {}",
                    *current_char
                )))
            }
        }
    }

    Ok(Token::Symbol(sym_string))
}

impl Token {
    pub fn new_identifier(s: &str) -> Self {
        Token::Identifier(s.to_owned())
    }

    pub fn new_dec_number(s: &str) -> Self {
        Token::Number(NumberToken::Decimal(s.to_owned()))
    }

    pub fn new_hex_number(s: &str) -> Self {
        Token::Number(NumberToken::Hex(s.to_owned()))
    }

    pub fn new_hex_float_number(s: &str) -> Self {
        Token::Number(NumberToken::HexFloat(s.to_owned()))
    }

    pub fn new_bin_number(s: &str) -> Self {
        Token::Number(NumberToken::Binary(s.to_owned()))
    }

    pub fn new_string(s: &str) -> Self {
        Token::String_(s.to_owned())
    }

    pub fn new_bytes(slice: &[u8]) -> Self {
        Token::ByteData(slice.to_vec())
    }

    pub fn new_symbol(s: &str) -> Self {
        Token::Symbol(s.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        lexer::{filter, CommentToken, NumberToken, Token},
        peekable_iterator::PeekableIterator,
        ParseError,
    };

    use super::lex;

    fn lex_from_str(s: &str) -> Result<Vec<Token>, ParseError> {
        let mut chars = s.chars();
        let mut iter = PeekableIterator::new(&mut chars, 3);
        lex(&mut iter)
    }

    #[test]
    fn test_lex_white_spaces() {
        assert_eq!(lex_from_str("  ").unwrap(), vec![]);

        assert_eq!(
            lex_from_str("()").unwrap(),
            vec![Token::LeftParen, Token::RightParen]
        );

        assert_eq!(
            lex_from_str("(  )").unwrap(),
            vec![Token::LeftParen, Token::RightParen]
        );

        assert_eq!(
            lex_from_str("(\t\r\n\n\r)").unwrap(),
            vec![
                Token::LeftParen,
                Token::NewLine,
                Token::NewLine,
                Token::NewLine,
                Token::RightParen
            ]
        );
    }

    #[test]
    fn test_lex_identifier() {
        assert_eq!(
            lex_from_str("$name").unwrap(),
            vec![Token::new_identifier("name")]
        );

        assert_eq!(
            lex_from_str("($name)").unwrap(),
            vec![
                Token::LeftParen,
                Token::new_identifier("name"),
                Token::RightParen
            ]
        );

        assert_eq!(
            lex_from_str("( $a )").unwrap(),
            vec![
                Token::LeftParen,
                Token::new_identifier("a"),
                Token::RightParen
            ]
        );

        assert_eq!(
            lex_from_str("$a__b__c").unwrap(),
            vec![Token::new_identifier("a__b__c"),]
        );

        assert_eq!(
            lex_from_str("$a::b").unwrap(),
            vec![Token::new_identifier("a::b"),]
        );

        assert_eq!(
            lex_from_str("$a::b::c").unwrap(),
            vec![Token::new_identifier("a::b::c"),]
        );

        assert_eq!(
            lex_from_str("$foo $bar").unwrap(),
            vec![Token::new_identifier("foo"), Token::new_identifier("bar"),]
        );

        // err: incomplete identifier
        assert!(matches!(lex_from_str("$"), Err(ParseError { message: _ })));

        // err: invalid identifier
        assert!(matches!(
            lex_from_str("$1abc"),
            Err(ParseError { message: _ })
        ));

        // err: invalid char for identifier
        assert!(matches!(
            lex_from_str("$abc+xyz"),
            Err(ParseError { message: _ })
        ));

        // err: single colon
        assert!(matches!(
            lex_from_str("$ab:c"),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_lex_number() {
        assert_eq!(
            lex_from_str("(211)").unwrap(),
            vec![
                Token::LeftParen,
                Token::new_dec_number("211"),
                Token::RightParen
            ]
        );

        assert_eq!(
            lex_from_str("211").unwrap(),
            vec![Token::new_dec_number("211")]
        );

        assert_eq!(
            lex_from_str("3.14").unwrap(),
            vec![Token::new_dec_number("3.14")]
        );

        assert_eq!(
            lex_from_str("2.998e8").unwrap(),
            vec![Token::new_dec_number("2.998e8")]
        );

        assert_eq!(
            lex_from_str("6.626e-34").unwrap(),
            vec![Token::new_dec_number("6.626e-34")]
        );

        // 3.1415927f32
        assert_eq!(
            lex_from_str("0x1.921fb6p1").unwrap(),
            vec![Token::new_hex_float_number("0x1.921fb6p1")]
        );

        // 2.718281828459045f64
        assert_eq!(
            lex_from_str("0x1.5bf0a8b145769p+1").unwrap(),
            vec![Token::new_hex_float_number("0x1.5bf0a8b145769p+1")]
        );

        assert_eq!(
            lex_from_str("+2017").unwrap(),
            vec![Token::new_dec_number("2017")]
        );

        assert_eq!(
            lex_from_str("-2027").unwrap(),
            vec![Token::new_dec_number("-2027")]
        );

        assert_eq!(
            lex_from_str("223_211").unwrap(),
            vec![Token::new_dec_number("223_211")]
        );

        assert_eq!(
            lex_from_str("223 211").unwrap(),
            vec![Token::new_dec_number("223"), Token::new_dec_number("211")]
        );

        assert_eq!(
            lex_from_str("0x1234abcd").unwrap(),
            vec![Token::new_hex_number("1234abcd")]
        );

        assert_eq!(
            lex_from_str("0b00110101").unwrap(),
            vec![Token::new_bin_number("00110101")]
        );

        assert_eq!(
            lex_from_str("11 0x11 0b11").unwrap(),
            vec![
                Token::new_dec_number("11"),
                Token::new_hex_number("11"),
                Token::new_bin_number("11")
            ]
        );

        assert_eq!(
            lex_from_str("-0xaabb").unwrap(),
            vec![Token::new_hex_number("-aabb")]
        );

        assert_eq!(
            lex_from_str("-0b1010").unwrap(),
            vec![Token::new_bin_number("-1010")]
        );

        assert_eq!(
            lex_from_str("-0x1.921fb6p1").unwrap(),
            vec![Token::new_hex_float_number("-0x1.921fb6p1")]
        );

        // err: invalid char for decimal number
        assert!(matches!(
            lex_from_str("123mn"),
            Err(ParseError { message: _ })
        ));

        // err: invalid symbol for decimal number
        assert!(matches!(
            lex_from_str("123-456"),
            Err(ParseError { message: _ })
        ));

        // err: multiple points
        assert!(matches!(
            lex_from_str("1.23.456"),
            Err(ParseError { message: _ })
        ));

        // err: multiple 'e' (exps)
        assert!(matches!(
            lex_from_str("1e2e3"),
            Err(ParseError { message: _ })
        ));

        // err: incomplete hex number
        assert!(matches!(lex_from_str("0x"), Err(ParseError { message: _ })));

        // err: invalid char for hex number
        assert!(matches!(
            lex_from_str("0x123xyz"),
            Err(ParseError { message: _ })
        ));

        // err: incomplete binary number
        assert!(matches!(lex_from_str("0b"), Err(ParseError { message: _ })));

        // err: invalid char for binary number
        assert!(matches!(
            lex_from_str("0b1234"),
            Err(ParseError { message: _ })
        ));

        // err: unsupported binary number expression
        assert!(matches!(
            lex_from_str("0b00_11.0101"),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_lex_string() {
        assert_eq!(lex_from_str("\"\"").unwrap(), vec![Token::new_string("")]);

        assert_eq!(
            lex_from_str("\"abc\"").unwrap(),
            vec![Token::new_string("abc")]
        );

        assert_eq!(
            lex_from_str("(\"abc\")").unwrap(),
            vec![
                Token::LeftParen,
                Token::new_string("abc"),
                Token::RightParen
            ]
        );

        assert_eq!(
            lex_from_str("\"abc\" \"xyz\"").unwrap(),
            vec![Token::new_string("abc"), Token::new_string("xyz"),]
        );

        assert_eq!(
            lex_from_str("\"abc\"\n\n\"xyz\"").unwrap(),
            vec![
                Token::new_string("abc"),
                Token::NewLine,
                Token::NewLine,
                Token::new_string("xyz"),
            ]
        );

        // unicode
        assert_eq!(
            lex_from_str(
                r#"
            "abc文字😊"
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("abc文字😊"),
                Token::NewLine,
            ]
        );

        // escape chars
        assert_eq!(
            lex_from_str(
                r#"
            "\r\n\t\\\"\u{2d}\u{6587}\0"
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("\r\n\t\\\"-文\0"),
                Token::NewLine
            ]
        );

        // err: unsupported escape char \v
        assert!(matches!(
            lex_from_str(
                r#"
            "abc\vxyz"
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // err: unsupported hex escape "\x.."
        assert!(matches!(
            lex_from_str(
                r#"
            "abc\x33xyz"
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // err: incomplete escape string
        assert!(matches!(
            lex_from_str(r#""abc\"#),
            Err(ParseError { message: _ })
        ));

        // err: invalid unicode code point
        assert!(matches!(
            lex_from_str(
                r#"
            "abc\u{110000}xyz"
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // err: invalid unicode escape sequence
        assert!(matches!(
            lex_from_str(
                r#"
            "abc\u{12mn}xyz"
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // err: missing left brace for unicode escape sequence
        assert!(matches!(
            lex_from_str(
                r#"
            "abc\u1234}xyz"
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // err: missing right brace for unicode escape sequence
        assert!(matches!(
            lex_from_str(r#""abc\u{1234"#),
            Err(ParseError { message: _ })
        ));

        // err: missing right quote
        assert!(matches!(
            lex_from_str(
                r#"
            "abc
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_lex_multiple_line_string() {
        assert_eq!(
            lex_from_str("\"abc\ndef\n    uvw\r\n\t  \txyz\"").unwrap(),
            vec![Token::new_string("abc\ndef\n    uvw\r\n\t  \txyz")]
        );

        assert_eq!(
            lex_from_str("\"abc\\\ndef\\\n    uvw\\\r\n\t  \txyz\"").unwrap(),
            vec![Token::new_string("abcdefuvwxyz")]
        );

        assert_eq!(
            lex_from_str("\"\\\n  \t  \"").unwrap(),
            vec![Token::new_string("")]
        );

        // err: missing right quote
        assert!(matches!(
            lex_from_str("\"abc\\\n    "),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_lex_law_string() {
        assert_eq!(
            lex_from_str(
                "r\"abc\ndef\n    uvw\r\n\t escape: \\r\\n\\t\\\\ unicode: \\u{1234} xyz\""
            )
            .unwrap(),
            vec![Token::new_string(
                "abc\ndef\n    uvw\r\n\t escape: \\r\\n\\t\\\\ unicode: \\u{1234} xyz"
            )]
        );

        // err: missing right quote
        assert!(matches!(
            lex_from_str("r\"abc    "),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_lex_law_string_varaint() {
        assert_eq!(
            lex_from_str(
                "r#\"abc\ndef\n    uvw\r\n\t escape: \\r\\n\\t\\\\ unicode: \\u{1234} xyz quote: \"foo\"\"#"
            )
            .unwrap(),
            vec![Token::new_string(
                "abc\ndef\n    uvw\r\n\t escape: \\r\\n\\t\\\\ unicode: \\u{1234} xyz quote: \"foo\""
            )]
        );

        // err: missing the ending marker
        assert!(matches!(
            lex_from_str("r#\"abc    "),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_lex_auto_trimmed_string() {
        assert_eq!(
            lex_from_str(
                r#"
            r|"
            one
              two
                three
            end
            "|
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("one\n  two\n    three\nend"),
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
            r|"
            one
          two
        three
            end
            "|
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("one\ntwo\nthree\nend"),
                Token::NewLine
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
            r|"
                one\\\"\t\r\n\u{1234}

                end
            "|
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("one\\\\\\\"\\t\\r\\n\\u{1234}\n\nend"),
                Token::NewLine,
            ]
        );

        // test the ending symbol ("|) does not start in a new line

        assert_eq!(
            lex_from_str(
                r#"
            r|"
                one"|
                two
            "|
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_string("one\"|\ntwo"),
                Token::NewLine,
            ]
        );

        // test inline
        assert_eq!(
            lex_from_str(
                r#"
            11 r|"
                abc
            "| 13
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Number(NumberToken::Decimal("11".to_owned())),
                Token::new_string("abc"),
                Token::Number(NumberToken::Decimal("13".to_owned())),
                Token::NewLine,
            ]
        );

        // err: the content does not start on a new line
        assert!(matches!(
            lex_from_str(
                r#"
            r|"hello"|
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // err: the ending marker does not start on a new line
        assert!(matches!(
            lex_from_str(
                r#"
        r|"
        hello"|
        "#
            ),
            Err(ParseError { message: _ })
        ));

        // // err: the ending marker does not occupy the whole line
        // assert!(matches!(
        //     lex_from_str(
        //         r#"
        //     |"
        //     hello
        //     "|world
        //     "#
        //     ),
        //     Err(ParseError { message: _ })
        // ));

        // err: missing the ending marker
        assert!(matches!(
            lex_from_str(
                r#"
            r|"
            hello
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_lex_byte_data() {
        assert_eq!(
            lex_from_str(
                r#"
            h""
            "#
            )
            .unwrap(),
            vec![Token::NewLine, Token::ByteData(vec![]), Token::NewLine,]
        );

        assert_eq!(
            lex_from_str(
                r#"
            h"11131719"
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::ByteData(vec![0x11, 0x13, 0x17, 0x19]),
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
            h"11 13 1719"
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::ByteData(vec![0x11, 0x13, 0x17, 0x19]),
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
            h"11-13-1719"
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::ByteData(vec![0x11, 0x13, 0x17, 0x19]),
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
            h"11:13:1719"
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::ByteData(vec![0x11, 0x13, 0x17, 0x19]),
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                "
            h\"1113\n17\t19\"
            "
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::ByteData(vec![0x11, 0x13, 0x17, 0x19]),
                Token::NewLine,
            ]
        );

        // err: incomplete byte string, the amount of digits should be even
        assert!(matches!(
            lex_from_str(
                r#"
            h"1113171"
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // err: invalid char for byte string
        assert!(matches!(
            lex_from_str(
                r#"
            h"1113171z"
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // err: missing the ending quote
        assert!(matches!(
            lex_from_str(
                r#"
            h"11131719
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_lex_line_comment() {
        assert_eq!(
            lex_from_str(
                r#"
            7 //11
            13 17// 19 23
            // 29
            31// 37
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_dec_number("7"),
                Token::Comment(CommentToken::Line("11".to_owned())),
                Token::new_dec_number("13"),
                Token::new_dec_number("17"),
                Token::Comment(CommentToken::Line(" 19 23".to_owned())),
                Token::Comment(CommentToken::Line(" 29".to_owned())),
                Token::new_dec_number("31"),
                Token::Comment(CommentToken::Line(" 37".to_owned())),
                // note that the line comment includes the ending new line symbol (\n or \r\n),
                // so there is NO `Token::NewLine` follows the line comment.
            ]
        );
    }

    #[test]
    fn test_lex_block_comment() {
        assert_eq!(
            lex_from_str(
                r#"
            7 /* 11 13 */ 17
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_dec_number("7"),
                Token::Comment(CommentToken::Block(" 11 13 ".to_owned())),
                Token::new_dec_number("17"),
                Token::NewLine,
            ]
        );

        // nested block comment
        assert_eq!(
            lex_from_str(
                r#"
            7 /* 11 /* 13 */ 17 */ 19
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_dec_number("7"),
                Token::Comment(CommentToken::Block(" 11 /* 13 */ 17 ".to_owned())),
                Token::new_dec_number("19"),
                Token::NewLine,
            ]
        );

        // line comment symbol "//" within the block comment
        assert_eq!(
            lex_from_str(
                r#"
            7 /* 11 // 13 17 */ 19
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_dec_number("7"),
                Token::Comment(CommentToken::Block(" 11 // 13 17 ".to_owned())),
                Token::new_dec_number("19"),
                Token::NewLine,
            ]
        );

        // document comment symbol (""") within the block comment
        assert_eq!(
            lex_from_str(
                r#"
            7 /* 11
            """
            abc
            """
            13 */ 19
            "#
                .lines()
                .map(&str::trim_start)
                .map(&str::to_owned)
                .collect::<Vec<String>>()
                .join("\n")
                .as_str()
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::new_dec_number("7"),
                Token::Comment(CommentToken::Block(
                    " 11\n\"\"\"\nabc\n\"\"\"\n13 ".to_owned()
                )),
                Token::new_dec_number("19"),
                Token::NewLine,
            ]
        );

        // assert_eq!(
        //     lex_from_str(
        //         r#"
        //     7 (; 11 #(13 17) ;) 19
        //     "#
        //     )
        //     .unwrap(),
        //     vec![Token::new_dec_number("7"), Token::new_dec_number("19"),]
        // );

        // err: unpaired, missing the ending pair
        assert!(matches!(
            lex_from_str(
                r#"
            7 /* 11 /* 13 */ 17
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // err: unpaired
        assert!(matches!(
            lex_from_str(
                r#"
            7 */ 11
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    //     #[test]
    //     fn test_lex_node_comment() {
    //         assert_eq!(
    //             lex_from_str(
    //                 r#"
    //             7 #(add 11 (mul 13 17)) 29
    //             "#
    //             )
    //             .unwrap(),
    //             vec![Token::new_dec_number("7"), Token::new_dec_number("29"),]
    //         );
    //
    //         assert_eq!(
    //             lex_from_str(
    //                 r#"
    //             7 #(add 11 #(mul 13 17) (mul 19 23)) 29
    //             "#
    //             )
    //             .unwrap(),
    //             vec![Token::new_dec_number("7"), Token::new_dec_number("29"),]
    //         );
    //
    //         assert_eq!(
    //             lex_from_str(
    //                 r#"
    //             7 #(add (; 11 ;)) 29
    //             "#
    //             )
    //             .unwrap(),
    //             vec![Token::new_dec_number("7"), Token::new_dec_number("29"),]
    //         );
    //
    //         assert_eq!(
    //             lex_from_str(
    //                 r#"
    //             7 #(add // 11 13)
    //             ) 29
    //             "#
    //             )
    //             .unwrap(),
    //             vec![Token::new_dec_number("7"), Token::new_dec_number("29"),]
    //         );
    //
    //         assert_eq!(
    //             lex_from_str(
    //                 r#"
    //             7 #(add (; 11 // 13 ;)) 29
    //             "#
    //             )
    //             .unwrap(),
    //             vec![Token::new_dec_number("7"), Token::new_dec_number("29"),]
    //         );
    //
    //         // missing end pair
    //         assert!(matches!(
    //             lex_from_str(
    //                 r#"
    //             7 #( 11
    //             "#
    //             ),
    //             Err(ParseError { message: _ })
    //         ));
    //
    //         // missing end pair
    //         assert!(matches!(
    //             lex_from_str(
    //                 r#"
    //             7 #) 11
    //             "#
    //             ),
    //             Err(ParseError { message: _ })
    //         ));
    //
    //         // missing end pair
    //         assert!(matches!(
    //             lex_from_str(
    //                 r#"
    //             7 #( 11 ()
    //             "#
    //             ),
    //             Err(ParseError { message: _ })
    //         ));
    //     }

    #[test]
    fn test_lex_document_comment() {
        assert_eq!(
            lex_from_str(
                r#"
            """
            one
              two
                three
            end
            """
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Comment(CommentToken::Document(
                    "one\n  two\n    three\nend".to_owned()
                )),
                // note that the ending marker includes the new line symbol (\n or \r\n),
                // i.e., the ("""\n) or ("""\r\n), so there is NO `Token::NewLine` follows
                // the ending marker.
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
            """
            one
          two
        three
            end
            """
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Comment(CommentToken::Document("one\ntwo\nthree\nend".to_owned()))
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
            """
                one\\\"\t\r\n\u{1234}

                end
            """
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Comment(CommentToken::Document(
                    "one\\\\\\\"\\t\\r\\n\\u{1234}\n\nend".to_owned()
                ))
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
            """
                one"""
                """two
                """"
                end
            """
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::Comment(CommentToken::Document(
                    "one\"\"\"\n\"\"\"two\n\"\"\"\"\nend".to_owned()
                ))
            ]
        );

        // err: the content does not start on a new line
        assert!(matches!(
            lex_from_str(
                r#"
            """hello"""
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // err: the ending marker does not start on a new line
        assert!(matches!(
            lex_from_str(
                r#"
        """
        hello"""
        "#
            ),
            Err(ParseError { message: _ })
        ));

        // err: the ending marker does not occupy the whole line
        assert!(matches!(
            lex_from_str(
                r#"
            """
            hello
            """world
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // err: missing the ending marker
        assert!(matches!(
            lex_from_str(
                r#"
            """
            hello
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_lex_symbol() {
        assert_eq!(
            lex_from_str("name").unwrap(),
            vec![Token::new_symbol("name")]
        );

        assert_eq!(
            lex_from_str("i32.imm").unwrap(),
            vec![Token::new_symbol("i32.imm")]
        );

        assert_eq!(
            lex_from_str("i32.div_s").unwrap(),
            vec![Token::new_symbol("i32.div_s")]
        );

        assert_eq!(
            lex_from_str("(name)").unwrap(),
            vec![
                Token::LeftParen,
                Token::new_symbol("name"),
                Token::RightParen
            ]
        );

        assert_eq!(
            lex_from_str("( a )").unwrap(),
            vec![Token::LeftParen, Token::new_symbol("a"), Token::RightParen]
        );

        assert_eq!(
            lex_from_str("foo bar").unwrap(),
            vec![Token::new_symbol("foo"), Token::new_symbol("bar"),]
        );

        // err: invalid symbol
        assert!(matches!(
            lex_from_str("1abc"),
            Err(ParseError { message: _ })
        ));

        // err: invalid char for symbol
        assert!(matches!(
            lex_from_str("abc+xyz"),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_lex_shebang() {
        assert_eq!(
            lex_from_str(
                r#"#!/bin/ancl
                name
            "#
            )
            .unwrap(),
            vec![
                Token::Shebang("/bin/ancl".to_owned()),
                Token::new_symbol("name"),
                Token::NewLine
            ]
        );
    }

    #[test]
    fn test_lex_assembly_text() {
        assert_eq!(
            lex_from_str(
                r#"
            (local $a i32)
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::LeftParen,
                Token::new_symbol("local"),
                Token::new_identifier("a"),
                Token::new_symbol("i32"),
                Token::RightParen,
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
            (i32.imm 211)
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::LeftParen,
                Token::new_symbol("i32.imm"),
                Token::new_dec_number("211"),
                Token::RightParen,
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
            (i32.imm 0x223) // line comment
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::LeftParen,
                Token::new_symbol("i32.imm"),
                Token::new_hex_number("223"),
                Token::RightParen,
                Token::Comment(CommentToken::Line(" line comment".to_owned())),
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
            (i32.imm /* block comment */ 0x11)
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::LeftParen,
                Token::new_symbol("i32.imm"),
                Token::Comment(CommentToken::Block(" block comment ".to_owned())),
                Token::new_hex_number("11"),
                Token::RightParen,
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
            (i32.imm /* nested /* block comment */*/ 0x11_22)
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::LeftParen,
                Token::new_symbol("i32.imm"),
                Token::Comment(CommentToken::Block(
                    " nested /* block comment */".to_owned()
                )),
                Token::new_hex_number("11_22"),
                Token::RightParen,
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
            (i32.div_s          // multiple lines
                (i32.imm 11)    // left-hand-side
                (i32.imm /*right hand side*/ 17)
            )
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::LeftParen,
                Token::new_symbol("i32.div_s"),
                Token::Comment(CommentToken::Line(" multiple lines".to_owned())),
                Token::LeftParen,
                Token::new_symbol("i32.imm"),
                Token::new_dec_number("11"),
                Token::RightParen,
                Token::Comment(CommentToken::Line(" left-hand-side".to_owned())),
                Token::LeftParen,
                Token::new_symbol("i32.imm"),
                Token::Comment(CommentToken::Block("right hand side".to_owned())),
                Token::new_dec_number("17"),
                Token::RightParen,
                Token::NewLine,
                Token::RightParen,
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"
            (import
                (module
                    (local "math")
                    (function $add "add" (param $lhs i32))
                )
            )
            "#
            )
            .unwrap(),
            vec![
                Token::NewLine,
                Token::LeftParen,
                Token::new_symbol("import"),
                Token::NewLine,
                Token::LeftParen,
                Token::new_symbol("module"),
                Token::NewLine,
                Token::LeftParen,
                Token::new_symbol("local"),
                Token::new_string("math"),
                Token::RightParen,
                Token::NewLine,
                Token::LeftParen,
                Token::new_symbol("function"),
                Token::new_identifier("add"),
                Token::new_string("add"),
                Token::LeftParen,
                Token::new_symbol("param"),
                Token::new_identifier("lhs"),
                Token::new_symbol("i32"),
                Token::RightParen,
                Token::RightParen,
                Token::NewLine,
                Token::RightParen,
                Token::NewLine,
                Token::RightParen,
                Token::NewLine,
            ]
        );

        assert_eq!(
            lex_from_str(
                r#"#!/bin/ancl
                (module)
            "#
            )
            .unwrap(),
            vec![
                Token::Shebang("/bin/ancl".to_owned()),
                Token::LeftParen,
                Token::new_symbol("module"),
                Token::RightParen,
                Token::NewLine,
            ]
        );
    }

    #[test]
    fn test_filter() {
        assert_eq!(
            filter(
                &lex_from_str(
                    r#"#!/bin/ancl
            /* block comment */ 11 // line comment
            """
            document comment
            """
            13
            "#
                )
                .unwrap()
            ),
            vec![Token::new_dec_number("11"), Token::new_dec_number("13"),]
        );
    }
}
