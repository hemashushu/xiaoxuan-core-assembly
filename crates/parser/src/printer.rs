// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use hexfloat2::format;

use crate::ast::{
    ArgumentValue, BlockNode, BreakNode, DataNode, DataSection, DataValue, ExpressionNode,
    ExternalData, ExternalFunction, ExternalNode, FixedMemoryDataType, FunctionDataType,
    FunctionNode, IfNode, InstructionNode, LiteralNumber, LocalVariable, MemoryDataType,
    ModuleNode, NamedParameter, UseNode, WhenNode,
};
use std::fmt::Display;
use std::io::{Error, Write};

pub const DEFAULT_INDENT_CHARS: &str = "    ";

fn print_function_node(
    writer: &mut dyn Write,
    node: &FunctionNode,
    indent_chars: &str,
) -> Result<(), Error> {
    if node.is_public {
        write!(writer, "pub ")?;
    }

    write!(
        writer,
        "fn {}{} -> {}",
        node.name,
        format_params(&node.params),
        format_returns(&node.returns)
    )?;

    if !node.locals.is_empty() {
        let list = format_local_variables(&node.locals);
        write!(writer, "\n{}{}", indent_chars, list)?;
    }

    write!(
        writer,
        " {{\n{}{}\n}}",
        indent_chars,
        format_expression(&node.body, indent_chars, 1)
    )
}

fn print_data_node(
    writer: &mut dyn Write,
    node: &DataNode,
    indent_chars: &str,
) -> Result<(), Error> {
    if node.is_public {
        write!(writer, "pub ")?;
    }

    match &node.data_section {
        DataSection::ReadOnly(sec) => {
            write!(
                writer,
                "readonly data {}:{} = {}",
                node.name,
                sec.data_type,
                format_data_value(&sec.value, indent_chars, 0)
            )?;
        }
        DataSection::ReadWrite(sec) => {
            write!(
                writer,
                "data {}:{} = {}",
                node.name,
                sec.data_type,
                format_data_value(&sec.value, indent_chars, 0)
            )?;
        }
        DataSection::Uninit(data_type) => {
            write!(writer, "uninit data {}:{}", node.name, data_type)?;
        }
    }

    Ok(())
}

fn print_external_node(writer: &mut dyn Write, node: &ExternalNode) -> Result<(), Error> {
    match node {
        ExternalNode::Function(f) => print_external_function(writer, f),
        ExternalNode::Data(d) => print_external_data(writer, d),
    }
}

fn print_external_function(
    writer: &mut dyn Write,
    external_function: &ExternalFunction,
) -> Result<(), Error> {
    write!(
        writer,
        "external fn {}::{}",
        external_function.library, external_function.name
    )?;
    write!(
        writer,
        "({})",
        external_function
            .params
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    )?;
    write!(writer, " -> ")?;

    if let Some(fdt) = &external_function.return_ {
        write!(writer, "{}", fdt)?;
    } else {
        write!(writer, "()")?;
    }

    if let Some(alias) = &external_function.alias_name {
        write!(writer, " as {}", alias)?;
    }
    Ok(())
}

fn print_external_data(writer: &mut dyn Write, external_data: &ExternalData) -> Result<(), Error> {
    write!(
        writer,
        "external data {}::{}:{}",
        external_data.library, external_data.name, external_data.data_type
    )?;

    if let Some(alias) = &external_data.alias_name {
        write!(writer, " as {}", alias)?;
    }
    Ok(())
}

fn print_use_node(writer: &mut dyn Write, node: &UseNode) -> Result<(), Error> {
    write!(writer, "use {}", node.name_path)?;
    if let Some(alias) = &node.alias_name {
        write!(writer, " as {}", alias)?;
    }
    Ok(())
}

fn print_module_node(
    writer: &mut dyn Write,
    node: &ModuleNode,
    indent_chars: &str,
) -> Result<(), Error> {
    if !node.uses.is_empty() {
        for item in &node.uses {
            print_use_node(writer, item)?;
            writeln!(writer)?;
        }
        writeln!(writer)?;
    }

    if !node.externals.is_empty() {
        for item in &node.externals {
            print_external_node(writer, item)?;
            writeln!(writer)?;
        }
        writeln!(writer)?;
    }

    if !node.datas.is_empty() {
        for item in &node.datas {
            print_data_node(writer, item, indent_chars)?;
            writeln!(writer)?;
        }
        writeln!(writer)?;
    }

    for item in &node.functions {
        print_function_node(writer, item, indent_chars)?;
        writeln!(writer)?;
    }

    Ok(())
}

fn format_expression(node: &ExpressionNode, indent_chars: &str, indent_level: usize) -> String {
    match node {
        ExpressionNode::Group(expression_nodes) => {
            format_expression_group(expression_nodes, indent_chars, indent_level)
        }
        ExpressionNode::Instruction(instruction_node) => {
            format_expression_instruction(instruction_node, indent_chars, indent_level)
        }
        ExpressionNode::When(when_node) => {
            format_expression_when(when_node, indent_chars, indent_level)
        }
        ExpressionNode::If(if_node) => format_expression_if(if_node, indent_chars, indent_level),
        ExpressionNode::Block(block_node) => {
            format_expression_block(block_node, false, indent_chars, indent_level)
        }
        ExpressionNode::For(for_node) => {
            format_expression_block(for_node, true, indent_chars, indent_level)
        }
        ExpressionNode::Break(break_node) => {
            format_expression_break(break_node, false, indent_chars, indent_level)
        }
        ExpressionNode::Recur(recur_node) => {
            format_expression_break(recur_node, true, indent_chars, indent_level)
        }
    }
}

