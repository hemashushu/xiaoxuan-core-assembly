// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use anc_isa::{DataSectionType, OperandDataType};

use crate::ast::{
    ArgumentValue, BlockNode, BreakNode, DataNode, DataSection, DataValue, ExpressionNode,
    ExternalDataNode, ExternalFunctionNode, ExternalNode, FunctionNode, IfNode, ImportDataNode,
    ImportFunctionNode, ImportNode, InstructionNode, LiteralNumber, LocalVariable, ModuleNode,
    NameValueParameter, NamedParameter, WhenNode,
};

use std::io::{Error, Write};

pub const DEFAULT_INDENT_CHARS: &str = "    ";

fn print_function_node(
    writer: &mut dyn Write,
    node: &FunctionNode,
    indent_chars: &str,
) -> Result<(), Error> {
    // ```
    // fn name(params) -> results
    //     body
    //
    // fn name(params) -> results
    //     [locals]
    //     body
    // ```

    if node.export {
        write!(writer, "pub ")?;
    }

    if node.locals.is_empty() {
        write!(
            writer,
            "fn {}{} -> {}\n{}{}",
            node.name,
            format_params(&node.params),
            format_results(&node.results),
            indent_chars,
            format_expression(&node.body, indent_chars, 1)
        )
    } else {
        write!(
            writer,
            "fn {}{} -> {}\n{}{}\n{}{}",
            node.name,
            format_params(&node.params),
            format_results(&node.results),
            indent_chars,
            format_local_variables(&node.locals),
            indent_chars,
            format_expression(&node.body, indent_chars, 1)
        )
    }
}