fn format_expression_instruction(
    node: &InstructionNode,
    indent_chars: &str,
    indent_level: usize,
) -> String {
    // name(position_args, ..., named_args, ...)
    let pas = node.position_args.iter().map(|item| match item {
        ArgumentValue::Identifier(id) => id.to_owned(),
        ArgumentValue::LiteralNumber(num) => format_literal_number(num),
        ArgumentValue::Expression(exp) => {
            format!(
                "\n{}{}",
                indent_chars.repeat(indent_level + 1),
                format_expression(exp, indent_chars, indent_level + 1)
            )
        }
    });

    let nas = node.named_args.iter().map(|item| {
        format!(
            "{}={}",
            &item.name,
            match &item.value {
                ArgumentValue::Identifier(id) => id.to_owned(),
                ArgumentValue::LiteralNumber(num) => format_literal_number(num),
                ArgumentValue::Expression(exp) =>
                    format_expression(exp, indent_chars, indent_level + 1),
            }
        )
    });

    let mut args = pas.chain(nas).collect::<Vec<String>>();

    args.iter_mut().skip(1).for_each(|item| {
        if item.starts_with('\n') {
            *item = format!(",{}", item)
        } else {
            *item = format!(", {}", item)
        }
    });

    format!("{}({})", node.name, args.join(""))
}

fn format_expression_group(
    nodes: &[ExpressionNode],
    indent_chars: &str,
    indent_level: usize,
) -> String {
    // ```
    // {
    //     expression0
    //     expression1
    //     ...
    // }
    // ```
    format!(
        "{{\n{}\n{}}}",
        format_expression_list(nodes, indent_chars, indent_level + 1),
        indent_chars.repeat(indent_level)
    )
}