fn print_data_node(
    writer: &mut dyn Write,
    node: &DataNode,
    indent_chars: &str,
) -> Result<(), Error> {
    if node.export {
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

fn print_import_node(writer: &mut dyn Write, node: &ImportNode) -> Result<(), Error> {
    match node {
        ImportNode::Function(node) => print_import_function_node(writer, node),
        ImportNode::Data(node) => print_import_data_node(writer, node),
    }
}

fn print_import_function_node(
    writer: &mut dyn Write,
    node: &ImportFunctionNode,
) -> Result<(), Error> {
    write!(
        writer,
        "import fn {}{} -> {}",
        node.full_name,
        format_import_params(&node.params),
        format_results(&node.results),
    )?;

    if let Some(alias) = &node.alias_name {
        write!(writer, " as {}", alias)?;
    }

    if let Some(from) = &node.from {
        write!(writer, " from {}", from)?;
    }

    Ok(())
}

fn print_import_data_node(writer: &mut dyn Write, node: &ImportDataNode) -> Result<(), Error> {
    match node.data_section_type {
        DataSectionType::ReadOnly => {
            write!(
                writer,
                "import readonly data {} type {}",
                node.full_name, node.data_type
            )?;
        }
        DataSectionType::ReadWrite => {
            write!(writer, "import data {} type {}", node.full_name, node.data_type)?;
        }
        DataSectionType::Uninit => {
            write!(
                writer,
                "import uninit data {} type {}",
                node.full_name, node.data_type
            )?;
        }
    }

    if let Some(alias) = &node.alias_name {
        write!(writer, " as {}", alias)?;
    }

    if let Some(from) = &node.from {
        write!(writer, " from {}", from)?;
    }

    Ok(())
}

fn print_external_node(writer: &mut dyn Write, node: &ExternalNode) -> Result<(), Error> {
    match node {
        ExternalNode::Function(node) => print_external_function_node(writer, node),
        ExternalNode::Data(node) => print_external_data_node(writer, node),
    }
}

fn print_external_function_node(
    writer: &mut dyn Write,
    node: &ExternalFunctionNode,
) -> Result<(), Error> {
    write!(
        writer,
        "external fn {}{} -> {}",
        node.full_name,
        format_import_params(&node.params),
        if let Some(fdt) = &node.result {
            fdt.to_string()
        } else {
            "()".to_owned()
        }
    )?;

    if let Some(alias) = &node.alias_name {
        write!(writer, " as {}", alias)?;
    }
    Ok(())
}

fn print_external_data_node(writer: &mut dyn Write, node: &ExternalDataNode) -> Result<(), Error> {
    write!(
        writer,
        "external data {} type {}",
        node.full_name, node.data_type
    )?;

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
    if !node.imports.is_empty() {
        for item in &node.imports {
            print_import_node(writer, item)?;
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

    for (function_index, item) in node.functions.iter().enumerate() {
        print_function_node(writer, item, indent_chars)?;
        writeln!(writer)?;

        // add a new line between functions
        if function_index != node.functions.len() - 1 {
            writeln!(writer)?;
        }
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
        // ExpressionNode::Branch(branch_node) => {
        //     format_expression_branch(branch_node, indent_chars, indent_level)
        // }
        ExpressionNode::Block(block_node) => {
            format_expression_block(block_node, indent_chars, indent_level)
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
    let pas = node.positional_args.iter().map(|item| match item {
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
    //     [local variables]
    //     testing
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
            format_local_variables(&node.locals),
            indent,
            format_expression(&node.testing, indent_chars, indent_level + 1),
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
        "if -> {}\n{}{}\n{}{}\n{}{}",
        // format_params(&node.params),
        format_results(&node.results),
        indent,
        format_expression(&node.testing, indent_chars, indent_level + 1),
        indent,
        format_expression(&node.consequence, indent_chars, indent_level + 1),
        indent,
        format_expression(&node.alternative, indent_chars, indent_level + 1),
    )
}

// fn format_expression_branch(node: &BranchNode, indent_chars: &str, indent_level: usize) -> String {
//     // ```
//     // branch (...) -> (...)
//     //     [locals]
//     //     {
//     //     case
//     //          testing
//     //          consequence
//     //     case
//     //          testing
//     //          consequence
//     //     default
//     //          consequence
//     //     }
//     //```
//     let indent = indent_chars.repeat(indent_level + 1);
//     let indent2 = indent_chars.repeat(indent_level + 2);
//
//     let cases = node
//         .cases
//         .iter()
//         .map(|item| {
//             format!(
//                 "{}case\n{}{}\n{}{}",
//                 indent,
//                 indent2,
//                 format_expression(&item.testing, indent_chars, indent_level + 2),
//                 indent2,
//                 format_expression(&item.consequence, indent_chars, indent_level + 2)
//             )
//         })
//         .collect::<Vec<String>>()
//         .join("\n");
//
//     if node.locals.is_empty() {
//         format!(
//             "branch {} -> {}\n{}{{\n{}\n{}default\n{}{}\n{}}}",
//             format_params(&node.params),
//             format_results(&node.results),
//             indent,
//             cases,
//             indent,
//             indent2,
//             format_expression(&node.default, indent_chars, indent_level + 2),
//             indent
//         )
//     } else {
//         format!(
//             "branch {} -> {}\n{}{}\n{}{{\n{}\n{}default\n{}{}\n{}}}",
//             format_params(&node.params),
//             format_results(&node.results),
//             indent,
//             format_local_variables(&node.locals),
//             indent,
//             cases,
//             indent,
//             indent2,
//             format_expression(&node.default, indent_chars, indent_level + 2),
//             indent
//         )
//     }
// }

fn format_expression_block(node: &BlockNode, indent_chars: &str, indent_level: usize) -> String {
    // ```
    // block (...) -> (...)
    //     [locals]
    //     expression
    // ```

    let indent = indent_chars.repeat(indent_level + 1);

    if node.locals.is_empty() {
        format!(
            "block {} -> {}\n{}{}",
            /* params */
            format_param_values(&node.param_values, indent_chars, indent_level),
            /* results */ format_results(&node.results),
            indent,
            format_expression(&node.body, indent_chars, indent_level + 1)
        )
    } else {
        format!(
            "block {} -> {}\n{}{}\n{}{}",
            /* params */
            format_param_values(&node.param_values, indent_chars, indent_level),
            /* results */ format_results(&node.results),
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
    // let indent = indent_chars.repeat(indent_level + 1);

    match node {
        BreakNode::Break(nodes) => {
            format!(
                "{}(\n{}\n{})",
                if is_recur { "recur" } else { "break" },
                format_expression_list(nodes, indent_chars, indent_level + 1),
                indent_chars.repeat(indent_level)
            )
        }
        BreakNode::BreakFn(nodes) => {
            format!(
                "{}(\n{}\n{})",
                if is_recur { "recur_fn" } else { "break_fn" },
                format_expression_list(nodes, indent_chars, indent_level + 1),
                indent_chars.repeat(indent_level)
            )
        }
    }
}

/// format:
/// "(name0:data_type0=value0, name1:data_type1=value1, ...)"
fn format_param_values(
    param_values: &[NameValueParameter],
    indent_chars: &str,
    indent_level: usize,
) -> String {
    format!(
        "({})",
        param_values
            .iter()
            .map(|item| {
                format!(
                    "{}:{}={}",
                    item.name,
                    item.data_type,
                    format_expression(&item.value, indent_chars, indent_level)
                )
            })
            .collect::<Vec<String>>()
            .join(", ")
    )
}

/// format:
/// "(name0:data_type0, name1:data_type1, ...)"
fn format_params(params: &[NamedParameter]) -> String {
    format!(
        "({})",
        params
            .iter()
            .map(|item| { format!("{}:{}", item.name, item.data_type) })
            .collect::<Vec<String>>()
            .join(", ")
    )
}

fn format_import_params(params: &[OperandDataType]) -> String {
    format!(
        "({})",
        params
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    )
}

/// format:
/// - ()
/// - data_type
/// - (data_type0, data_type1, ...)
fn format_results(results: &[OperandDataType]) -> String {
    if results.is_empty() {
        "()".to_owned()
    } else if results.len() == 1 {
        results[0].to_string()
    } else {
        format!(
            "({})",
            results
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
            s
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
///
/// e.g.
///
/// h"00 11 22 33  44 55 66 77
///   88 99 aa bb  cc dd ee ff"
///
fn format_hex_byte_data(data: &[u8], indent_chars: &str) -> String {
    let line_sep = format!("\n{}", indent_chars);
    let content = data
        .chunks(8)
        .map(|chunk| {
            // line
            chunk
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
                .join("")
        })
        .collect::<Vec<String>>()
        .join(&line_sep);

    format!("h\"{}\"", content)
}

fn format_data_value(data_value: &DataValue, indent_chars: &str, indent_level: usize) -> String {
    match data_value {
        DataValue::I8(v) => format!("{}_i8", v),
        DataValue::I16(v) => format!("{}_i16", v),
        DataValue::I64(v) => format!("{}_i64", v),
        DataValue::I32(v) => format!("{}", v), // the default type for integer
        DataValue::F64(v) => format!("{}", v), // the default type for floating-point
        DataValue::F32(v) => format!("{}_f32", v),
        DataValue::ByteData(v) => format_hex_byte_data(v, &indent_chars.repeat(indent_level + 1)),
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
    use anc_isa::{DataSectionType, MemoryDataType, OperandDataType};
    use pretty_assertions::assert_eq;

    use crate::ast::{
        ArgumentValue, BlockNode, BreakNode, DataNode, DataSection, DataTypeValuePair, DataValue,
        DeclareDataType, ExpressionNode, ExternalDataNode, ExternalFunctionNode, ExternalNode,
        FixedDeclareDataType, FunctionNode, IfNode, ImportDataNode, ImportFunctionNode, ImportNode,
        InstructionNode, LiteralNumber, LocalVariable, ModuleNode, NameValueParameter,
        NamedArgument, NamedParameter, WhenNode,
    };

    use crate::printer::{
        print_external_data_node, print_external_function_node, print_function_node,
        print_import_data_node, print_import_function_node, DEFAULT_INDENT_CHARS,
    };

    use super::{print_data_node, print_to_string};

    // #[test]
    // fn test_print_use_node() {
    //     let print = |node: &UseNode| {
    //         let mut buf: Vec<u8> = vec![];
    //         print_use_node(&mut buf, node).unwrap();
    //         String::from_utf8(buf).unwrap()
    //     };
    //
    //     let node0 = UseNode {
    //         full_name: "foo::bar".to_owned(),
    //         alias_name: None,
    //     };
    //
    //     assert_eq!(print(&node0), "use foo::bar");
    //
    //     let node1 = UseNode {
    //         full_name: "foo::bar::Baz".to_owned(),
    //         alias_name: Some("Bar".to_owned()),
    //     };
    //     assert_eq!(print(&node1), "use foo::bar::Baz as Bar");
    // }

    #[test]
    fn test_print_import_function_node() {
        let print = |e: &ImportFunctionNode| {
            let mut buf: Vec<u8> = vec![];
            print_import_function_node(&mut buf, e).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let f0 = ImportFunctionNode {
            full_name: "foo::bar".to_owned(),
            params: vec![],
            results: vec![],
            alias_name: None,
            from: None,
        };

        assert_eq!(print(&f0), "import fn foo::bar() -> ()");

        let f1 = ImportFunctionNode {
            full_name: "foo::bar".to_owned(),
            params: vec![OperandDataType::I32, OperandDataType::I32],
            results: vec![OperandDataType::I64, OperandDataType::I64],
            alias_name: Some("baz".to_owned()),
            from: Some("mymod".to_owned()),
        };

        assert_eq!(
            print(&f1),
            "import fn foo::bar(i32, i32) -> (i64, i64) as baz from mymod"
        );
    }

    #[test]
    fn test_print_import_data_node() {
        let print = |e: &ImportDataNode| {
            let mut buf: Vec<u8> = vec![];
            print_import_data_node(&mut buf, e).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let d0 = ImportDataNode {
            data_section_type: DataSectionType::ReadWrite,
            full_name: "foo::count".to_owned(),
            data_type: MemoryDataType::I32,
            alias_name: None,
            from: None,
        };

        assert_eq!(print(&d0), "import data foo::count type i32");

        let d1 = ImportDataNode {
            data_section_type: DataSectionType::Uninit,
            full_name: "foo::got".to_owned(),
            data_type: MemoryDataType::Bytes,
            alias_name: Some("global_offset_table".to_owned()),
            from: Some("mymod".to_owned()),
        };

        assert_eq!(
            print(&d1),
            "import uninit data foo::got type byte[] as global_offset_table from mymod"
        );
    }

    #[test]
    fn test_print_external_function_node() {
        let print = |e: &ExternalFunctionNode| {
            let mut buf: Vec<u8> = vec![];
            print_external_function_node(&mut buf, e).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let f0 = ExternalFunctionNode {
            full_name: "libfoo::bar".to_owned(),
            params: vec![],
            result: None,
            alias_name: None,
        };

        assert_eq!(print(&f0), "external fn libfoo::bar() -> ()");

        let f1 = ExternalFunctionNode {
            full_name: "libfoo::bar".to_owned(),
            params: vec![OperandDataType::I32, OperandDataType::I32],
            result: Some(OperandDataType::I64),
            alias_name: Some("baz".to_owned()),
        };

        assert_eq!(
            print(&f1),
            "external fn libfoo::bar(i32, i32) -> i64 as baz"
        );
    }

    #[test]
    fn test_print_external_data_node() {
        let print = |e: &ExternalDataNode| {
            let mut buf: Vec<u8> = vec![];
            print_external_data_node(&mut buf, e).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let d0 = ExternalDataNode {
            full_name: "libfoo::count".to_owned(),
            data_type: MemoryDataType::I32,
            alias_name: None,
        };

        assert_eq!(print(&d0), "external data libfoo::count type i32");

        let d1 = ExternalDataNode {
            full_name: "libfoo::got".to_owned(),
            data_type: MemoryDataType::Bytes,
            alias_name: Some("global_offset_table".to_owned()),
        };

        assert_eq!(
            print(&d1),
            "external data libfoo::got type byte[] as global_offset_table"
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
            export: false,
            name: "foo".to_owned(),
            data_section: DataSection::ReadOnly(DataTypeValuePair {
                data_type: DeclareDataType::I32,
                value: DataValue::I32(123),
            }),
        };

        assert_eq!(print(&node0), "readonly data foo:i32 = 123");

        // test byte array data type
        let node1 = DataNode {
            export: true,
            name: "foo".to_owned(),
            data_section: DataSection::ReadOnly(DataTypeValuePair {
                data_type: DeclareDataType::FixedBytes(32, None),
                value: DataValue::String("hello".to_owned()),
            }),
        };

        assert_eq!(print(&node1), "pub readonly data foo:byte[32] = \"hello\"");

        // test byte array data type with unspecific length
        let node2 = DataNode {
            export: true,
            name: "foo".to_owned(),
            data_section: DataSection::ReadWrite(DataTypeValuePair {
                data_type: DeclareDataType::Bytes(None),
                value: DataValue::String("world".to_owned()),
            }),
        };

        assert_eq!(print(&node2), "pub data foo:byte[] = \"world\"");

        // test uninit
        let node3 = DataNode {
            export: false,
            name: "got".to_owned(),
            data_section: DataSection::Uninit(FixedDeclareDataType::FixedBytes(1024, None)),
        };

        assert_eq!(print(&node3), "uninit data got:byte[1024]");

        // test byte array align
        let node4 = DataNode {
            export: false,
            name: "foo".to_owned(),
            data_section: DataSection::Uninit(FixedDeclareDataType::FixedBytes(1024, Some(8))),
        };

        assert_eq!(print(&node4), "uninit data foo:byte[1024, align=8]");

        // test hex byte data
        let node5 = DataNode {
            export: true,
            name: "foo".to_owned(),
            data_section: DataSection::ReadWrite(DataTypeValuePair {
                data_type: DeclareDataType::Bytes(None),
                value: DataValue::ByteData(vec![
                    0x11u8, 0x13, 0x17, 0x19, 0x23, 0x29, 0x31, 0x37, 0x41, 0x43, 0x47, 0x53, 0x59,
                    0x61, 0x67, 0x71, 0x73, 0x79,
                ]),
            }),
        };

        assert_eq!(
            print(&node5),
            "pub data foo:byte[] = h\"11 13 17 19  23 29 31 37
    41 43 47 53  59 61 67 71
    73 79\""
        );

        // test data value list
        let node6 = DataNode {
            export: false,
            name: "bar".to_owned(),
            data_section: DataSection::ReadWrite(DataTypeValuePair {
                data_type: DeclareDataType::Bytes(Some(4)),
                value: DataValue::List(vec![
                    DataValue::I8(11),
                    DataValue::I16(13),
                    DataValue::I32(17),
                    DataValue::I64(19),
                    DataValue::String("hello".to_owned()),
                    DataValue::ByteData(vec![0x11, 0x13, 0x17]),
                    DataValue::List(vec![DataValue::I8(211), DataValue::I8(223)]),
                ]),
            }),
        };

        assert_eq!(
            print(&node6),
            "\
data bar:byte[align=4] = [
    11_i8
    13_i16
    17
    19_i64
    \"hello\"
    h\"11 13 17\"
    [
        211_i8
        223_i8
    ]
]"
        );
    }

    #[test]
    fn test_print_function_node() {
        let print = |node: &FunctionNode| {
            let mut buf: Vec<u8> = vec![];
            print_function_node(&mut buf, node, DEFAULT_INDENT_CHARS).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let node0 = FunctionNode {
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::Instruction(InstructionNode {
                name: "local_load_i64".to_owned(),
                positional_args: vec![ArgumentValue::Identifier("left".to_owned())],
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
            })),
        };

        assert_eq!(
            print(&node0),
            "\
fn foo() -> ()
    local_load_i64(left, rindex=1_i16, offset=4_i16)"
        );

        // test params, results
        let node1 = FunctionNode {
            export: true,
            name: "add".to_owned(),
            params: vec![
                NamedParameter {
                    name: "left".to_owned(),
                    data_type: OperandDataType::I32,
                },
                NamedParameter {
                    name: "right".to_owned(),
                    data_type: OperandDataType::I32,
                },
            ],
            results: vec![OperandDataType::I32],
            locals: vec![],
            body: Box::new(ExpressionNode::Instruction(InstructionNode {
                name: "add_i32".to_owned(),
                positional_args: vec![
                    ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                        InstructionNode {
                            name: "local_load_i32".to_owned(),
                            positional_args: vec![ArgumentValue::Identifier("left".to_owned())],
                            named_args: vec![],
                        },
                    ))),
                    ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                        InstructionNode {
                            name: "add_imm_i32".to_owned(),
                            positional_args: vec![
                                ArgumentValue::LiteralNumber(LiteralNumber::I16(11)),
                                ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                                    InstructionNode {
                                        name: "local_load_i32".to_owned(),
                                        positional_args: vec![ArgumentValue::Identifier(
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
            })),
        };

        assert_eq!(
            print(&node1),
            "\
pub fn add(left:i32, right:i32) -> i32
    add_i32(
        local_load_i32(left),
        add_imm_i32(11_i16,
            local_load_i32(right)))"
        );

        // test multiple results and local variables
        let node2 = FunctionNode {
            export: false,
            name: "hello".to_owned(),
            params: vec![],
            results: vec![OperandDataType::I32, OperandDataType::I64],
            locals: vec![
                LocalVariable {
                    name: "foo".to_owned(),
                    data_type: FixedDeclareDataType::I32,
                },
                LocalVariable {
                    name: "bar".to_owned(),
                    data_type: FixedDeclareDataType::FixedBytes(8, None),
                },
                LocalVariable {
                    name: "baz".to_owned(),
                    data_type: FixedDeclareDataType::FixedBytes(24, Some(4)),
                },
            ],
            body: Box::new(ExpressionNode::Instruction(InstructionNode {
                name: "end".to_owned(),
                positional_args: vec![],
                named_args: vec![],
            })),
        };

        assert_eq!(
            print(&node2),
            "\
fn hello() -> (i32, i64)
    [foo:i32, bar:byte[8], baz:byte[24, align=4]]
    end()"
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
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::Group(vec![
                ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    positional_args: vec![],
                    named_args: vec![],
                }),
                ExpressionNode::Instruction(InstructionNode {
                    name: "local_store_i32".to_owned(),
                    positional_args: vec![
                        ArgumentValue::Identifier("left".to_owned()),
                        ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                            InstructionNode {
                                name: "imm_i32".to_owned(),
                                positional_args: vec![ArgumentValue::LiteralNumber(
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
                    positional_args: vec![
                        ArgumentValue::Identifier("right".to_owned()),
                        ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                            InstructionNode {
                                name: "add_i32".to_owned(),
                                positional_args: vec![
                                    ArgumentValue::Expression(Box::new(
                                        ExpressionNode::Instruction(InstructionNode {
                                            name: "imm_i32".to_owned(),
                                            positional_args: vec![ArgumentValue::LiteralNumber(
                                                LiteralNumber::I32(123),
                                            )],
                                            named_args: vec![],
                                        }),
                                    )),
                                    ArgumentValue::Expression(Box::new(
                                        ExpressionNode::Instruction(InstructionNode {
                                            name: "imm_i32".to_owned(),
                                            positional_args: vec![ArgumentValue::LiteralNumber(
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
            ])),
        };

        assert_eq!(
            print(&node0),
            "\
fn foo() -> ()
    {
        nop()
        local_store_i32(left,
            imm_i32(123))
        local_store_i32(right,
            add_i32(
                imm_i32(123),
                imm_i32(123)))
    }"
        );

        // test nested group
        let node1 = FunctionNode {
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::Group(vec![
                ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    positional_args: vec![],
                    named_args: vec![],
                }),
                ExpressionNode::Group(vec![ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    positional_args: vec![],
                    named_args: vec![],
                })]),
            ])),
        };

        assert_eq!(
            print(&node1),
            "\
fn foo() -> ()
    {
        nop()
        {
            nop()
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
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::When(WhenNode {
                testing: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "imm_i32".to_owned(),
                    positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(1))],
                    named_args: vec![],
                })),
                locals: vec![],
                consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    positional_args: vec![],
                    named_args: vec![],
                })),
            })),
        };

        assert_eq!(
            print(&node0),
            "\
fn foo() -> ()
    when
        imm_i32(1)
        nop()"
        );

        // test `when` with multiple layers instructions
        let node1 = FunctionNode {
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::When(WhenNode {
                testing: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "eqz_i32".to_owned(),
                    positional_args: vec![ArgumentValue::Expression(Box::new(
                        ExpressionNode::Instruction(InstructionNode {
                            name: "local_load_i32".to_owned(),
                            positional_args: vec![ArgumentValue::Identifier("a".to_owned())],
                            named_args: vec![],
                        }),
                    ))],
                    named_args: vec![],
                })),
                locals: vec![],
                consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "data_store_i32".to_owned(),
                    positional_args: vec![
                        ArgumentValue::Identifier("b".to_owned()),
                        ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                            InstructionNode {
                                name: "local_load_i32".to_owned(),
                                positional_args: vec![ArgumentValue::Identifier("a".to_owned())],
                                named_args: vec![],
                            },
                        ))),
                    ],
                    named_args: vec![],
                })),
            })),
        };

        assert_eq!(
            print(&node1),
            "\
fn foo() -> ()
    when
        eqz_i32(
            local_load_i32(a))
        data_store_i32(b,
            local_load_i32(a))"
        );

        // test `when` with local variables
        let node2 = FunctionNode {
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::When(WhenNode {
                testing: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "imm_i32".to_owned(),
                    positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(1))],
                    named_args: vec![],
                })),
                locals: vec![
                    LocalVariable {
                        name: "foo".to_owned(),
                        data_type: FixedDeclareDataType::I32,
                    },
                    LocalVariable {
                        name: "bar".to_owned(),
                        data_type: FixedDeclareDataType::FixedBytes(8, None),
                    },
                    LocalVariable {
                        name: "baz".to_owned(),
                        data_type: FixedDeclareDataType::FixedBytes(24, Some(4)),
                    },
                ],
                consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    positional_args: vec![],
                    named_args: vec![],
                })),
            })),
        };

        assert_eq!(
            print(&node2),
            "\
fn foo() -> ()
    when
        [foo:i32, bar:byte[8], baz:byte[24, align=4]]
        imm_i32(1)
        nop()"
        );

        // test 'when' with 'group'
        let node3 = FunctionNode {
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::When(WhenNode {
                testing: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "imm_i32".to_owned(),
                    positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(1))],
                    named_args: vec![],
                })),
                locals: vec![],
                consequence: Box::new(ExpressionNode::Group(vec![
                    ExpressionNode::Instruction(InstructionNode {
                        name: "nop".to_owned(),
                        positional_args: vec![],
                        named_args: vec![],
                    }),
                    ExpressionNode::Instruction(InstructionNode {
                        name: "local_store_i32".to_owned(),
                        positional_args: vec![
                            ArgumentValue::Identifier("left".to_owned()),
                            ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                                InstructionNode {
                                    name: "local_load_i32".to_owned(),
                                    positional_args: vec![ArgumentValue::Identifier(
                                        "right".to_owned(),
                                    )],
                                    named_args: vec![],
                                },
                            ))),
                        ],
                        named_args: vec![],
                    }),
                ])),
            })),
        };

        assert_eq!(
            print(&node3),
            "\
fn foo() -> ()
    when
        imm_i32(1)
        {
            nop()
            local_store_i32(left,
                local_load_i32(right))
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
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::If(IfNode {
                // params: vec![],
                results: vec![],
                testing: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "eqz_i32".to_owned(),
                    positional_args: vec![ArgumentValue::Expression(Box::new(
                        ExpressionNode::Instruction(InstructionNode {
                            name: "local_load_i32".to_owned(),
                            positional_args: vec![ArgumentValue::Identifier("in".to_owned())],
                            named_args: vec![],
                        }),
                    ))],
                    named_args: vec![],
                })),
                consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "local_store_i32".to_owned(),
                    positional_args: vec![
                        ArgumentValue::Identifier("out".to_owned()),
                        ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                            InstructionNode {
                                name: "imm_i32".to_owned(),
                                positional_args: vec![ArgumentValue::LiteralNumber(
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
                    positional_args: vec![
                        ArgumentValue::Identifier("out".to_owned()),
                        ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                            InstructionNode {
                                name: "imm_i32".to_owned(),
                                positional_args: vec![ArgumentValue::LiteralNumber(
                                    LiteralNumber::I32(13),
                                )],
                                named_args: vec![],
                            },
                        ))),
                    ],
                    named_args: vec![],
                })),
            })),
        };

        assert_eq!(
            print(&node0),
            "\
fn foo() -> ()
    if -> ()
        eqz_i32(
            local_load_i32(in))
        local_store_i32(out,
            imm_i32(11))
        local_store_i32(out,
            imm_i32(13))"
        );

        // test `if` with params and return value
        let node1 = FunctionNode {
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::If(IfNode {
                // params: vec![
                //     NamedParameter {
                //         name: "left".to_owned(),
                //         data_type: OperandDataType::I32,
                //     },
                //     NamedParameter {
                //         name: "right".to_owned(),
                //         data_type: OperandDataType::I32,
                //     },
                // ],
                results: vec![OperandDataType::I32],
                testing: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "imm_i32".to_owned(),
                    positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(11))],
                    named_args: vec![],
                })),
                consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "imm_i32".to_owned(),
                    positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(13))],
                    named_args: vec![],
                })),
                alternative: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "imm_i32".to_owned(),
                    positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(17))],
                    named_args: vec![],
                })),
            })),
        };

        assert_eq!(
            print(&node1),
            "\
fn foo() -> ()
    if -> i32
        imm_i32(11)
        imm_i32(13)
        imm_i32(17)"
        );

        // test `if` with multiple return multiple values
        let node2 = FunctionNode {
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::If(IfNode {
                // params: vec![],
                results: vec![OperandDataType::I32, OperandDataType::I64],
                testing: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "imm_i32".to_owned(),
                    positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(11))],
                    named_args: vec![],
                })),
                consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    positional_args: vec![],
                    named_args: vec![],
                })),
                alternative: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    positional_args: vec![],
                    named_args: vec![],
                })),
            })),
        };

        assert_eq!(
            print(&node2),
            "\
fn foo() -> ()
    if -> (i32, i64)
        imm_i32(11)
        nop()
        nop()"
        );
    }

    // #[test]
    // fn test_print_expression_branch() {
    //     let print = |node: &FunctionNode| {
    //         let mut buf: Vec<u8> = vec![];
    //         print_function_node(&mut buf, node, DEFAULT_INDENT_CHARS).unwrap();
    //         String::from_utf8(buf).unwrap()
    //     };
    //
    //     let node0 = FunctionNode {
    //         export: true,
    //         name: "foo".to_owned(),
    //         params: vec![],
    //         results: vec![],
    //         locals: vec![],
    //         body: Box::new(ExpressionNode::Branch(BranchNode {
    //             params: vec![
    //                 NamedParameter {
    //                     name: "left".to_owned(),
    //                     data_type: OperandDataType::I32,
    //                 },
    //                 NamedParameter {
    //                     name: "right".to_owned(),
    //                     data_type: OperandDataType::I32,
    //                 },
    //             ],
    //             results: vec![OperandDataType::I32],
    //             locals: vec![
    //                 LocalVariable {
    //                     name: "temp".to_owned(),
    //                     data_type: FixedDeclareDataType::I32,
    //                 },
    //                 LocalVariable {
    //                     name: "count".to_owned(),
    //                     data_type: FixedDeclareDataType::I32,
    //                 },
    //             ],
    //             cases: vec![
    //                 CaseNode {
    //                     testing: Box::new(ExpressionNode::Instruction(InstructionNode {
    //                         name: "imm_i32".to_owned(),
    //                         positional_args: vec![ArgumentValue::LiteralNumber(
    //                             LiteralNumber::I32(11),
    //                         )],
    //                         named_args: vec![],
    //                     })),
    //
    //                     consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
    //                         name: "imm_i32".to_owned(),
    //                         positional_args: vec![ArgumentValue::LiteralNumber(
    //                             LiteralNumber::I32(13),
    //                         )],
    //                         named_args: vec![],
    //                     })),
    //                 },
    //                 CaseNode {
    //                     testing: Box::new(ExpressionNode::Instruction(InstructionNode {
    //                         name: "imm_i32".to_owned(),
    //                         positional_args: vec![ArgumentValue::LiteralNumber(
    //                             LiteralNumber::I32(17),
    //                         )],
    //                         named_args: vec![],
    //                     })),
    //
    //                     consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
    //                         name: "imm_i32".to_owned(),
    //                         positional_args: vec![ArgumentValue::LiteralNumber(
    //                             LiteralNumber::I32(19),
    //                         )],
    //                         named_args: vec![],
    //                     })),
    //                 },
    //             ],
    //             default: Box::new(ExpressionNode::Instruction(InstructionNode {
    //                 name: "imm_i32".to_owned(),
    //                 positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(42))],
    //                 named_args: vec![],
    //             })),
    //         })),
    //     };
    //
    //     assert_eq!(
    //         print(&node0),
    //         "\
    // pub fn foo() -> ()
    // branch (left:i32, right:i32) -> i32
    //     [temp:i32, count:i32]
    //     {
    //     case
    //         imm_i32(11)
    //         imm_i32(13)
    //     case
    //         imm_i32(17)
    //         imm_i32(19)
    //     default
    //         imm_i32(42)
    //     }"
    //     );
    //
    //     // test no locals
    //     let node0 = FunctionNode {
    //         export: true,
    //         name: "foo".to_owned(),
    //         params: vec![],
    //         results: vec![],
    //         locals: vec![],
    //         body: Box::new(ExpressionNode::Branch(BranchNode {
    //             params: vec![
    //             ],
    //             results: vec![],
    //             locals: vec![
    //             ],
    //             cases: vec![
    //                 CaseNode {
    //                     testing: Box::new(ExpressionNode::Instruction(InstructionNode {
    //                         name: "imm_i32".to_owned(),
    //                         positional_args: vec![ArgumentValue::LiteralNumber(
    //                             LiteralNumber::I32(11),
    //                         )],
    //                         named_args: vec![],
    //                     })),
    //
    //                     consequence: Box::new(ExpressionNode::Instruction(InstructionNode {
    //                         name: "imm_i32".to_owned(),
    //                         positional_args: vec![ArgumentValue::LiteralNumber(
    //                             LiteralNumber::I32(13),
    //                         )],
    //                         named_args: vec![],
    //                     })),
    //                 },
    //             ],
    //             default: Box::new(ExpressionNode::Instruction(InstructionNode {
    //                 name: "imm_i32".to_owned(),
    //                 positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(42))],
    //                 named_args: vec![],
    //             })),
    //         })),
    //     };
    //
    //     assert_eq!(
    //         print(&node0),
    //         "\
    // pub fn foo() -> ()
    // branch () -> ()
    //     {
    //     case
    //         imm_i32(11)
    //         imm_i32(13)
    //     default
    //         imm_i32(42)
    //     }"
    //     );
    // }

    #[test]
    fn test_print_expression_block() {
        let print = |node: &FunctionNode| {
            let mut buf: Vec<u8> = vec![];
            print_function_node(&mut buf, node, DEFAULT_INDENT_CHARS).unwrap();
            String::from_utf8(buf).unwrap()
        };

        let node0 = FunctionNode {
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::Block(BlockNode {
                param_values: vec![],
                results: vec![],
                locals: vec![],
                body: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "local_store_i32".to_owned(),
                    positional_args: vec![
                        ArgumentValue::Identifier("out".to_owned()),
                        ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                            InstructionNode {
                                name: "imm_i32".to_owned(),
                                positional_args: vec![ArgumentValue::LiteralNumber(
                                    LiteralNumber::I32(11),
                                )],
                                named_args: vec![],
                            },
                        ))),
                    ],
                    named_args: vec![],
                })),
            })),
        };

        assert_eq!(
            print(&node0),
            "\
fn foo() -> ()
    block () -> ()
        local_store_i32(out,
            imm_i32(11))"
        );

        // test 'block' with params, results and local variablers
        let node1 = FunctionNode {
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::Block(BlockNode {
                param_values: vec![
                    NameValueParameter {
                        name: "left".to_owned(),
                        data_type: OperandDataType::I32,
                        value: Box::new(ExpressionNode::Instruction(InstructionNode {
                            name: "imm_i32".to_owned(),
                            positional_args: vec![ArgumentValue::LiteralNumber(
                                LiteralNumber::I32(11),
                            )],
                            named_args: vec![],
                        })),
                    },
                    NameValueParameter {
                        name: "right".to_owned(),
                        data_type: OperandDataType::I32,
                        value: Box::new(ExpressionNode::Instruction(InstructionNode {
                            name: "imm_i32".to_owned(),
                            positional_args: vec![ArgumentValue::LiteralNumber(
                                LiteralNumber::I32(13),
                            )],
                            named_args: vec![],
                        })),
                    },
                ],
                results: vec![OperandDataType::I32],
                locals: vec![
                    LocalVariable {
                        name: "abc".to_owned(),
                        data_type: FixedDeclareDataType::I32,
                    },
                    LocalVariable {
                        name: "def".to_owned(),
                        data_type: FixedDeclareDataType::FixedBytes(32, None),
                    },
                ],
                body: Box::new(ExpressionNode::Instruction(InstructionNode {
                    name: "nop".to_owned(),
                    positional_args: vec![],
                    named_args: vec![],
                })),
            })),
        };

        assert_eq!(
            print(&node1),
            "\
fn foo() -> ()
    block (left:i32=imm_i32(11), right:i32=imm_i32(13)) -> i32
        [abc:i32, def:byte[32]]
        nop()"
        );

        // test 'block' with 'group' and complex param value
        let node2 = FunctionNode {
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::Block(BlockNode {
                param_values: vec![NameValueParameter {
                    name: "num".to_owned(),
                    data_type: OperandDataType::I32,
                    value: Box::new(ExpressionNode::Instruction(InstructionNode {
                        name: "data_load_extend_i32_s".to_owned(),
                        positional_args: vec![
                            ArgumentValue::Identifier("buf".to_owned()),
                            ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                                InstructionNode {
                                    name: "local_load_i32_s".to_owned(),
                                    positional_args: vec![ArgumentValue::Identifier(
                                        "off".to_owned(),
                                    )],
                                    named_args: vec![],
                                },
                            ))),
                        ],
                        named_args: vec![],
                    })),
                }],
                results: vec![],
                locals: vec![LocalVariable {
                    name: "temp".to_owned(),
                    data_type: FixedDeclareDataType::I32,
                }],
                body: Box::new(ExpressionNode::Group(vec![
                    ExpressionNode::Instruction(InstructionNode {
                        name: "local_store_i32".to_owned(),
                        positional_args: vec![
                            ArgumentValue::Identifier("abc".to_owned()),
                            ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                                InstructionNode {
                                    name: "imm_i32".to_owned(),
                                    positional_args: vec![ArgumentValue::LiteralNumber(
                                        LiteralNumber::I32(11),
                                    )],
                                    named_args: vec![],
                                },
                            ))),
                        ],
                        named_args: vec![],
                    }),
                    ExpressionNode::Instruction(InstructionNode {
                        name: "local_store_i32".to_owned(),
                        positional_args: vec![
                            ArgumentValue::Identifier("def".to_owned()),
                            ArgumentValue::Expression(Box::new(ExpressionNode::Instruction(
                                InstructionNode {
                                    name: "imm_i32".to_owned(),
                                    positional_args: vec![ArgumentValue::LiteralNumber(
                                        LiteralNumber::I32(31),
                                    )],
                                    named_args: vec![],
                                },
                            ))),
                        ],
                        named_args: vec![],
                    }),
                ])),
            })),
        };

        assert_eq!(
            print(&node2),
            "\
fn foo() -> ()
    block (num:i32=data_load_extend_i32_s(buf,
        local_load_i32_s(off))) -> ()
        [temp:i32]
        {
            local_store_i32(abc,
                imm_i32(11))
            local_store_i32(def,
                imm_i32(31))
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
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::Group(vec![
                ExpressionNode::Break(BreakNode::Break(vec![
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(11))],
                        named_args: vec![],
                    }),
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(13))],
                        named_args: vec![],
                    }),
                ])),
                ExpressionNode::Break(BreakNode::BreakFn(vec![
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(23))],
                        named_args: vec![],
                    }),
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(29))],
                        named_args: vec![],
                    }),
                ])),
            ])),
        };

        assert_eq!(
            print(&node0),
            "\
fn foo() -> ()
    {
        break(
            imm_i32(11)
            imm_i32(13)
        )
        break_fn(
            imm_i32(23)
            imm_i32(29)
        )
    }"
        );

        // test 'recur'
        let node1 = FunctionNode {
            export: false,
            name: "foo".to_owned(),
            params: vec![],
            results: vec![],
            locals: vec![],
            body: Box::new(ExpressionNode::Group(vec![
                ExpressionNode::Recur(BreakNode::Break(vec![
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(11))],
                        named_args: vec![],
                    }),
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(13))],
                        named_args: vec![],
                    }),
                ])),
                ExpressionNode::Recur(BreakNode::BreakFn(vec![
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(23))],
                        named_args: vec![],
                    }),
                    ExpressionNode::Instruction(InstructionNode {
                        name: "imm_i32".to_owned(),
                        positional_args: vec![ArgumentValue::LiteralNumber(LiteralNumber::I32(29))],
                        named_args: vec![],
                    }),
                ])),
            ])),
        };

        assert_eq!(
            print(&node1),
            "\
fn foo() -> ()
    {
        recur(
            imm_i32(11)
            imm_i32(13)
        )
        recur_fn(
            imm_i32(23)
            imm_i32(29)
        )
    }"
        );
    }

    #[test]
    fn test_print_module_node() {
        let node = ModuleNode {
            // full_name: "foo".to_owned(),
            // uses: vec![
            //     UseNode {
            //         full_name: "foo::bar".to_owned(),
            //         alias_name: None,
            //     },
            //     UseNode {
            //         full_name: "foo::bar::baz".to_owned(),
            //         alias_name: Some("Baz".to_owned()),
            //     },
            // ],
            imports: vec![
                ImportNode::Function(ImportFunctionNode {
                    full_name: "std::abc".to_owned(),
                    params: vec![OperandDataType::I32, OperandDataType::I64],
                    results: vec![OperandDataType::I64],
                    alias_name: None,
                    from: None,
                }),
                ImportNode::Data(ImportDataNode {
                    data_section_type: DataSectionType::ReadOnly,
                    full_name: "std::def".to_owned(),
                    data_type: MemoryDataType::I32,
                    alias_name: Some("xyz".to_owned()),
                    from: Some("mymod".to_owned()),
                }),
            ],
            externals: vec![
                ExternalNode::Function(ExternalFunctionNode {
                    full_name: "liba::abc".to_owned(),
                    params: vec![OperandDataType::I32, OperandDataType::I64],
                    result: Some(OperandDataType::I64),
                    alias_name: None,
                }),
                ExternalNode::Data(ExternalDataNode {
                    full_name: "libb::def".to_owned(),
                    data_type: MemoryDataType::I32,
                    alias_name: Some("xyz".to_owned()),
                }),
            ],
            datas: vec![
                DataNode {
                    export: false,
                    name: "count".to_owned(),
                    data_section: DataSection::ReadWrite(DataTypeValuePair {
                        data_type: DeclareDataType::I32,
                        value: DataValue::I32(37),
                    }),
                },
                DataNode {
                    export: true,
                    name: "plt".to_owned(),
                    data_section: DataSection::ReadOnly(DataTypeValuePair {
                        data_type: DeclareDataType::FixedBytes(128, Some(8)),
                        value: DataValue::List(vec![
                            DataValue::I8(41),
                            DataValue::I8(43),
                            DataValue::I8(47),
                            DataValue::I8(53),
                        ]),
                    }),
                },
            ],
            functions: vec![
                FunctionNode {
                    export: false,
                    name: "add".to_owned(),
                    params: vec![
                        NamedParameter {
                            name: "left".to_owned(),
                            data_type: OperandDataType::I32,
                        },
                        NamedParameter {
                            name: "right".to_owned(),
                            data_type: OperandDataType::I32,
                        },
                    ],
                    results: vec![OperandDataType::I32],
                    locals: vec![],
                    body: Box::new(ExpressionNode::Instruction(InstructionNode {
                        name: "nop".to_owned(),
                        positional_args: vec![],
                        named_args: vec![],
                    })),
                },
                FunctionNode {
                    export: true,
                    name: "entry".to_owned(),
                    params: vec![],
                    results: vec![OperandDataType::I32],
                    locals: vec![LocalVariable {
                        name: "temp".to_owned(),
                        data_type: FixedDeclareDataType::I32,
                    }],
                    body: Box::new(ExpressionNode::Instruction(InstructionNode {
                        name: "nop".to_owned(),
                        positional_args: vec![],
                        named_args: vec![],
                    })),
                },
            ],
        };

        assert_eq!(
            print_to_string(&node),
            "\
import fn std::abc(i32, i64) -> i64
import readonly data std::def type i32 as xyz from mymod

external fn liba::abc(i32, i64) -> i64
external data libb::def type i32 as xyz

data count:i32 = 37
pub readonly data plt:byte[128, align=8] = [
    41_i8
    43_i8
    47_i8
    53_i8
]

fn add(left:i32, right:i32) -> i32
    nop()

pub fn entry() -> i32
    [temp:i32]
    nop()
"
        )
    }
}