fn format_expression_list(
    nodes: &[ExpressionNode],
    indent_chars: &str,
    indent_level: usize,
) -> String {
    // ```
    // expression0
    // expression1
    // ...
    // ```
    let indent = indent_chars.repeat(indent_level);
    nodes
        .iter()
        .map(|item| {
            format!(
                "{}{}",
                indent,
                format_expression(item, indent_chars, indent_level)
            )
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn format_expression_when(node: &WhenNode, indent_chars: &str, indent_level: usize) -> String {
    // ```
    // when
    //     testing
    //     [local variables]
    //     consequence
    // ```

    let indent = indent_chars.repeat(indent_level + 1);

    if node.locals.is_empty() {
        format!(
            "when\n{}{}\n{}{}",
            indent,
            format_expression(&node.testing, indent_chars, indent_level + 1),
            indent,
            format_expression(&node.consequence, indent_chars, indent_level + 1),
        )
    } else {
        format!(
            "when\n{}{}\n{}{}\n{}{}",
            indent,
            format_expression(&node.testing, indent_chars, indent_level + 1),
            indent,
            format_local_variables(&node.locals),
            indent,
            format_expression(&node.consequence, indent_chars, indent_level + 1),
        )
    }
}

fn format_expression_if(node: &IfNode, indent_chars: &str, indent_level: usize) -> String {
    // ```
    // if -> (...)
    //     testing
    //     consequence
    //     alternative
    // ```

    let indent = indent_chars.repeat(indent_level + 1);

    format!(
        "{}\n{}{}\n{}{}\n{}{}",
        format!("if -> {}", format_returns(&node.returns)),
        indent,
        format_expression(&node.testing, indent_chars, indent_level + 1),
        indent,
        format_expression(&node.consequence, indent_chars, indent_level + 1),
        indent,
        format_expression(&node.alternative, indent_chars, indent_level + 1),
    )
}

fn format_params(params: &[NamedParameter]) -> String {
    // this function returns:
    // (name0:data_type0, name1:data_type1, ...)
    format!(
        "({})",
        params
            .iter()
            .map(|item| { format!("{}:{}", item.name, item.data_type) })
            .collect::<Vec<String>>()
            .join(", ")
    )
}

fn format_returns(returns: &[FunctionDataType]) -> String {
    // this function returns:
    // - ()
    // - data_type
    // - (data_type0, data_type1, ...)
    if returns.is_empty() {
        "()".to_owned()
    } else if returns.len() == 1 {
        returns[0].to_string()
    } else {
        format!(
            "({})",
            returns
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

fn format_local_variables(locals: &[LocalVariable]) -> String {
    let list = locals
        .iter()
        .map(|item| format!("{}:{}", item.name, item.data_type))
        .collect::<Vec<String>>()
        .join(", ");
    format!("[{}]", list)
}

fn format_expression_block(
    node: &BlockNode,
    is_for: bool,
    indent_chars: &str,
    indent_level: usize,
) -> String {
    // ```
    // block (...) -> (...)
    //     [locals]
    //     expression
    // ```

    let indent = indent_chars.repeat(indent_level + 1);

    if node.locals.is_empty() {
        format!(
            "{} {} -> {}\n{}{}",
            /* title */ if is_for { "for" } else { "block" },
            /* params */ format_params(&node.params),
            /* returns */ format_returns(&node.returns),
            indent,
            format_expression(&node.body, indent_chars, indent_level + 1)
        )
    } else {
        format!(
            "{} {} -> {}\n{}{}\n{}{}",
            /* title */ if is_for { "for" } else { "block" },
            /* params */ format_params(&node.params),
            /* returns */ format_returns(&node.returns),
            indent,
            format_local_variables(&node.locals),
            indent,
            format_expression(&node.body, indent_chars, indent_level + 1)
        )
    }
}

fn format_expression_break(
    node: &BreakNode,
    is_recur: bool,
    indent_chars: &str,
    indent_level: usize,
) -> String {
    let indent = indent_chars.repeat(indent_level + 1);

    match node {
        BreakNode::Break(nodes) => {
            format!(
                "{} (\n{}\n{})",
                if is_recur { "recur" } else { "break" },
                format_expression_list(nodes, indent_chars, indent_level + 1),
                indent_chars.repeat(indent_level)
            )
        }
        BreakNode::BreakIf(testing, nodes) => {
            format!(
                "{}\n{}{}\n{}(\n{}\n{})",
                if is_recur { "recur_if" } else { "break_if" },
                indent,
                format_expression(testing, indent_chars, indent_level + 1),
                indent,
                format_expression_list(nodes, indent_chars, indent_level + 1),
                indent_chars.repeat(indent_level)
            )
        }
        BreakNode::BreakFn(nodes) => {
            format!(
                "{} (\n{}\n{})",
                if is_recur { "recur" } else { "break" },
                format_expression_list(nodes, indent_chars, indent_level + 1),
                indent_chars.repeat(indent_level)
            )
        }
    }
}

fn format_literal_number(num: &LiteralNumber) -> String {
    match num {
        LiteralNumber::I8(v) => {
            format!("{}_i8", v)
        }
        LiteralNumber::I16(v) => {
            format!("{}_i16", v)
        }
        LiteralNumber::I32(v) => {
            // default type for integer numbers
            format!("{}", v)
        }
        LiteralNumber::I64(v) => {
            format!("{}_i64", v)
        }
        LiteralNumber::F32(v) => {
            format!("{}_f32", v)
        }
        LiteralNumber::F64(v) => {
            // default type for floating-point number s
            // a decimal point needs to be appended if there is no decimal point
            // in the literal.
            let mut s = v.to_string();
            if !s.contains('.') {
                s.push_str(".0");
            }
            format!("{}", s)
        }
    }
}

// fn print_char(writer: &mut dyn Write, ch: &char) -> Result<(), Error> {
//     // escape single char
//     let s = match ch {
//         '\\' => "\\\\".to_owned(),
//         '\'' => "\\'".to_owned(),
//         '\t' => {
//             // horizontal tabulation
//             "\\t".to_owned()
//         }
//         '\r' => {
//             // carriage return, jump to the beginning of the line (CR)
//             "\\r".to_owned()
//         }
//         '\n' => {
//             // new line/line feed (LF)
//             "\\n".to_owned()
//         }
//         '\0' => {
//             // null char
//             "\\0".to_owned()
//         }
//         _ => ch.to_string(),
//     };
//
//     write!(writer, "'{}'", s)
// }

fn format_string(s: &str) -> String {
    format!(
        "\"{}\"",
        s.chars()
            .map(|c| match c {
                '\\' => "\\\\".to_owned(),
                '"' => "\\\"".to_owned(),

                // null char is allowed in the source code,
                // it is used to represent the null-terminated string.
                '\0' => "\\0".to_owned(),

                // some text editors automatically remove the tab at
                // the end of a line, so it is best to escape the tab character.
                '\t' => "\\t".to_owned(),

                _ => c.to_string(),
            })
            .collect::<Vec<String>>()
            .join("")
    )
}

/// format the byte array with fixed length hex:
//
// e.g.
//
// [
//     00 11 22 33  44 55 66 77
//     88 99 aa bb  cc dd ee ff
// ]
fn format_byte_array(data: &[u8], indent_chars: &str) -> String {
    data.chunks(8)
        // .enumerate()
        // .map(|(chunk_addr, chunk)| {
        .map(|chunk| {
            // content
            let content = chunk
                .iter()
                .enumerate()
                .map(|(idx, byte)| {
                    // format the bytes as the following text:
                    // 00 11 22 33  44 55 66 77
                    // 00 11 22 33
                    // 00 11
                    //
                    // Rust std format!()
                    // https://doc.rust-lang.org/std/fmt/
                    if idx == 4 {
                        format!("  {:02x}", byte)
                    } else if idx == 0 {
                        format!("{:02x}", byte)
                    } else {
                        format!(" {:02x}", byte)
                    }
                })
                .collect::<Vec<String>>()
                .join("");

            // address
            // let address = format!("0x{:04x}  {}", chunk_addr * 8, binary);

            format!("{}{}", indent_chars, content)
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn format_data_value(data_value: &DataValue, indent_chars: &str, indent_level: usize) -> String {
    match data_value {
        DataValue::I8(v) => format!("{}_i8", v),
        DataValue::I16(v) => format!("{}_i16", v),
        DataValue::I64(v) => format!("{}_i64", v),
        DataValue::I32(v) => format!("{}", v), // the default type for integer
        DataValue::F64(v) => format!("{}", v), // the default type for floating-point
        DataValue::F32(v) => format!("{}_f32", v),
        // DataValue::Byte(v) => {
        //     format!(
        //         "[\n{}\n{}]",
        //         format_byte_array(v, &indent_chars.repeat(indent_level + 1)),
        //         indent_chars.repeat(indent_level),
        //     )
        // }
        DataValue::String(v) => format_string(v),
        DataValue::List(v) => format!(
            "[\n{}\n{}]",
            v.iter()
                .map(|item| format!(
                    "{}{}",
                    indent_chars.repeat(indent_level + 1),
                    format_data_value(item, indent_chars, indent_level + 1)
                ))
                .collect::<Vec<String>>()
                .join("\n"),
            indent_chars.repeat(indent_level)
        ),
    }
}

pub fn print_to_writer(writer: &mut dyn Write, node: &ModuleNode) -> Result<(), Error> {
    // let mut printer = Printer::new(DEFAULT_INDENT_CHARS, writer);
    print_module_node(writer, node, DEFAULT_INDENT_CHARS)
}

pub fn print_to_string(node: &ModuleNode) -> String {
    // let mut buf = String::new();
    let mut buf: Vec<u8> = vec![];
    print_to_writer(&mut buf, node).unwrap();
    String::from_utf8(buf).unwrap()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        ast::{
            ArgumentValue, BlockNode, BreakNode, DataNode, DataSection, DataTypeValuePair,
            DataValue, ExpressionNode, ExternalData, ExternalFunction, ExternalNode,
            FixedMemoryDataType, FunctionDataType, FunctionNode, IfNode, InstructionNode,
            LiteralNumber, LocalVariable, MemoryDataType, ModuleNode, NamedArgument,
            NamedParameter, UseNode, WhenNode,
        },
        printer::{
            print_external_data, print_external_function, print_function_node, print_use_node,
            DEFAULT_INDENT_CHARS,
        },
    };

    use super::{print_data_node, print_to_string};

    #[test]
    fn test_print_use_node() {
        let print = |node: &UseNode| {
            let mut buf: Vec<u8> = vec![];
            print_use_node(&mut buf, node).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let node0 = UseNode {
            name_path: "foo::bar".to_owned(),
            alias_name: None,
        };

        assert_eq!(print(&node0), "use foo::bar");

        let node1 = UseNode {
            name_path: "foo::bar::Baz".to_owned(),
            alias_name: Some("Bar".to_owned()),
        };
        assert_eq!(print(&node1), "use foo::bar::Baz as Bar");
    }

    #[test]
    fn test_print_external_function() {
        let print = |e: &ExternalFunction| {
            let mut buf: Vec<u8> = vec![];
            print_external_function(&mut buf, e).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let f0 = ExternalFunction {
            library: "libfoo".to_owned(),
            name: "bar".to_owned(),
            params: vec![],
            return_: None,
            alias_name: None,
        };

        assert_eq!(print(&f0), "external fn libfoo::bar() -> ()");

        let f1 = ExternalFunction {
            library: "libfoo".to_owned(),
            name: "bar".to_owned(),
            params: vec![FunctionDataType::I32, FunctionDataType::I32],
            return_: Some(FunctionDataType::I64),
            alias_name: Some("baz".to_owned()),
        };

        assert_eq!(
            print(&f1),
            "external fn libfoo::bar(i32, i32) -> i64 as baz"
        );
    }

    #[test]
    fn test_print_external_data() {
        let print = |e: &ExternalData| {
            let mut buf: Vec<u8> = vec![];
            print_external_data(&mut buf, e).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let d0 = ExternalData {
            library: "libfoo".to_owned(),
            name: "count".to_owned(),
            data_type: MemoryDataType::I32,
            alias_name: None,
        };

        assert_eq!(print(&d0), "external data libfoo::count:i32");

        let d1 = ExternalData {
            library: "libfoo".to_owned(),
            name: "got".to_owned(),
            data_type: MemoryDataType::FixedBytes(128, None),
            alias_name: Some("global_offset_table".to_owned()),
        };

        assert_eq!(
            print(&d1),
            "external data libfoo::got:byte[128] as global_offset_table"
        );
    }

    #[test]
    fn test_print_data_node() {
        let print = |node: &DataNode| {
            let mut buf: Vec<u8> = vec![];
            print_data_node(&mut buf, node, DEFAULT_INDENT_CHARS).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let node0 = DataNode {
            is_public: false,
            name: "foo".to_owned(),
            data_section: DataSection::ReadOnly(DataTypeValuePair {
                data_type: MemoryDataType::I32,
                value: DataValue::I32(123),
            }),
        };

        assert_eq!(print(&node0), "readonly data foo:i32 = 123");

        // test byte array data type
        let node1 = DataNode {
            is_public: true,
            name: "foo".to_owned(),
            data_section: DataSection::ReadOnly(DataTypeValuePair {
                data_type: MemoryDataType::FixedBytes(32, None),
                value: DataValue::String("hello".to_owned()),
            }),
        };

        assert_eq!(print(&node1), "pub readonly data foo:byte[32] = \"hello\"");

        // test byte array data type with unspecific length
        let node2 = DataNode {
            is_public: true,
            name: "foo".to_owned(),
            data_section: DataSection::ReadWrite(DataTypeValuePair {
                data_type: MemoryDataType::Bytes(None),
                value: DataValue::String("world".to_owned()),
            }),
        };

        assert_eq!(print(&node2), "pub data foo:byte[] = \"world\"");

        // test uninit
        let node3 = DataNode {
            is_public: false,
            name: "got".to_owned(),
            data_section: DataSection::Uninit(FixedMemoryDataType::FixedBytes(1024, None)),
        };

        assert_eq!(print(&node3), "uninit data got:byte[1024]");

        // test align
        let node4 = DataNode {
            is_public: false,
            name: "foo".to_owned(),
            data_section: DataSection::Uninit(FixedMemoryDataType::FixedBytes(1024, Some(8))),
        };

        assert_eq!(print(&node4), "uninit data foo:byte[1024, align=8]");

        // test data value list
        let node5 = DataNode {
            is_public: false,
            name: "bar".to_owned(),
            data_section: DataSection::ReadWrite(DataTypeValuePair {
                data_type: MemoryDataType::Bytes(Some(4)),
                value: DataValue::List(vec![
                    DataValue::I8(11),
                    DataValue::I16(13),
                    DataValue::I32(17),
                    DataValue::I64(19),
                    DataValue::String("hello".to_owned()),
                    DataValue::List(vec![DataValue::I8(211), DataValue::I8(223)]),
                ]),
            }),
        };

        assert_eq!(
            print(&node5),
            "\
data bar:byte[align=4] = [
    11_i8
    13_i16
    17
    19_i64
    \"hello\"
    [
        211_i8
        223_i8
    ]
]"
        );
    }

    #[test]
    fn test_print_function_node_and_expression_instruction() {
        let print = |node: &FunctionNode| {
            let mut buf: Vec<u8> = vec![];
            print_function_node(&mut buf, node, DEFAULT_INDENT_CHARS).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let node0 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::Instruction(InstructionNode {
                name: "local_load_i64".to_owned(),
                position_args: vec![ArgumentValue::Identifier("left".to_owned())],
                named_args: vec![
                    NamedArgument {
                        name: "rindex".to_owned(),
                        value: ArgumentValue::LiteralNumber(LiteralNumber::I16(1)),
                    },
                    NamedArgument {
                        name: "offset".to_owned(),
                        value: ArgumentValue::LiteralNumber(LiteralNumber::I16(4)),
                    },
                ],
            }),
        };

        assert_eq!(
            print(&node0),
            "\
fn foo() -> () {
    local_load_i64(left, rindex=1_i16, offset=4_i16)
}"
        );

        // test params, returns and multiple layers instructions
        let node1 = FunctionNode {
            is_public: true,
            name: "add".to_owned(),
            params: vec![
                NamedParameter {
                    name: "left".to_owned(),
                    data_type: FunctionDataType::I32,
                },
                NamedParameter {
                    name: "right".to_owned(),
                    data_type: FunctionDataType::I32,
                },
            ],
            returns: vec![FunctionDataType::I32],
            locals: vec![],
            body: ExpressionNode::Instruction(InstructionNode {
                name: "add_i32".to_owned(),
                position_args: vec![
                    ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                        InstructionNode {
                            name: "local_load_i32".to_owned(),
                            position_args: vec![ArgumentValue::Identifier("left".to_owned())],
                            named_args: vec![],
                        },
                    ))),
                    ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                        InstructionNode {
                            name: "add_imm_i32".to_owned(),
                            position_args: vec![
                                ArgumentValue::LiteralNumber(LiteralNumber::I16(11)),
                                ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                                    InstructionNode {
                                        name: "local_load_i32".to_owned(),
                                        position_args: vec![ArgumentValue::Identifier(
                                            "right".to_owned(),
                                        )],
                                        named_args: vec![],
                                    },
                                ))),
                            ],
                            named_args: vec![],
                        },
                    ))),
                ],
                named_args: vec![],
            }),
        };

        assert_eq!(
            print(&node1),
            "\
pub fn add(left:i32, right:i32) -> i32 {
    add_i32(
        local_load_i32(left),
        add_imm_i32(11_i16,
            local_load_i32(right)))
}"
        );

        // test returns multiple values and local variables
        let node2 = FunctionNode {
            is_public: false,
            name: "hello".to_owned(),
            params: vec![],
            returns: vec![FunctionDataType::I32, FunctionDataType::I64],
            locals: vec![
                LocalVariable {
                    name: "foo".to_owned(),
                    data_type: FixedMemoryDataType::I32,
                },
                LocalVariable {
                    name: "bar".to_owned(),
                    data_type: FixedMemoryDataType::FixedBytes(8, None),
                },
                LocalVariable {
                    name: "baz".to_owned(),
                    data_type: FixedMemoryDataType::FixedBytes(24, Some(4)),
                },
            ],
            body: ExpressionNode::Instruction(InstructionNode {
                name: "end".to_owned(),
                position_args: vec![],
                named_args: vec![],
            }),
        };

        assert_eq!(
            print(&node2),
            "\
fn hello() -> (i32, i64)
    [foo:i32, bar:byte[8], baz:byte[24, align=4]] {
    end()
}"
        );
    }

    #[test]
    fn test_print_expression_group() {
        let print = |node: &FunctionNode| {
            let mut buf: Vec<u8> = vec![];
            print_function_node(&mut buf, node, DEFAULT_INDENT_CHARS).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let node0 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::Group(vec![
                ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    position_args: vec![],
                    named_args: vec![],
                }),
                ExpressionNode::Instruction(InstructionNode {
                    name: "local_store_i32".to_owned(),
                    position_args: vec![
                        ArgumentValue::Identifier("left".to_owned()),
                        ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                            InstructionNode {
                                name: "imm_i32".to_owned(),
                                position_args: vec![ArgumentValue::LiteralNumber(
                                    LiteralNumber::I32(123),
                                )],
                                named_args: vec![],
                            },
                        ))),
                    ],
                    named_args: vec![],
                }),
                ExpressionNode::Instruction(InstructionNode {
                    name: "local_store_i32".to_owned(),
                    position_args: vec![
                        ArgumentValue::Identifier("right".to_owned()),
                        ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                            InstructionNode {
                                name: "add_i32".to_owned(),
                                position_args: vec![
                                    ArgumentValue::Expression(Box::new(
                                        ExpressionNode::Instruction(InstructionNode {
                                            name: "imm_i32".to_owned(),
                                            position_args: vec![ArgumentValue::LiteralNumber(
                                                LiteralNumber::I32(123),
                                            )],
                                            named_args: vec![],
                                        }),
                                    )),
                                    ArgumentValue::Expression(Box::new(
                                        ExpressionNode::Instruction(InstructionNode {
                                            name: "imm_i32".to_owned(),
                                            position_args: vec![ArgumentValue::LiteralNumber(
                                                LiteralNumber::I32(123),
                                            )],
                                            named_args: vec![],
                                        }),
                                    )),
                                ],
                                named_args: vec![],
                            },
                        ))),
                    ],
                    named_args: vec![],
                }),
            ]),
        };

        assert_eq!(
            print(&node0),
            "\
fn foo() -> () {
    {
        nop()
        local_store_i32(left,
            imm_i32(123))
        local_store_i32(right,
            add_i32(
                imm_i32(123),
                imm_i32(123)))
    }
}"
        );

        // test nested group
        let node1 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::Group(vec![
                ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    position_args: vec![],
                    named_args: vec![],
                }),
                ExpressionNode::Group(vec![ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    position_args: vec![],
                    named_args: vec![],
                })]),
            ]),
        };

        assert_eq!(
            print(&node1),
            "\
fn foo() -> () {
    {
        nop()
        {
            nop()
        }
    }
}"
        );
    }

    #[test]
    fn test_print_expression_when() {
        let print = |node: &FunctionNode| {
            let mut buf: Vec<u8> = vec![];
            print_function_node(&mut buf, node, DEFAULT_INDENT_CHARS).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let node0 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::When(WhenNode {
                testing: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "imm_i32".to_owned(),
                    position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(1))],
                    named_args: vec![],
                })),
                locals: vec![],
                consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    position_args: vec![],
                    named_args: vec![],
                })),
            }),
        };

        assert_eq!(
            print(&node0),
            "\
fn foo() -> () {
    when
        imm_i32(1)
        nop()
}"
        );

        // test `when` with multiple layers instructions
        let node1 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::When(WhenNode {
                testing: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "eqz_i32".to_owned(),
                    position_args: vec![ArgumentValue::Expression(Box::new(
                        ExpressionNode::Instruction(InstructionNode {
                            name: "local_load_i32".to_owned(),
                            position_args: vec![ArgumentValue::Identifier("a".to_owned())],
                            named_args: vec![],
                        }),
                    ))],
                    named_args: vec![],
                })),
                locals: vec![],
                consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "data_store_i32".to_owned(),
                    position_args: vec![
                        ArgumentValue::Identifier("b".to_owned()),
                        ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                            InstructionNode {
                                name: "local_load_i32".to_owned(),
                                position_args: vec![ArgumentValue::Identifier("a".to_owned())],
                                named_args: vec![],
                            },
                        ))),
                    ],
                    named_args: vec![],
                })),
            }),
        };

        assert_eq!(
            print(&node1),
            "\
fn foo() -> () {
    when
        eqz_i32(
            local_load_i32(a))
        data_store_i32(b,
            local_load_i32(a))
}"
        );

        // test `when` with local variables
        let node2 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::When(WhenNode {
                testing: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "imm_i32".to_owned(),
                    position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(1))],
                    named_args: vec![],
                })),
                locals: vec![
                    LocalVariable {
                        name: "foo".to_owned(),
                        data_type: FixedMemoryDataType::I32,
                    },
                    LocalVariable {
                        name: "bar".to_owned(),
                        data_type: FixedMemoryDataType::FixedBytes(8, None),
                    },
                    LocalVariable {
                        name: "baz".to_owned(),
                        data_type: FixedMemoryDataType::FixedBytes(24, Some(4)),
                    },
                ],
                consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    position_args: vec![],
                    named_args: vec![],
                })),
            }),
        };

        assert_eq!(
            print(&node2),
            "\
fn foo() -> () {
    when
        imm_i32(1)
        [foo:i32, bar:byte[8], baz:byte[24, align=4]]
        nop()
}"
        );

        // test 'when' + 'group'
        let node3 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::When(WhenNode {
                testing: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "imm_i32".to_owned(),
                    position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(1))],
                    named_args: vec![],
                })),
                locals: vec![],
                consequence: Box::new(ExpressionNode::Group(vec![
                    ExpressionNode::Instruction(InstructionNode {
                        name: "nop".to_owned(),
                        position_args: vec![],
                        named_args: vec![],
                    }),
                    ExpressionNode::Instruction(InstructionNode {
                        name: "local_store_i32".to_owned(),
                        position_args: vec![
                            ArgumentValue::Identifier("left".to_owned()),
                            ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                                InstructionNode {
                                    name: "local_load_i32".to_owned(),
                                    position_args: vec![ArgumentValue::Identifier(
                                        "right".to_owned(),
                                    )],
                                    named_args: vec![],
                                },
                            ))),
                        ],
                        named_args: vec![],
                    }),
                ])),
            }),
        };

        assert_eq!(
            print(&node3),
            "\
fn foo() -> () {
    when
        imm_i32(1)
        {
            nop()
            local_store_i32(left,
                local_load_i32(right))
        }
}"
        );
    }

    #[test]
    fn test_print_expression_if() {
        let print = |node: &FunctionNode| {
            let mut buf: Vec<u8> = vec![];
            print_function_node(&mut buf, node, DEFAULT_INDENT_CHARS).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let node0 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::If(IfNode {
                returns: vec![],
                testing: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "eqz_i32".to_owned(),
                    position_args: vec![ArgumentValue::Expression(Box::new(
                        ExpressionNode::Instruction(InstructionNode {
                            name: "local_load_i32".to_owned(),
                            position_args: vec![ArgumentValue::Identifier("in".to_owned())],
                            named_args: vec![],
                        }),
                    ))],
                    named_args: vec![],
                })),
                consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "local_store_i32".to_owned(),
                    position_args: vec![
                        ArgumentValue::Identifier("out".to_owned()),
                        ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                            InstructionNode {
                                name: "imm_i32".to_owned(),
                                position_args: vec![ArgumentValue::LiteralNumber(
                                    LiteralNumber::I32(11),
                                )],
                                named_args: vec![],
                            },
                        ))),
                    ],
                    named_args: vec![],
                })),
                alternative: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "local_store_i32".to_owned(),
                    position_args: vec![
                        ArgumentValue::Identifier("out".to_owned()),
                        ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                            InstructionNode {
                                name: "imm_i32".to_owned(),
                                position_args: vec![ArgumentValue::LiteralNumber(
                                    LiteralNumber::I32(13),
                                )],
                                named_args: vec![],
                            },
                        ))),
                    ],
                    named_args: vec![],
                })),
            }),
        };

        assert_eq!(
            print(&node0),
            "\
fn foo() -> () {
    if -> ()
        eqz_i32(
            local_load_i32(in))
        local_store_i32(out,
            imm_i32(11))
        local_store_i32(out,
            imm_i32(13))
}"
        );

        // test `if` with return value
        let node1 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::If(IfNode {
                returns: vec![FunctionDataType::I32],
                testing: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "imm_i32".to_owned(),
                    position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(11))],
                    named_args: vec![],
                })),
                consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "imm_i32".to_owned(),
                    position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(13))],
                    named_args: vec![],
                })),
                alternative: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "imm_i32".to_owned(),
                    position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(17))],
                    named_args: vec![],
                })),
            }),
        };

        assert_eq!(
            print(&node1),
            "\
fn foo() -> () {
    if -> i32
        imm_i32(11)
        imm_i32(13)
        imm_i32(17)
}"
        );

        // test `if` with multiple return values
        let node2 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::If(IfNode {
                returns: vec![FunctionDataType::I32, FunctionDataType::I64],
                testing: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "imm_i32".to_owned(),
                    position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(11))],
                    named_args: vec![],
                })),
                consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    position_args: vec![],
                    named_args: vec![],
                })),
                alternative: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    position_args: vec![],
                    named_args: vec![],
                })),
            }),
        };

        assert_eq!(
            print(&node2),
            "\
fn foo() -> () {
    if -> (i32, i64)
        imm_i32(11)
        nop()
        nop()
}"
        );
    }

    #[test]
    fn test_print_expression_block() {
        let print = |node: &FunctionNode| {
            let mut buf: Vec<u8> = vec![];
            print_function_node(&mut buf, node, DEFAULT_INDENT_CHARS).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let node0 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::Block(BlockNode {
                params: vec![],
                returns: vec![],
                locals: vec![],
                body: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "local_store_i32".to_owned(),
                    position_args: vec![
                        ArgumentValue::Identifier("out".to_owned()),
                        ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                            InstructionNode {
                                name: "imm_i32".to_owned(),
                                position_args: vec![ArgumentValue::LiteralNumber(
                                    LiteralNumber::I32(11),
                                )],
                                named_args: vec![],
                            },
                        ))),
                    ],
                    named_args: vec![],
                })),
            }),
        };

        assert_eq!(
            print(&node0),
            "\
fn foo() -> () {
    block () -> ()
        local_store_i32(out,
            imm_i32(11))
}"
        );

        // test 'block' with params, returns and local variablers
        let node1 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::Block(BlockNode {
                params: vec![
                    NamedParameter {
                        name: "left".to_owned(),
                        data_type: FunctionDataType::I32,
                    },
                    NamedParameter {
                        name: "right".to_owned(),
                        data_type: FunctionDataType::I32,
                    },
                ],
                returns: vec![FunctionDataType::I32],
                locals: vec![
                    LocalVariable {
                        name: "abc".to_owned(),
                        data_type: FixedMemoryDataType::I32,
                    },
                    LocalVariable {
                        name: "def".to_owned(),
                        data_type: FixedMemoryDataType::FixedBytes(32, None),
                    },
                ],
                body: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    position_args: vec![],
                    named_args: vec![],
                })),
            }),
        };

        assert_eq!(
            print(&node1),
            "\
fn foo() -> () {
    block (left:i32, right:i32) -> i32
        [abc:i32, def:byte[32]]
        nop()
}"
        );

        // test 'for'
        let node2 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::For(BlockNode {
                params: vec![],
                returns: vec![],
                locals: vec![],
                body: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    position_args: vec![],
                    named_args: vec![],
                })),
            }),
        };

        assert_eq!(
            print(&node2),
            "\
fn foo() -> () {
    for () -> ()
        nop()
}"
        );
    }

    #[test]
    fn test_print_expression_break() {
        let print = |node: &FunctionNode| {
            let mut buf: Vec<u8> = vec![];
            print_function_node(&mut buf, node, DEFAULT_INDENT_CHARS).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let node0 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::Group(vec![
                ExpressionNode::Break(BreakNode::Break(vec![
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(11))],
                        named_args: vec![],
                    }),
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(31))],
                        named_args: vec![],
                    }),
                ])),
                ExpressionNode::Break(BreakNode::BreakIf(
                    Box::new(ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(7))],
                        named_args: vec![],
                    })),
                    vec![
                        ExpressionNode::Instruction(InstructionNode {
                            name: "imm_i32".to_owned(),
                            position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(
                                11,
                            ))],
                            named_args: vec![],
                        }),
                        ExpressionNode::Instruction(InstructionNode {
                            name: "imm_i32".to_owned(),
                            position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(
                                31,
                            ))],
                            named_args: vec![],
                        }),
                    ],
                )),
                ExpressionNode::Break(BreakNode::BreakFn(vec![
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(11))],
                        named_args: vec![],
                    }),
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(31))],
                        named_args: vec![],
                    }),
                ])),
            ]),
        };

        assert_eq!(
            print(&node0),
            "\
fn foo() -> () {
    {
        break (
            imm_i32(11)
            imm_i32(31)
        )
        break_if
            imm_i32(7)
            (
            imm_i32(11)
            imm_i32(31)
        )
        break (
            imm_i32(11)
            imm_i32(31)
        )
    }
}"
        );

        let node1 = FunctionNode {
            is_public: false,
            name: "foo".to_owned(),
            params: vec![],
            returns: vec![],
            locals: vec![],
            body: ExpressionNode::Group(vec![
                ExpressionNode::Recur(BreakNode::Break(vec![
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(11))],
                        named_args: vec![],
                    }),
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(31))],
                        named_args: vec![],
                    }),
                ])),
                ExpressionNode::Recur(BreakNode::BreakIf(
                    Box::new(ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(7))],
                        named_args: vec![],
                    })),
                    vec![
                        ExpressionNode::Instruction(InstructionNode {
                            name: "imm_i32".to_owned(),
                            position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(
                                11,
                            ))],
                            named_args: vec![],
                        }),
                        ExpressionNode::Instruction(InstructionNode {
                            name: "imm_i32".to_owned(),
                            position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(
                                31,
                            ))],
                            named_args: vec![],
                        }),
                    ],
                )),
                ExpressionNode::Recur(BreakNode::BreakFn(vec![
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(11))],
                        named_args: vec![],
                    }),
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        position_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(31))],
                        named_args: vec![],
                    }),
                ])),
            ]),
        };

        assert_eq!(
            print(&node1),
            "\
fn foo() -> () {
    {
        recur (
            imm_i32(11)
            imm_i32(31)
        )
        recur_if
            imm_i32(7)
            (
            imm_i32(11)
            imm_i32(31)
        )
        recur (
            imm_i32(11)
            imm_i32(31)
        )
    }
}"
        );
    }

    fn test_print_module_node() {
        let node = ModuleNode {
            name_path: "foo".to_owned(),
            uses: vec![
                UseNode {
                    name_path: "foo::bar".to_owned(),
                    alias_name: None,
                },
                UseNode {
                    name_path: "foo::bar::Baz".to_owned(),
                    alias_name: Some("Bar".to_owned()),
                },
            ],
            externals: vec![
                ExternalNode::Function(ExternalFunction {
                    library: "libzstd".to_owned(),
                    name: "abc".to_owned(),
                    params: vec![FunctionDataType::I32, FunctionDataType::I64],
                    return_: Some(FunctionDataType::I64),
                    alias_name: None,
                }),
                ExternalNode::Data(ExternalData {
                    library: "libz".to_owned(),
                    name: "def".to_owned(),
                    data_type: MemoryDataType::I32,
                    alias_name: Some("xyz".to_owned()),
                }),
            ],
            datas: vec![],
            functions: vec![],
        };

        assert_eq!(
            print_to_string(&node),
            "use foo::bar
    use foo::bar::Baz as Abc\n\n"
        )
    }
}
