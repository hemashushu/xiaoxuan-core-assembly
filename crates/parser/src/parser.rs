// Copyright (c) 2023 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

// the fundanmental of XiaoXuan Core VM Assembly s-expression
// ----------------------------------------------------------
//
// 1. the assembly text is in s-expression format, and the text consists of
//    one or more nodes.
// 2. each node consists of a pair of parentheses, a node name and one or more
//    arguments. e.g.
//
//    `(node_name param0 param1 param2)`
//
// 3. the parameter values can be symbols, identifiers, numbers and strings.
//    parameter values can also be nodes, so the assembly text looks like a
//    tree structure. e.g.
//
//    ```clojure
//    (function $name (param $lhs i32) (param $rhs i32) (result i32)
//        (code
//            (i32.add
//                (local.load32_i32 $lhs) (local.load32_i32 $rhs)
//            )
//        )
//    )
//    ```
//
// 4. the parameters have a fixed order and the positions of the parameters
//    cannot be changed.
//
//    `(local $sum i32)` =/= `(local i32 $sum)`
//
// 5. some of the parameters can be omitted, in this case, the parameters must
//    still be in their original order. e.g.
//
//   `(local.load32_i32 $db (offset 0))` == `(local.load32_i32 $db)`
//   ;; the child node '(offset ...)' above can be omitted.

// the instruction syntax
// ----------------------
//
// `(instruction_name param_0 ... param_N operand_0 ... operand_N)`
//
// 1. instructions with NO parameters and NO operands, can be written
//    with or without parentheses, e.g.
//    '(nop)'
//    'nop'
//
// 2. instructions that have NO parameters but HAVE operands, should be
//    written with parentheses and all the operands (instructions) should be
//    written inside the parentheses, e.g.
//    '(i32.add zero zero)'
//
// 3. instructions WITH parameters must be written with parentheses, e.g.
//    '(i32.imm 0x1133)'
//    '(local.load64_i64 $abc)'
//    '(local.load64_i64 $abc 8)  ;; 8 is an optional parameter'
//
// 4. instructions that HAVE BOTH parameters and operands must be written
//    with parentheses, and the operands must be written after the parameters, e.g.
//    '(local.store $xyz (i32.imm 11))'
//
//    ```
//    (local.store $xyz
//        (i32.add
//            (i32.imm 11) (i32.imm 13)
//        )
//    )
//    ```

use ancvm_types::{
    opcode::Opcode, DataSectionType, DataType, ExternalLibraryType, MemoryDataType, ModuleShareType,
};

use crate::{
    ast::{
        BranchCase, DataKindNode, DataNode, ExternalFunctionNode, ExternalItem,
        ExternalLibraryNode, ExternalNode, FunctionNode, ImportDataNode, ImportFunctionNode,
        ImportItem, ImportModuleNode, ImportNode, InitedData, Instruction, LocalNode,
        ModuleElementNode, ModuleNode, ParamNode, UninitData,
    },
    core_assembly_instruction::{init_instruction_map, InstructionSyntaxKind, INSTRUCTION_MAP},
    lexer::{NumberToken, Token},
    peekable_iterator::PeekableIterator,
    ParseError, NAME_PATH_SEPARATOR,
};

pub fn parse(iter: &mut PeekableIterator<Token>) -> Result<ModuleNode, ParseError> {
    // initialize the instruction kind table
    init_instruction_map();

    // there is only one node 'module' in a assembly text
    parse_module_node(iter)
}

pub fn parse_module_node(iter: &mut PeekableIterator<Token>) -> Result<ModuleNode, ParseError> {
    // (module ...) ...  //
    // ^            ^____// to here
    // |_________________// current token, i.e. the value of 'iter.peek(0)'

    // the node 'module' syntax:
    //
    // (module $name (runtime_version "1.0") ...)   ;; base
    // (module $name (runtime_version "1.0")
    //                                              ;; optional parameters
    //      (constructor $function_name_path)       ;; similar to GCC '__attribute__((constructor))', run before main()
    //      (destructor $function_name_path)        ;; similar to GCC '__attribute__((destructor))', run after main()
    //      ...
    // )

    consume_left_paren(iter, "module")?;
    consume_symbol(iter, "module")?;

    let name_path = expect_identifier(iter, "module.name")?;
    let (runtime_version_major, runtime_version_minor) = parse_module_runtime_version_node(iter)?;

    // optional parameters
    let is_sub_module = name_path.contains(NAME_PATH_SEPARATOR);
    let constructor_function_name_path = if exist_child_node(iter, "constructor") {
        if is_sub_module {
            return Err(ParseError::new(&format!(
                "Only the main module can define the constructor function, current submodule: {}",
                name_path
            )));
        }

        consume_left_paren(iter, "module.constructor")?;
        consume_symbol(iter, "constructor")?;
        let name_path = expect_identifier(iter, "constructor")?;
        consume_right_paren(iter)?;

        Some(name_path)
    } else {
        None
    };

    let destructor_function_name_path = if exist_child_node(iter, "destructor") {
        if is_sub_module {
            return Err(ParseError::new(&format!(
                "Only the main module can define the constructor function, current submodule: {}",
                name_path
            )));
        }

        consume_left_paren(iter, "module.destructor")?;
        consume_symbol(iter, "destructor")?;
        let name_path = expect_identifier(iter, "destructor")?;
        consume_right_paren(iter)?;

        Some(name_path)
    } else {
        None
    };

    let mut element_nodes: Vec<ModuleElementNode> = vec![];

    // parse module elements
    while iter.look_ahead_equals(0, &Token::LeftParen) {
        if let Some(Token::Symbol(child_node_name)) = iter.peek(1) {
            let element_node = match child_node_name.as_str() {
                "function" => parse_function_node(iter)?,
                "data" => parse_data_node(iter)?,
                "external" => parse_external_node(iter)?,
                "import" => parse_import_node(iter)?,
                _ => {
                    return Err(ParseError::new(&format!(
                        "Unknown module element: {}",
                        child_node_name
                    )))
                }
            };
            element_nodes.push(element_node);
        } else {
            break;
        }
    }

    consume_right_paren(iter)?;

    let module_node = ModuleNode {
        name_path,
        runtime_version_major,
        runtime_version_minor,
        constructor_function_name_path,
        destructor_function_name_path,
        element_nodes,
    };

    Ok(module_node)
}

fn parse_module_runtime_version_node(
    iter: &mut PeekableIterator<Token>,
) -> Result<(u16, u16), ParseError> {
    // (runtime_version "1.0") ...  //
    // ^                       ^____// to here
    // |____________________________// current token

    consume_left_paren(iter, "module.runtime_version")?;
    consume_symbol(iter, "runtime_version")?;
    let ver_string = expect_string(iter, "module.runtime_version")?;
    consume_right_paren(iter)?;

    parse_version(&ver_string)
}

fn parse_version(ver_string: &str) -> Result<(u16, u16), ParseError> {
    let ver_parts: Vec<&str> = ver_string.split('.').collect();
    if ver_parts.len() != 2 {
        return Err(ParseError::new(&format!(
            "Error version number, expect: \"major.minor\", actual: \"{}\"",
            ver_string
        )));
    }

    let major = ver_parts[0].parse::<u16>().map_err(|_| {
        ParseError::new(&format!(
            "Major version '{}' is not a valid number.",
            ver_parts[0]
        ))
    })?;

    let minor = ver_parts[1].parse::<u16>().map_err(|_| {
        ParseError::new(&format!(
            "Minor version '{}' is not a valid number.",
            ver_parts[1]
        ))
    })?;

    Ok((major, minor))
}

fn parse_function_node(
    iter: &mut PeekableIterator<Token>,
) -> Result<ModuleElementNode, ParseError> {
    // (function ...) ...  //
    // ^              ^____// to here
    // |___________________// current token

    // the node 'function' syntax:
    //
    // (function $name (param $param_0 DATA_TYPE) ...
    //           (result DATA_TYPE) ...
    //           (local $local_variable_name LOCAL_DATA_TYPE) ...
    //           (code ...)
    //)

    // e.g.
    //
    // (function $add (param $lhs i32) (param $rhs i32) (result i32) ...)     ;; signature
    // (function $add (param $lhs i32) (result i32) (result i32) ...)         ;; signature with multiple return values
    // (function $add (param $lhs i32) (results i32 i32) ...)                 ;; signature with multiple return values
    // (function $add
    //     (local $sum i32)             ;; local variable with identifier and data type
    //     (local $db (bytes 12 4))     ;; bytes-type local variable
    //     ...
    // )
    //
    // (function $add
    //     (code ...)                   ;; the function body, the instructions sequence, sholud be written inside the node '(code)'
    // )

    // function with 'export' annotation
    // (function export $add ...)

    consume_left_paren(iter, "function")?;
    consume_symbol(iter, "function")?;
    let export = expect_specified_symbol_optional(iter, "export");
    let name = expect_identifier(iter, "function")?;
    let (params, results) = parse_optional_signature(iter)?;
    let locals: Vec<LocalNode> = parse_optional_local_variables(iter)?;
    let code = parse_code_node(iter)?;
    consume_right_paren(iter)?;

    if name.contains(NAME_PATH_SEPARATOR) {
        return Err(ParseError {
            message: format!(
                "The name of function can not contains path separator, name: \"{}\"",
                name
            ),
        });
    }

    // function's code implies an instruction 'end' at the end.
    // instructions.push(Instruction::NoParams(Opcode::end));

    let function_node = FunctionNode {
        name,
        export,
        params,
        results,
        locals,
        code,
    };

    Ok(ModuleElementNode::FunctionNode(function_node))
}

fn parse_optional_signature(
    iter: &mut PeekableIterator<Token>,
) -> Result<(Vec<ParamNode>, Vec<DataType>), ParseError> {
    // (param|result|results ...){0,} ...  //
    // ^                              ^____// to here
    // |___________________________________// current token

    let mut params: Vec<ParamNode> = vec![];
    let mut results: Vec<DataType> = vec![];

    while iter.look_ahead_equals(0, &Token::LeftParen) {
        if let Some(Token::Symbol(child_node_name)) = iter.peek(1) {
            match child_node_name.as_str() {
                "param" => {
                    let param_node = parse_param_node(iter)?;
                    params.push(param_node);
                }
                "result" => {
                    let data_type = parse_result_node(iter)?;
                    results.push(data_type);
                }
                "results" => {
                    let mut data_types = parse_results_node(iter)?;
                    results.append(&mut data_types);
                }
                _ => break,
            }
        } else {
            break;
        }
    }

    Ok((params, results))
}

fn parse_optional_signature_results_only(
    iter: &mut PeekableIterator<Token>,
) -> Result<Vec<DataType>, ParseError> {
    // (result|results ...){0,} ...  //
    // ^                        ^____// to here
    // |_____________________________// current token

    let mut results: Vec<DataType> = vec![];

    while iter.look_ahead_equals(0, &Token::LeftParen) {
        if let Some(Token::Symbol(child_node_name)) = iter.peek(1) {
            match child_node_name.as_str() {
                "result" => {
                    let data_type = parse_result_node(iter)?;
                    results.push(data_type);
                }
                "results" => {
                    let mut data_types = parse_results_node(iter)?;
                    results.append(&mut data_types);
                }
                _ => break,
            }
        } else {
            break;
        }
    }

    Ok(results)
}

fn parse_param_node(iter: &mut PeekableIterator<Token>) -> Result<ParamNode, ParseError> {
    // (param $name i32) ...  //
    // ^                 ^____// to here
    // |______________________// current token

    consume_left_paren(iter, "param")?;
    consume_symbol(iter, "param")?;
    let name = expect_identifier(iter, "param")?;
    let data_type = parse_data_type(iter)?;
    consume_right_paren(iter)?;

    Ok(ParamNode { name, data_type })
}

fn parse_result_node(iter: &mut PeekableIterator<Token>) -> Result<DataType, ParseError> {
    // (result i32) ...  //
    // ^            ^____// to here
    // |_________________// current token

    consume_left_paren(iter, "result")?;
    consume_symbol(iter, "result")?;
    let data_type = parse_data_type(iter)?;
    consume_right_paren(iter)?;

    Ok(data_type)
}

fn parse_results_node(iter: &mut PeekableIterator<Token>) -> Result<Vec<DataType>, ParseError> {
    // (results i32 i32 i64) ...  //
    // ^                     ^____// to here
    // |__________________________// current token

    let mut data_types: Vec<DataType> = vec![];

    consume_left_paren(iter, "results")?;
    consume_symbol(iter, "results")?;
    while matches!(iter.peek(0), &Some(Token::Symbol(_))) {
        let data_type = parse_data_type(iter)?;
        data_types.push(data_type);
    }

    consume_right_paren(iter)?;

    Ok(data_types)
}

fn parse_data_type(iter: &mut PeekableIterator<Token>) -> Result<DataType, ParseError> {
    // i32 ...  //
    // i64 ...  //
    // f32 ...  //
    // f64 ...  //
    // ^   ^____// to here
    // |________// current token

    let data_type_name = expect_symbol(iter, "data.type")?;
    let data_type = match data_type_name.as_str() {
        "i32" => DataType::I32,
        "i64" => DataType::I64,
        "f32" => DataType::F32,
        "f64" => DataType::F64,
        _ => {
            return Err(ParseError::new(&format!(
                "Unknown data type: {}",
                data_type_name
            )))
        }
    };
    Ok(data_type)
}

fn parse_optional_local_variables(
    iter: &mut PeekableIterator<Token>,
) -> Result<Vec<LocalNode>, ParseError> {
    // (local $name i32){0,} ...  //
    // ^                     ^____// to here
    // |__________________________// current token

    let mut local_nodes: Vec<LocalNode> = vec![];

    while iter.look_ahead_equals(0, &Token::LeftParen) {
        if let Some(Token::Symbol(child_node_name)) = iter.peek(1) {
            match child_node_name.as_str() {
                "local" => {
                    let local_node = parse_local_node(iter)?;
                    local_nodes.push(local_node);
                }
                _ => break,
            }
        } else {
            break;
        }
    }

    Ok(local_nodes)
}

fn parse_local_node(iter: &mut PeekableIterator<Token>) -> Result<LocalNode, ParseError> {
    // (local $name i32) ...  //
    // ^                 ^____// to here
    // |______________________// current token

    // also:
    // (local $name (bytes DATA_LENGTH:i32 ALIGN:i16))

    consume_left_paren(iter, "local")?;
    consume_symbol(iter, "local")?;
    let name = expect_identifier(iter, "local.name")?;
    let (memory_data_type, data_length, align) =
        parse_memory_data_type_with_length_and_align(iter)?;

    consume_right_paren(iter)?;

    if name.contains(NAME_PATH_SEPARATOR) {
        return Err(ParseError {
            message: format!(
                "The name of local variable can not contains path separator, name: \"{}\"",
                name
            ),
        });
    }

    Ok(LocalNode {
        name,
        memory_data_type,
        data_length,
        align,
    })
}

// return '(MemoryDataType, data length, align)'
fn parse_memory_data_type_with_length_and_align(
    iter: &mut PeekableIterator<Token>,
) -> Result<(MemoryDataType, u32, u16), ParseError> {
    // i32 ...  //
    // ^   ^____// to here
    // |________// current token

    // also:
    // i64
    // f32
    // f64
    // (bytes DATA_LENGTH:i32 ALIGN:i16)

    if iter.look_ahead_equals(0, &Token::LeftParen) {
        parse_memory_data_type_bytes_with_length_and_align(iter)
    } else {
        parse_memory_data_type_primitive_with_length_and_align(iter)
    }
}

// return '(MemoryDataType, data length, align)'
fn parse_memory_data_type_primitive_with_length_and_align(
    iter: &mut PeekableIterator<Token>,
) -> Result<(MemoryDataType, u32, u16), ParseError> {
    // i32 ...  //
    // i64 ...  //
    // f32 ...  //
    // f64 ...  //
    // ^   ^____// to here
    // |________// current token

    let memory_data_type_name = expect_symbol(iter, "data.type")?;
    let memory_data_type_detail = match memory_data_type_name.as_str() {
        "i32" => (MemoryDataType::I32, 4, 4),
        "i64" => (MemoryDataType::I64, 8, 8),
        "f32" => (MemoryDataType::F32, 4, 4),
        "f64" => (MemoryDataType::F64, 8, 8),
        _ => {
            return Err(ParseError::new(&format!(
                "Unknown data node memory data type: {}",
                memory_data_type_name
            )))
        }
    };

    Ok(memory_data_type_detail)
}

// return '(MemoryDataType, data length, align)'
fn parse_memory_data_type_bytes_with_length_and_align(
    iter: &mut PeekableIterator<Token>,
) -> Result<(MemoryDataType, u32, u16), ParseError> {
    // (bytes DATA_LENGTH:i32 ALIGN:i16)) ...  //
    // ^                                  ^____// to here
    // |_______________________________________// current token

    consume_left_paren(iter, "data.type.bytes")?;
    consume_symbol(iter, "bytes")?;

    let length_number_token = expect_number(iter, "data.type.bytes.length")?;
    let align_number_token = expect_number(iter, "data.type.bytes.align")?;

    let length = parse_u32_string(&length_number_token).map_err(|_| {
        ParseError::new(&format!(
            "The length of memory data type bytes '{:?}' is not a valid number.",
            length_number_token
        ))
    })?;

    let align = parse_u16_string(&align_number_token).map_err(|_| {
        ParseError::new(&format!(
            "The align of memory data type bytes '{:?}' is not a valid number.",
            align_number_token
        ))
    })?;

    if align == 0 || align > 8 {
        return Err(ParseError::new(&format!(
            "The range of align of memory data type bytes should be 1 to 8, actual: '{}'.",
            align
        )));
    }

    consume_right_paren(iter)?;

    Ok((MemoryDataType::Bytes, length, align))
}

fn parse_memory_data_type(memory_data_type_str: &str) -> Result<MemoryDataType, ParseError> {
    // i32   ...  //
    // i64   ...  //
    // f32   ...  //
    // f64   ...  //
    // bytes ...  //
    // ^     ^____// to here
    // |__________// current token

    let memory_data_type = match memory_data_type_str {
        "i32" => MemoryDataType::I32,
        "i64" => MemoryDataType::I64,
        "f32" => MemoryDataType::F32,
        "f64" => MemoryDataType::F64,
        "bytes" => MemoryDataType::Bytes,
        _ => {
            return Err(ParseError::new(&format!(
                "Unknown imported data memory data type: {}",
                memory_data_type_str
            )))
        }
    };

    Ok(memory_data_type)
}

fn parse_code_node(iter: &mut PeekableIterator<Token>) -> Result<Vec<Instruction>, ParseError> {
    // (code ...) ...  //
    // ^          ^____// to here
    // |_______________// current token

    consume_left_paren(iter, "code")?;
    consume_symbol(iter, "code")?;
    let mut instructions = vec![];

    while let Some(instruction) = parse_next_instruction_optional(iter)? {
        instructions.push(instruction);
    }

    consume_right_paren(iter)?;

    Ok(instructions)
}

fn parse_instruction_sequence_node(
    iter: &mut PeekableIterator<Token>,
    node_name: &str,
) -> Result<Instruction, ParseError> {
    // (do ...) ...  //
    // ^        ^____// to here
    // |_____________// current token

    // other sequence nodes:
    //
    // - (break ...)
    // - (recur ...)
    // - (return ...)
    // - (rerun ...)

    consume_left_paren(iter, &format!("instruction.{}", node_name))?;
    consume_symbol(iter, node_name)?;
    let mut instructions = vec![];

    while let Some(instruction) = parse_next_instruction_optional(iter)? {
        instructions.push(instruction);
    }

    consume_right_paren(iter)?;

    let instruction = match node_name {
        "do" => Instruction::Do(instructions),
        "break" => Instruction::Break(instructions),
        "recur" => Instruction::Recur(instructions),
        "return" => Instruction::Return(instructions),
        "rerun" => Instruction::Rerun(instructions),
        _ => unreachable!(),
    };
    Ok(instruction)
}

fn parse_next_instruction_optional(
    iter: &mut PeekableIterator<Token>,
) -> Result<Option<Instruction>, ParseError> {
    let instruction = if let Some(token) = iter.peek(0) {
        match token {
            Token::LeftParen => {
                // parse instruction WITH parentheses
                parse_instruction_with_parentheses(iter)?
            }
            Token::Symbol(_) => {
                // parse instruction WITHOUT parentheses
                parse_instruction_without_parentheses(iter)?
            }
            _ => return Ok(None),
        }
    } else {
        return Ok(None);
    };

    Ok(Some(instruction))
}

fn parse_next_operand(
    iter: &mut PeekableIterator<Token>,
    for_what: &str,
) -> Result<Instruction, ParseError> {
    let instruction = if let Some(token) = iter.peek(0) {
        match token {
            Token::LeftParen => {
                // parse instruction WITH parentheses
                parse_instruction_with_parentheses(iter)?
            }
            Token::Symbol(_) => {
                // parse instruction WITHOUT parentheses
                parse_instruction_without_parentheses(iter)?
            }
            _ => {
                return Err(ParseError::new(&format!(
                    "Expect operand for \"{}\", actual {:?}",
                    for_what, token
                )))
            }
        }
    } else {
        return Err(ParseError::new(&format!(
            "Missing operand for \"{}\"",
            for_what
        )));
    };

    Ok(instruction)
}

// parse the instruction with parentheses,
//
// ✖️: i32.add
// ✅: (i32.add ...)
//
fn parse_instruction_with_parentheses(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (inst_name) ...  //
    // ^           ^____// to here
    // |________________// current token
    //
    // also:
    //
    // (inst_name PARAM_0 PARAM_1 ... PARAM_N)
    // (inst_name OPERAND_0 OPERAND_1 ... OPERAND_N)
    // (inst_name PARAM_0 ... PARAM_N OPERAND_0 ... OPERAND_N)

    if let Some(Token::Symbol(child_node_name)) = iter.peek(1) {
        let owned_name = child_node_name.to_owned();
        let inst_name = owned_name.as_str();
        let instruction = if let Some(kind) = get_instruction_kind(inst_name) {
            match *kind {
                InstructionSyntaxKind::NoParams(opcode, operand_count) => {
                    parse_instruction_kind_no_params(iter, inst_name, opcode, operand_count)?
                }
                //
                InstructionSyntaxKind::LocalLoad(opcode) => {
                    parse_instruction_kind_local_load(iter, inst_name, opcode, true)?
                }
                InstructionSyntaxKind::LocalStore(opcode) => {
                    parse_instruction_kind_local_store(iter, inst_name, opcode, true)?
                }
                InstructionSyntaxKind::LocalLongLoad(opcode) => {
                    parse_instruction_kind_local_long_load(iter, inst_name, opcode, true)?
                }
                InstructionSyntaxKind::LocalLongStore(opcode) => {
                    parse_instruction_kind_local_long_store(iter, inst_name, opcode, true)?
                }
                InstructionSyntaxKind::DataLoad(opcode) => {
                    parse_instruction_kind_local_load(iter, inst_name, opcode, false)?
                }
                InstructionSyntaxKind::DataStore(opcode) => {
                    parse_instruction_kind_local_store(iter, inst_name, opcode, false)?
                }
                InstructionSyntaxKind::DataLongLoad(opcode) => {
                    parse_instruction_kind_local_long_load(iter, inst_name, opcode, false)?
                }
                InstructionSyntaxKind::DataLongStore(opcode) => {
                    parse_instruction_kind_local_long_store(iter, inst_name, opcode, false)?
                }
                //
                InstructionSyntaxKind::HeapLoad(opcode) => {
                    parse_instruction_kind_heap_load(iter, inst_name, opcode)?
                }
                InstructionSyntaxKind::HeapStore(opcode) => {
                    parse_instruction_kind_heap_store(iter, inst_name, opcode)?
                }
                //
                InstructionSyntaxKind::UnaryOp(opcode) => {
                    parse_instruction_kind_unary_op(iter, inst_name, opcode)?
                }
                InstructionSyntaxKind::UnaryOpWithImmI16(opcode) => {
                    parse_instruction_kind_unary_op_with_imm_i16(iter, inst_name, opcode)?
                }
                InstructionSyntaxKind::BinaryOp(opcode) => {
                    parse_instruction_kind_binary_op(iter, inst_name, opcode)?
                }
                //
                InstructionSyntaxKind::ImmI32 => parse_instruction_kind_imm_i32(iter)?,
                InstructionSyntaxKind::ImmI64 => parse_instruction_kind_imm_i64(iter)?,
                InstructionSyntaxKind::ImmF32 => parse_instruction_kind_imm_f32(iter)?,
                InstructionSyntaxKind::ImmF64 => parse_instruction_kind_imm_f64(iter)?,
                //
                InstructionSyntaxKind::When => parse_instruction_kind_when(iter)?,
                InstructionSyntaxKind::If => parse_instruction_kind_if(iter)?,
                InstructionSyntaxKind::Branch => parse_instruction_kind_branch(iter)?,
                InstructionSyntaxKind::For => parse_instruction_kind_for(iter)?,

                InstructionSyntaxKind::Sequence(node_name) => {
                    parse_instruction_sequence_node(iter, node_name)?
                }
                //
                InstructionSyntaxKind::Call => {
                    parse_instruction_kind_call_by_id(iter, "call", true)?
                }
                InstructionSyntaxKind::DynCall => parse_instruction_kind_call_by_operand(iter)?,
                InstructionSyntaxKind::EnvCall => {
                    parse_instruction_kind_call_by_num(iter, "envcall", true)?
                }
                InstructionSyntaxKind::SysCall => {
                    parse_instruction_kind_call_by_num(iter, "syscall", false)?
                }
                InstructionSyntaxKind::ExtCall => {
                    parse_instruction_kind_call_by_id(iter, "extcall", false)?
                }
                // macro
                InstructionSyntaxKind::MacroGetFunctionPublicIndex => {
                    parse_instruction_kind_get_function_public_index(iter)?
                }
                InstructionSyntaxKind::Debug => parse_instruction_kind_debug(iter)?,
                InstructionSyntaxKind::Unreachable => parse_instruction_kind_unreachable(iter)?,
                InstructionSyntaxKind::HostAddrFunction => {
                    parse_instruction_kind_host_addr_function(iter)?
                }
            }
        } else {
            return Err(ParseError::new(&format!(
                "Unknown instruction: {}",
                child_node_name
            )));
        };

        Ok(instruction)
    } else {
        Err(ParseError::new("Missing symbol for instruction node."))
    }
}

// parse the instruction without parentheses,
// that is, the instruction has no_parameters and no operands.
//
// ✅: zero
// ✖️: (i32.add ...)
//
fn parse_instruction_without_parentheses(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // zero ... //
    // ^    ^___// to here
    // |________// current token

    let node_name = expect_symbol(iter, "instruction")?;
    let inst_name = node_name.as_str();

    if let Some(kind) = get_instruction_kind(inst_name) {
        match kind {
            InstructionSyntaxKind::NoParams(opcode, operand_cound) => {
                if *operand_cound > 0 {
                    Err(ParseError::new(&format!(
                        "Instruction \"{}\" has operands and should be written with parentheses.",
                        inst_name
                    )))
                } else {
                    Ok(Instruction::NoParams {
                        opcode: *opcode,
                        operands: vec![],
                    })
                }
            }
            _ => Err(ParseError::new(&format!(
                "Instruction \"{}\" should be written with parentheses.",
                inst_name
            ))),
        }
    } else {
        Err(ParseError::new(&format!(
            "Unknown instruction: {}",
            inst_name
        )))
    }
}

fn parse_instruction_kind_no_params(
    iter: &mut PeekableIterator<Token>,
    inst_name: &str,
    opcode: Opcode,
    operand_count: u8,
) -> Result<Instruction, ParseError> {
    // (name) ...  //
    // ^      ^____// to here
    // |___________// current token
    //
    // also:
    // (name OPERAND_0 ... OPERAND_N) ...  //
    // ^                              ^____// to here
    // |___________________________________// current token

    let mut operands = vec![];

    consume_left_paren(iter, &format!("instruction.{}", inst_name))?;
    consume_symbol(iter, inst_name)?;

    // operands
    for _ in 0..operand_count {
        let operand = parse_next_operand(iter, &format!("instruction.{}", inst_name))?;
        operands.push(operand);
    }

    consume_right_paren(iter)?;

    Ok(Instruction::NoParams { opcode, operands })
}

fn parse_instruction_kind_imm_i32(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (i32.imm 123) ... //
    // ^             ^___// to here
    // |_________________// current token

    consume_left_paren(iter, "instruction.i32.imm")?;
    consume_symbol(iter, "i32.imm")?;
    let number_token = expect_number(iter, "instruction.i32.imm.value")?;
    consume_right_paren(iter)?;

    Ok(Instruction::ImmI32(parse_u32_string(&number_token)?))
}

fn parse_instruction_kind_imm_i64(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (i64.imm 123) ... //
    // ^             ^___// to here
    // |_________________// current token

    consume_left_paren(iter, "instruction.i64.imm")?;
    consume_symbol(iter, "i64.imm")?;
    let number_token = expect_number(iter, "instruction.i64.imm.value")?;
    consume_right_paren(iter)?;

    Ok(Instruction::ImmI64(parse_u64_string(&number_token)?))
}

fn parse_instruction_kind_imm_f32(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (f32.imm 3.14) ... //
    // ^              ^___// to here
    // |__________________// current token
    //
    // also:
    // (f32.imm 0x1.23p+4)

    consume_left_paren(iter, "instruction.f32.imm")?;
    consume_symbol(iter, "f32.imm")?;
    let number_token = expect_number(iter, "instruction.f32.imm.value")?;
    consume_right_paren(iter)?;

    let imm_f32 = parse_f32_string(&number_token)?;
    Ok(Instruction::ImmF32(imm_f32))
}

fn parse_instruction_kind_imm_f64(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (f64.imm 3.14) ... //
    // ^              ^___// to here
    // |__________________// current token
    //
    // also:
    // (f64.imm 0x1.23p+4)

    consume_left_paren(iter, "instruction.f64.imm")?;
    consume_symbol(iter, "f64.imm")?;
    let number_token = expect_number(iter, "instruction.f64.imm.value")?;
    consume_right_paren(iter)?;

    let imm_f64 = parse_f64_string(&number_token)?;
    Ok(Instruction::ImmF64(imm_f64))
}

fn parse_instruction_kind_local_load(
    iter: &mut PeekableIterator<Token>,
    inst_name: &str,
    opcode: Opcode,
    is_local: bool,
) -> Result<Instruction, ParseError> {
    // (local.load64_i64 $name) ... //
    // ^                        ^___// to here
    // |____________________________// current token
    //
    // also:
    // (local.load64_i64 $name OFFSET:i16)

    consume_left_paren(iter, &format!("instruction.{}", inst_name))?;
    consume_symbol(iter, inst_name)?;
    let name = expect_identifier(iter, &format!("instruction.{}.name", inst_name))?;
    let offset = if let Some(offset_number_token) = expect_number_optional(iter) {
        parse_u16_string(&offset_number_token)?
    } else {
        0
    };
    consume_right_paren(iter)?;

    if is_local {
        Ok(Instruction::LocalLoad {
            opcode,
            name,
            offset,
        })
    } else {
        Ok(Instruction::DataLoad {
            opcode,
            id: name,
            offset,
        })
    }
}

fn parse_instruction_kind_local_store(
    iter: &mut PeekableIterator<Token>,
    inst_name: &str,
    opcode: Opcode,
    is_local: bool,
) -> Result<Instruction, ParseError> {
    // (local.store $name VALUE) ... //
    // ^                         ^___// to here
    // |_____________________________// current token
    //
    // also:
    // (local.store $name OFFSET:i16 VALUE)

    consume_left_paren(iter, &format!("instruction.{}", inst_name))?;
    consume_symbol(iter, inst_name)?;
    let name = expect_identifier(iter, &format!("instruction.{}.name", inst_name))?;
    let offset = if let Some(offset_number_token) = expect_number_optional(iter) {
        parse_u16_string(&offset_number_token)?
    } else {
        0
    };

    let operand = parse_next_operand(iter, &format!("instruction.{}", inst_name))?;
    consume_right_paren(iter)?;

    if is_local {
        Ok(Instruction::LocalStore {
            opcode,
            name,
            offset,
            value: Box::new(operand),
        })
    } else {
        Ok(Instruction::DataStore {
            opcode,
            id: name,
            offset,
            value: Box::new(operand),
        })
    }
}

fn parse_instruction_kind_local_long_load(
    iter: &mut PeekableIterator<Token>,
    inst_name: &str,
    opcode: Opcode,
    is_local: bool,
) -> Result<Instruction, ParseError> {
    // (local.long_load $name OFFSET:i32) ... //
    // ^                                  ^___// to here
    // |______________________________________// current token

    consume_left_paren(iter, &format!("instruction.{}", inst_name))?;
    consume_symbol(iter, inst_name)?;
    let name = expect_identifier(iter, &format!("instruction.{}.name", inst_name))?;
    let offset = parse_next_operand(iter, &format!("instruction.{}", inst_name))?;
    consume_right_paren(iter)?;

    if is_local {
        Ok(Instruction::LocalLongLoad {
            opcode,
            name,
            offset: Box::new(offset),
        })
    } else {
        Ok(Instruction::DataLongLoad {
            opcode,
            id: name,
            offset: Box::new(offset),
        })
    }
}

fn parse_instruction_kind_local_long_store(
    iter: &mut PeekableIterator<Token>,
    inst_name: &str,
    opcode: Opcode,
    is_local: bool,
) -> Result<Instruction, ParseError> {
    // (local.long_store $name OFFSET:i32 VALUE) ... //
    // ^                                         ^___// to here
    // |_____________________________________________// current token

    consume_left_paren(iter, &format!("instruction.{}", inst_name))?;
    consume_symbol(iter, inst_name)?;
    let name = expect_identifier(iter, &format!("instruction.{}.name", inst_name))?;
    let offset = parse_next_operand(iter, &format!("instruction.{}", inst_name))?;
    let operand = parse_next_operand(iter, &format!("instruction.{}", inst_name))?;
    consume_right_paren(iter)?;

    if is_local {
        Ok(Instruction::LocalLongStore {
            opcode,
            name,
            offset: Box::new(offset),
            value: Box::new(operand),
        })
    } else {
        Ok(Instruction::DataLongStore {
            opcode,
            id: name,
            offset: Box::new(offset),
            value: Box::new(operand),
        })
    }
}

fn parse_instruction_kind_heap_load(
    iter: &mut PeekableIterator<Token>,
    inst_name: &str,
    opcode: Opcode,
) -> Result<Instruction, ParseError> {
    // (heap.load ADDR) ... //
    // ^                ^___// to here
    // |____________________// current token
    //
    // also:
    // (heap.load OFFSET:i16 ADDR)

    consume_left_paren(iter, &format!("instruction.{}", inst_name))?;
    consume_symbol(iter, inst_name)?;

    let offset = if let Some(offset_token_number) = expect_number_optional(iter) {
        parse_u16_string(&offset_token_number)?
    } else {
        0
    };

    let addr = parse_next_operand(iter, &format!("instruction.{}.addr", inst_name))?;
    consume_right_paren(iter)?;

    Ok(Instruction::HeapLoad {
        opcode,
        offset,
        addr: Box::new(addr),
    })
}

fn parse_instruction_kind_heap_store(
    iter: &mut PeekableIterator<Token>,
    inst_name: &str,
    opcode: Opcode,
) -> Result<Instruction, ParseError> {
    // (heap.store ADDR VALUE) ... //
    // ^                       ^___// to here
    // |___________________________// current token
    //
    // also:
    // (heap.store OFFSET:i16 ADDR VALUE)

    consume_left_paren(iter, &format!("instruction.{}", inst_name))?;
    consume_symbol(iter, inst_name)?;

    let offset = if let Some(offset_number_token) = expect_number_optional(iter) {
        parse_u16_string(&offset_number_token)?
    } else {
        0
    };

    let addr = parse_next_operand(iter, &format!("instruction.{}.addr", inst_name))?;
    let value = parse_next_operand(iter, &format!("instruction.{}.value", inst_name))?;

    consume_right_paren(iter)?;

    Ok(Instruction::HeapStore {
        opcode,
        offset,
        addr: Box::new(addr),
        value: Box::new(value),
    })
}

fn parse_instruction_kind_unary_op(
    iter: &mut PeekableIterator<Token>,
    inst_name: &str,
    opcode: Opcode,
) -> Result<Instruction, ParseError> {
    // (i32.not VALUE) ... //
    // ^               ^___// to here
    // |___________________// current token

    consume_left_paren(iter, &format!("instruction.{}", inst_name))?;
    consume_symbol(iter, inst_name)?;
    let source = parse_next_operand(iter, &format!("instruction.{}.source", inst_name))?;
    consume_right_paren(iter)?;

    Ok(Instruction::UnaryOp {
        opcode,
        source: Box::new(source),
    })
}

fn parse_instruction_kind_unary_op_with_imm_i16(
    iter: &mut PeekableIterator<Token>,
    inst_name: &str,
    opcode: Opcode,
) -> Result<Instruction, ParseError> {
    // (i32.inc imm:i16 VALUE) ... //
    // ^                       ^___// to here
    // |___________________________// current token

    consume_left_paren(iter, &format!("instruction.{}", inst_name))?;
    consume_symbol(iter, inst_name)?;
    let imm_token = expect_number(iter, &format!("instruction.{}.imm", inst_name))?;
    let imm_i16 = parse_u16_string(&imm_token)?;
    let source = parse_next_operand(iter, &format!("instruction.{}.source", inst_name))?;
    consume_right_paren(iter)?;

    Ok(Instruction::UnaryOpWithImmI16 {
        opcode,
        imm: imm_i16,
        source: Box::new(source),
    })
}

fn parse_instruction_kind_binary_op(
    iter: &mut PeekableIterator<Token>,
    inst_name: &str,
    opcode: Opcode,
) -> Result<Instruction, ParseError> {
    // (i32.add LHS RHS) ... //
    // ^                 ^___// to here
    // |_____________________// current token

    consume_left_paren(iter, &format!("instruction.{}", inst_name))?;
    consume_symbol(iter, inst_name)?;
    let left = parse_next_operand(iter, &format!("instruction.{}.left", inst_name))?;
    let right = parse_next_operand(iter, &format!("instruction.{}.right", inst_name))?;
    consume_right_paren(iter)?;

    Ok(Instruction::BinaryOp {
        opcode,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn parse_instruction_kind_when(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (when TEST CONSEQUENT) ... //
    // ^                      ^___// to here
    // |__________________________// current token

    consume_left_paren(iter, "instruction.when")?;
    consume_symbol(iter, "when")?;
    let test = parse_next_operand(iter, "instruction.when.test")?;
    // let locals = parse_optional_local_variables(iter)?;
    let consequent = parse_next_operand(iter, "instruction.when.consequent")?;
    consume_right_paren(iter)?;

    Ok(Instruction::When {
        // locals,
        test: Box::new(test),
        consequent: Box::new(consequent),
    })
}

fn parse_instruction_kind_if(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (if (result...) TEST CONSEQUENT ALTERNATE) ... //
    // ^                                          ^___// to here
    // |______________________________________________// current token

    consume_left_paren(iter, "instruction.if")?;
    consume_symbol(iter, "if")?;
    let results = parse_optional_signature_results_only(iter)?;
    let test = parse_next_operand(iter, "instruction.if.test")?;
    // let locals = parse_optional_local_variables(iter)?;
    let consequent = parse_next_operand(iter, "instruction.if.consequent")?;
    let alternate = parse_next_operand(iter, "instruction.if.alternate")?;
    consume_right_paren(iter)?;

    Ok(Instruction::If {
        // params,
        results,
        // locals,
        test: Box::new(test),
        consequent: Box::new(consequent),
        alternate: Box::new(alternate),
    })
}

fn parse_instruction_kind_branch(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (branch (result...)
    //     (case TEST_0 CONSEQUENT_0)
    //     ...
    //     (case TEST_N CONSEQUENT_N)
    //     (default CONSEQUENT_DEFAULT) ;; optional
    //     ) ... //
    // ^     ^___// to here
    // |_________// current token

    consume_left_paren(iter, "instruction.branch")?;
    consume_symbol(iter, "branch")?;
    let results = parse_optional_signature_results_only(iter)?;
    // let locals = parse_optional_local_variables(iter)?;
    let mut cases = vec![];

    while exist_child_node(iter, "case") {
        consume_left_paren(iter, "instruction.branch.case")?;
        consume_symbol(iter, "case")?;
        let test = parse_next_operand(iter, "instruction.branch.case.test")?;
        let consequent = parse_next_operand(iter, "instruction.branch.case.consequent")?;
        consume_right_paren(iter)?;

        cases.push(BranchCase {
            test: Box::new(test),
            consequent: Box::new(consequent),
        });
    }

    let mut default = None;
    if exist_child_node(iter, "default") {
        consume_left_paren(iter, "instruction.branch.default")?;
        consume_symbol(iter, "default")?;
        let consequent = parse_next_operand(iter, "instruction.branch.default")?;
        consume_right_paren(iter)?;

        default = Some(Box::new(consequent));
    }

    consume_right_paren(iter)?;

    Ok(Instruction::Branch {
        // params,
        results,
        // locals,
        cases,
        default,
    })
}

fn parse_instruction_kind_for(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (for (param...) (result...) (local...) INSTRUCTION) ... //
    // ^                                                   ^___// to here
    // |_______________________________________________________// current token

    consume_left_paren(iter, "instruction.for")?;
    consume_symbol(iter, "for")?;
    let (params, results) = parse_optional_signature(iter)?;
    let locals = parse_optional_local_variables(iter)?;
    let code = parse_next_operand(iter, "instruction.for.code")?;
    consume_right_paren(iter)?;

    Ok(Instruction::For {
        params,
        results,
        locals,
        code: Box::new(code),
    })
}

fn parse_instruction_kind_call_by_id(
    iter: &mut PeekableIterator<Token>,
    node_name: &str,
    is_call: bool,
) -> Result<Instruction, ParseError> {
    // (call/extcall $id ...) ...  //
    // ^                      ^____// to here
    // ____________________________// current token

    consume_left_paren(iter, &format!("instruction.{}", node_name))?;
    consume_symbol(iter, node_name)?;
    let name_path = expect_identifier(iter, &format!("instruction.{}.name", node_name))?;

    let mut args = vec![];
    while let Some(arg) = parse_next_instruction_optional(iter)? {
        args.push(arg);
    }

    consume_right_paren(iter)?;

    let instruction = if is_call {
        Instruction::Call {
            id: name_path,
            args,
        }
    } else {
        Instruction::ExtCall {
            id: name_path,
            args,
        }
    };

    Ok(instruction)
}

fn parse_instruction_kind_call_by_num(
    iter: &mut PeekableIterator<Token>,
    node_name: &str,
    is_envcall: bool,
) -> Result<Instruction, ParseError> {
    // (envcall/syscall NUM ...) ...  //
    // ^                         ^____// to here
    // _______________________________// current token

    consume_left_paren(iter, &format!("instruction.{}", node_name))?;
    consume_symbol(iter, node_name)?;
    let number_token = expect_number(iter, &format!("instruction.{}.number", node_name))?;
    let num = parse_u32_string(&number_token)?;

    let mut args = vec![];
    while let Some(arg) = parse_next_instruction_optional(iter)? {
        args.push(arg);
    }

    consume_right_paren(iter)?;

    let instruction = if is_envcall {
        Instruction::EnvCall { num, args }
    } else {
        Instruction::SysCall { num, args }
    };

    Ok(instruction)
}

fn parse_instruction_kind_call_by_operand(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (dyncall FUNC_INDEX ...) ...  //
    // ^                        ^____// to here
    // ______________________________// current token

    consume_left_paren(iter, "instruction.dyncall")?;
    consume_symbol(iter, "dyncall")?;

    // function public index
    let public_index = parse_next_operand(iter, "instruction.dyncall")?;

    let mut args = vec![];
    while let Some(arg) = parse_next_instruction_optional(iter)? {
        args.push(arg);
    }

    consume_right_paren(iter)?;

    Ok(Instruction::DynCall {
        public_index: Box::new(public_index),
        args,
    })
}

fn parse_instruction_kind_get_function_public_index(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (macro.get_function_public_index $id ...) ...  //
    // ^                                         ^____// to here
    // _______________________________________________// current token

    consume_left_paren(iter, "macro.get_function_public_index")?;
    consume_symbol(iter, "macro.get_function_public_index")?;
    let id = expect_identifier(iter, "macro.get_function_public_index.name")?;
    consume_right_paren(iter)?;

    Ok(Instruction::MacroGetFunctionPublicIndex { id })
}

fn parse_instruction_kind_debug(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (debug num ...) ...  //
    // ^               ^____// to here
    // _____________________// current token

    consume_left_paren(iter, "instruction.debug")?;
    consume_symbol(iter, "debug")?;
    let code_token = expect_number(iter, "instruction.debug.code")?;
    let code = parse_u32_string(&code_token)?;
    consume_right_paren(iter)?;

    Ok(Instruction::Debug { code })
}

fn parse_instruction_kind_unreachable(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (unreachable num ...) ...  //
    // ^                     ^____// to here
    // ___________________________// current token

    consume_left_paren(iter, "instruction.unreachable")?;
    consume_symbol(iter, "unreachable")?;
    let code_token = expect_number(iter, "instruction.unreachable.code")?;
    let code = parse_u32_string(&code_token)?;
    consume_right_paren(iter)?;

    Ok(Instruction::Unreachable { code })
}

fn parse_instruction_kind_host_addr_function(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (host.addr_function $id ...) ...  //
    // ^                            ^____// to here
    // __________________________________// current token

    consume_left_paren(iter, "instruction.host.addr_function")?;
    consume_symbol(iter, "host.addr_function")?;
    let id = expect_identifier(iter, "instruction.host.addr_function.name")?;
    consume_right_paren(iter)?;

    Ok(Instruction::HostAddrFunction { id })
}

fn parse_data_node(iter: &mut PeekableIterator<Token>) -> Result<ModuleElementNode, ParseError> {
    // (data $name (read_only i32 123)) ...  //
    // ^                                ^____// to here
    // |_____________________________________// current token

    // also:
    // (data $name (read_only string "Hello, World!"))    ;; UTF-8 encoding string
    // (data $name (read_only cstring "Hello, World!"))   ;; type `cstring` will append '\0' at the end of string
    // (data $name (read_only (bytes 2) h"11-13-17-19"))

    // other sections than 'read_only'
    //
    // read-write section:
    // (data $name (read_write i32 123))
    //
    // uninitialized section:
    // (data $name (uninit i32))
    // (data $name (uninit (bytes 12 4)))

    // with 'export' annotation
    // (data export $name (read_only i32 123))

    consume_left_paren(iter, "data")?;
    consume_symbol(iter, "data")?;

    let export = expect_specified_symbol_optional(iter, "export");
    let name = expect_identifier(iter, "data.name")?;
    let data_kind = parse_data_kind_node(iter)?;

    consume_right_paren(iter)?;

    if name.contains(NAME_PATH_SEPARATOR) {
        return Err(ParseError {
            message: format!(
                "The name of data can not contains path separator, name: \"{}\"",
                name
            ),
        });
    }

    let data_node = DataNode {
        name,
        export,
        data_kind,
    };

    Ok(ModuleElementNode::DataNode(data_node))
}

fn parse_data_kind_node(iter: &mut PeekableIterator<Token>) -> Result<DataKindNode, ParseError> {
    // (read_only i32 123) ...  //
    // ^                   ^____// to here
    // |________________________// current token

    // also
    // (read_write i32 123)
    // (uninit i32)

    match iter.peek(1) {
        Some(Token::Symbol(kind)) => match kind.as_str() {
            "read_only" => parse_data_kind_node_read_only(iter),
            "read_write" => parse_data_kind_node_read_write(iter),
            "uninit" => parse_data_kind_node_uninit(iter),
            _ => Err(ParseError::new(&format!(
                "Unknown data node kind: {}, only supports \"read_only\", \"read_write\", \"uninit\"",
                kind
            ))),
        },
        _ => Err(ParseError::new("Missing data kind for data node")),
    }
}

fn parse_data_kind_node_read_only(
    iter: &mut PeekableIterator<Token>,
) -> Result<DataKindNode, ParseError> {
    // (read_only i32 123) ...  //
    // ^                   ^____// to here
    // |________________________// current token

    // also:
    // (read_only string "Hello, World!")    ;; UTF-8 encoding string
    // (read_only cstring "Hello, World!")   ;; type `cstring` will append '\0' at the end of string
    // (read_only (bytes ALIGN:i16) h"11-13-17-19")

    consume_left_paren(iter, "data.read_only")?;
    consume_symbol(iter, "read_only")?;

    let inited_data = parse_inited_data(iter)?;
    consume_right_paren(iter)?;

    let data_kind_node = DataKindNode::ReadOnly(inited_data);
    Ok(data_kind_node)
}

fn parse_data_kind_node_read_write(
    iter: &mut PeekableIterator<Token>,
) -> Result<DataKindNode, ParseError> {
    // (read_write i32 123) ...  //
    // ^                    ^____// to here
    // |_________________________// current token

    // also:
    // (read_write string "Hello, World!")    ;; UTF-8 encoding string
    // (read_write cstring "Hello, World!")   ;; type `cstring` will append '\0' at the end of string
    // (read_write (bytes ALIGN:i16) h"11-13-17-19")

    consume_left_paren(iter, "data.read_write")?;
    consume_symbol(iter, "read_write")?;

    let inited_data = parse_inited_data(iter)?;
    consume_right_paren(iter)?;

    let data_kind_node = DataKindNode::ReadWrite(inited_data);
    Ok(data_kind_node)
}

fn parse_data_kind_node_uninit(
    iter: &mut PeekableIterator<Token>,
) -> Result<DataKindNode, ParseError> {
    // (uninit i32) ... //
    // ^            ^___// to here
    // |________________// current token

    // also:
    // (uninit (bytes 12 4))

    consume_left_paren(iter, "data.uninit")?;
    consume_symbol(iter, "uninit")?;

    let (memory_data_type, data_length, align) =
        parse_memory_data_type_with_length_and_align(iter)?;
    let uninit_data = UninitData {
        memory_data_type,
        length: data_length,
        align,
    };
    consume_right_paren(iter)?;

    let data_kind_node = DataKindNode::Uninit(uninit_data);
    Ok(data_kind_node)
}

fn parse_inited_data(iter: &mut PeekableIterator<Token>) -> Result<InitedData, ParseError> {
    // (read_only i32 123) ...  //
    //            ^      ^______// to here
    //            |_____________// current token

    // also:
    // (read_write string "Hello, World!")    ;; UTF-8 encoding string
    // (read_write cstring "Hello, World!")   ;; type `cstring` will append '\0' at the end of string
    // (read_write (bytes ALIGN:i16) h"11-13-17-19")

    let inited_data = match iter.next() {
        Some(Token::Symbol(inited_data_type)) => match inited_data_type.as_str() {
            "i32" => {
                let value_token = expect_number(iter, "data.i32.value")?;
                let value = parse_u32_string(&value_token)?;
                let bytes = value.to_le_bytes().to_vec();

                InitedData {
                    memory_data_type: MemoryDataType::I32,
                    length: 4,
                    align: 4,
                    value: bytes,
                }
            }
            "i64" => {
                let value_token = expect_number(iter, "data.i64.value")?;
                let value = parse_u64_string(&value_token)?;
                let bytes = value.to_le_bytes().to_vec();

                InitedData {
                    memory_data_type: MemoryDataType::I64,
                    length: 8,
                    align: 8,
                    value: bytes,
                }
            }
            "f32" => {
                let value_token = expect_number(iter, "data.f32.value")?;
                let value = parse_f32_string(&value_token)?;
                let bytes = value.to_le_bytes().to_vec();

                InitedData {
                    memory_data_type: MemoryDataType::F32,
                    length: 4,
                    align: 4,
                    value: bytes,
                }
            }
            "f64" => {
                let value_token = expect_number(iter, "data.f64.value")?;
                let value = parse_f64_string(&value_token)?;
                let bytes = value.to_le_bytes().to_vec();

                InitedData {
                    memory_data_type: MemoryDataType::F64,
                    length: 8,
                    align: 8,
                    value: bytes,
                }
            }
            "string" => {
                let value = expect_string(iter, "data.string.value")?;
                let bytes = value.as_bytes().to_vec();
                InitedData {
                    memory_data_type: MemoryDataType::Bytes,
                    length: bytes.len() as u32,
                    align: 1,
                    value: bytes,
                }
            }
            "cstring" => {
                let value = expect_string(iter, "data.cstring.value")?;
                let mut bytes = value.as_bytes().to_vec();
                bytes.push(0); // append '\0'

                InitedData {
                    memory_data_type: MemoryDataType::Bytes,
                    length: bytes.len() as u32,
                    align: 1,
                    value: bytes,
                }
            }
            _ => {
                return Err(ParseError::new(&format!(
                    "Unknown data type \"{}\" for the data item",
                    inited_data_type
                )))
            }
        },
        Some(Token::LeftParen)
            if iter.look_ahead_equals(0, &Token::Symbol("bytes".to_string())) =>
        {
            // (bytes ALIGH:i16) DATA ...  //
            //  ^                            ^____//
            //  |_________________________________//

            consume_symbol(iter, "bytes")?;
            let align_token = expect_number(iter, "data.bytes.align")?;
            let align = parse_u16_string(&align_token)?;
            consume_right_paren(iter)?;

            let bytes = expect_bytes(iter, "data.bytes.value")?;

            InitedData {
                memory_data_type: MemoryDataType::Bytes,
                length: bytes.len() as u32,
                align,
                value: bytes,
            }
        }
        _ => return Err(ParseError::new("Missing the value of data item")),
    };

    Ok(inited_data)
}

fn parse_external_node(
    iter: &mut PeekableIterator<Token>,
) -> Result<ModuleElementNode, ParseError> {
    // (external
    //     (library share "math.so.1")
    //     (function $add "add" (param i32) (param i32) (result i32))
    //     ) ...  //
    // ^     ^____// to here
    // |__________// current token

    consume_left_paren(iter, "external")?;
    consume_symbol(iter, "external")?;

    let external_library_node = parse_external_library_node(iter)?;

    let mut external_items: Vec<ExternalItem> = vec![];

    // parse external items
    while iter.look_ahead_equals(0, &Token::LeftParen) {
        if let Some(Token::Symbol(child_node_name)) = iter.peek(1) {
            let external_item = match child_node_name.as_str() {
                "function" => parse_external_function_node(iter)?,
                _ => {
                    return Err(ParseError::new(&format!(
                        "Unknown external item: {}",
                        child_node_name
                    )))
                }
            };
            external_items.push(external_item);
        } else {
            break;
        }
    }

    consume_right_paren(iter)?;

    let external_node = ExternalNode {
        external_library_node,
        external_items,
    };

    Ok(ModuleElementNode::ExternalNode(external_node))
}

fn parse_external_library_node(
    iter: &mut PeekableIterator<Token>,
) -> Result<ExternalLibraryNode, ParseError> {
    // (library share "math.so.1") ...  //
    // ^                           ^____// to here
    // |________________________________// current token

    // also:
    // (library system "libc.so.6")
    // (library user "lib-test-0.so.1")

    consume_left_paren(iter, "external.library")?;
    consume_symbol(iter, "library")?;

    let external_library_type_str = expect_symbol(iter, "external.library.type")?;
    let external_library_type = match external_library_type_str.as_str() {
        "share" => ExternalLibraryType::Share,
        "system" => ExternalLibraryType::System,
        "user" => ExternalLibraryType::User,
        _ => {
            return Err(ParseError {
                message: format!("Unknown share library type: {}", external_library_type_str),
            })
        }
    };

    let name = expect_string(iter, "external.library.name")?;
    consume_right_paren(iter)?;

    Ok(ExternalLibraryNode {
        external_library_type,
        name,
    })
}

fn parse_external_function_node(
    iter: &mut PeekableIterator<Token>,
) -> Result<ExternalItem, ParseError> {
    // (function $add "add"
    //      (param i32) (param i32)
    //      (result i32)) ...  //
    // ^                  ^____// to here
    // |_______________________// current token

    // also
    // (function $add "add" (params i32 i32) (result i32))

    consume_left_paren(iter, "external.function)")?;
    consume_symbol(iter, "function")?;

    let id = expect_identifier(iter, "external.function.id")?;
    let name = expect_string(iter, "external.function.name")?;
    let (params, results) = parse_optional_identifier_less_signature(iter)?;

    consume_right_paren(iter)?;

    if id.contains(NAME_PATH_SEPARATOR) {
        return Err(ParseError {
            message: format!(
                "The id of external function can not contains path separator, id: \"{}\"",
                id
            ),
        });
    }

    Ok(ExternalItem::ExternalFunction(ExternalFunctionNode {
        id,
        name,
        params,
        results,
    }))
}

fn parse_optional_identifier_less_signature(
    iter: &mut PeekableIterator<Token>,
) -> Result<(Vec<DataType>, Vec<DataType>), ParseError> {
    // (param|params|result|results ...){0,} ...  //
    // ^                                     ^____// to here
    // |__________________________________________// current token

    let mut params: Vec<DataType> = vec![];
    let mut results: Vec<DataType> = vec![];

    while iter.look_ahead_equals(0, &Token::LeftParen) {
        if let Some(Token::Symbol(child_node_name)) = iter.peek(1) {
            match child_node_name.as_str() {
                "param" => {
                    let data_type = parse_identifier_less_param_node(iter)?;
                    params.push(data_type);
                }
                "params" => {
                    let mut data_types = parse_identifier_less_params_node(iter)?;
                    params.append(&mut data_types);
                }
                "result" => {
                    let data_type = parse_result_node(iter)?;
                    results.push(data_type);
                }
                "results" => {
                    let mut data_types = parse_results_node(iter)?;
                    results.append(&mut data_types);
                }
                _ => break,
            }
        } else {
            break;
        }
    }

    Ok((params, results))
}

fn parse_identifier_less_param_node(
    iter: &mut PeekableIterator<Token>,
) -> Result<DataType, ParseError> {
    // (param i32) ...  //
    // ^           ^____// to here
    // |________________// current token

    // the simplified parameter has no identifier.

    consume_left_paren(iter, "param")?;
    consume_symbol(iter, "param")?;
    let data_type = parse_data_type(iter)?;
    consume_right_paren(iter)?;

    Ok(data_type)
}

fn parse_identifier_less_params_node(
    iter: &mut PeekableIterator<Token>,
) -> Result<Vec<DataType>, ParseError> {
    // (params i32 i32 i64) ...  //
    // ^                    ^____// to here
    // |_________________________// current token

    let mut data_types: Vec<DataType> = vec![];

    consume_left_paren(iter, "params")?;
    consume_symbol(iter, "params")?;
    while matches!(iter.peek(0), &Some(Token::Symbol(_))) {
        let data_type = parse_data_type(iter)?;
        data_types.push(data_type);
    }

    consume_right_paren(iter)?;

    Ok(data_types)
}

fn parse_import_node(iter: &mut PeekableIterator<Token>) -> Result<ModuleElementNode, ParseError> {
    // (import
    //     (module share "math" "1.0")
    //     (function $add "add" (param i32) (param i32) (result i32))
    //     (data $msg "msg" (read_only i32))
    //     ) ...  //
    // ^     ^____// to here
    // |__________// current token

    consume_left_paren(iter, "import")?;
    consume_symbol(iter, "import")?;

    let import_module_node = parse_import_module_node(iter)?;

    let mut import_items: Vec<ImportItem> = vec![];

    // parse import items
    while iter.look_ahead_equals(0, &Token::LeftParen) {
        if let Some(Token::Symbol(child_node_name)) = iter.peek(1) {
            let import_item = match child_node_name.as_str() {
                "function" => parse_import_function_node(iter)?,
                "data" => parse_import_data_node(iter)?,
                _ => {
                    return Err(ParseError::new(&format!(
                        "Unknown import item: {}",
                        child_node_name
                    )))
                }
            };
            import_items.push(import_item);
        } else {
            break;
        }
    }

    consume_right_paren(iter)?;

    let import_node = ImportNode {
        import_module_node,
        import_items,
    };

    Ok(ModuleElementNode::ImportNode(import_node))
}

fn parse_import_module_node(
    iter: &mut PeekableIterator<Token>,
) -> Result<ImportModuleNode, ParseError> {
    // (module share "math" "1.0") ...  //
    // ^                     ^____// to here
    // |__________________________// current token

    // also:
    // (module user "math" "1.0")

    consume_left_paren(iter, "import.module")?;
    consume_symbol(iter, "module")?;

    let module_share_type_str = expect_symbol(iter, "import.module.share_type")?;
    let module_share_type = match module_share_type_str.as_str() {
        "share" => ModuleShareType::Share,
        "user" => ModuleShareType::User,
        _ => {
            return Err(ParseError {
                message: format!("Unknown module share type: {}", module_share_type_str),
            })
        }
    };

    let name = expect_string(iter, "import.module.name")?;
    let ver_string = expect_string(iter, "import.module.version")?;
    consume_right_paren(iter)?;

    let (version_major, version_minor) = parse_version(&ver_string)?;

    Ok(ImportModuleNode {
        module_share_type,
        name,
        version_major,
        version_minor,
    })
}

fn parse_import_function_node(
    iter: &mut PeekableIterator<Token>,
) -> Result<ImportItem, ParseError> {
    // (function $add "add"
    //      (param i32) (param i32)
    //      (result i32)) ...  //
    // ^                  ^____// to here
    // |_______________________// current token

    // also
    // (function $add "add" (params i32 i32) (result i32))

    consume_left_paren(iter, "import.function)")?;
    consume_symbol(iter, "function")?;

    let id = expect_identifier(iter, "import.function.id")?;

    // the original exported name path (excludes the module name)
    let name_path = expect_string(iter, "import.function.name")?;

    let (params, results) = parse_optional_identifier_less_signature(iter)?;

    consume_right_paren(iter)?;

    if id.contains(NAME_PATH_SEPARATOR) {
        return Err(ParseError {
            message: format!(
                "The id of import function can not contains path separator, id: \"{}\"",
                id
            ),
        });
    }

    Ok(ImportItem::ImportFunction(ImportFunctionNode {
        id,
        name_path,
        params,
        results,
    }))
}

fn parse_import_data_node(iter: &mut PeekableIterator<Token>) -> Result<ImportItem, ParseError> {
    // (data $sum "sum" (read_write i32)) ...  //
    // ^                                  ^____// to here
    // |_______________________________________// current token

    // also
    // (data $msg "msg" (read_only i64))
    // (data $buf "utils::buf" (uninit bytes))

    consume_left_paren(iter, "import.data)")?;
    consume_symbol(iter, "data")?;

    let id = expect_identifier(iter, "import.data.id")?;

    // the original exported name path (excludes the module name)
    let name_path = expect_string(iter, "import.data.name")?;

    let memory_data_type_str = expect_symbol(iter, "import.data.type")?;
    let memory_data_type = parse_memory_data_type(&memory_data_type_str)?;

    let data_section_type_str = expect_symbol(iter, "import.data.section")?;
    let data_section_type = parse_data_section_kind(&data_section_type_str)?;

    consume_right_paren(iter)?;

    if id.contains(NAME_PATH_SEPARATOR) {
        return Err(ParseError {
            message: format!(
                "The id of import data can not contains path separator, id: \"{}\"",
                id
            ),
        });
    }

    Ok(ImportItem::ImportData(ImportDataNode {
        id,
        name_path,
        // data_kind_node,
        memory_data_type,
        data_section_type,
    }))
}

fn parse_data_section_kind(kind: &str) -> Result<DataSectionType, ParseError> {
    // "read_only"
    // "read_write"
    // "uninit"

    match kind {
        "read_only" => Ok(DataSectionType::ReadOnly),
        "read_write" => Ok(DataSectionType::ReadWrite),
        "uninit" => Ok(DataSectionType::Uninit),
        _ => Err(ParseError::new(&format!(
            "Unknown data section type: {}, only \"read_only\", \"read_write\", \"uninit\" are supported.",
            kind
        ))),
    }
}

// helper functions

fn consume_token(
    iter: &mut PeekableIterator<Token>,
    expect_token: Token,
) -> Result<(), ParseError> {
    let opt_token = iter.next();
    if let Some(token) = opt_token {
        if token == expect_token {
            Ok(())
        } else {
            Err(ParseError::new(&format!(
                "Expect token: {:?}, actual token: {:?}",
                expect_token, token
            )))
        }
    } else {
        Err(ParseError::new(&format!(
            "Missing token: {:?}",
            expect_token
        )))
    }
}

fn consume_left_paren(
    iter: &mut PeekableIterator<Token>,
    for_what: &str,
) -> Result<(), ParseError> {
    if let Some(Token::LeftParen) = iter.next() {
        Ok(())
    } else {
        Err(ParseError::new(&format!("Expect a node for {}", for_what)))
    }
}

fn consume_right_paren(iter: &mut PeekableIterator<Token>) -> Result<(), ParseError> {
    consume_token(iter, Token::RightParen)
}

fn consume_symbol(iter: &mut PeekableIterator<Token>, name: &str) -> Result<(), ParseError> {
    consume_token(iter, Token::new_symbol(name))
}

fn expect_number(
    iter: &mut PeekableIterator<Token>,
    for_what: &str,
) -> Result<NumberToken, ParseError> {
    match iter.next() {
        Some(Token::Number(number_token)) => Ok(number_token),
        _ => Err(ParseError::new(&format!(
            "Expect a number for {}",
            for_what
        ))),
    }
}

fn expect_number_optional(iter: &mut PeekableIterator<Token>) -> Option<NumberToken> {
    match iter.peek(0) {
        Some(Token::Number(n)) => {
            let value = n.to_owned();
            iter.next();
            Some(value)
        }
        _ => None,
    }
}

fn expect_string(iter: &mut PeekableIterator<Token>, for_what: &str) -> Result<String, ParseError> {
    match iter.next() {
        Some(Token::String_(s)) => Ok(s),
        _ => Err(ParseError::new(&format!(
            "Expect a string for {}",
            for_what
        ))),
    }
}

fn expect_bytes(iter: &mut PeekableIterator<Token>, for_what: &str) -> Result<Vec<u8>, ParseError> {
    match iter.next() {
        Some(Token::ByteData(s)) => Ok(s),
        _ => Err(ParseError::new(&format!("Expect a bytes for {}", for_what))),
    }
}

fn expect_symbol(iter: &mut PeekableIterator<Token>, for_what: &str) -> Result<String, ParseError> {
    match iter.next() {
        Some(token) => match token {
            Token::Symbol(s) => Ok(s),
            _ => Err(ParseError::new(&format!(
                "Expect a symbol for {}",
                for_what
            ))),
        },
        None => Err(ParseError::new(&format!(
            "Missing a symbol for {}",
            for_what
        ))),
    }
}

// consume the specified symbol if it exists
fn expect_specified_symbol_optional(iter: &mut PeekableIterator<Token>, name: &str) -> bool {
    match iter.peek(0) {
        Some(Token::Symbol(s)) if s == name => {
            iter.next();
            true
        }
        _ => false,
    }
}

fn expect_identifier(
    iter: &mut PeekableIterator<Token>,
    for_what: &str,
) -> Result<String, ParseError> {
    match iter.next() {
        Some(token) => match token {
            Token::Identifier(s) => Ok(s),
            _ => Err(ParseError::new(&format!(
                "Expect a identifier for {}",
                for_what
            ))),
        },
        None => Err(ParseError::new(&format!(
            "Missing a identifier for {}",
            for_what
        ))),
    }
}

fn exist_child_node(iter: &mut PeekableIterator<Token>, child_node_name: &str) -> bool {
    if let Some(Token::LeftParen) = iter.peek(0) {
        matches!(iter.peek(1), Some(Token::Symbol(n)) if n == child_node_name)
    } else {
        false
    }
}

fn get_instruction_kind(inst_name: &str) -> Option<&InstructionSyntaxKind> {
    unsafe {
        if let Some(table_ref) = &INSTRUCTION_MAP {
            table_ref.get(inst_name)
        } else {
            panic!("The instruction table is not initialized yet.")
        }
    }
}

fn parse_u16_string(number_token: &NumberToken) -> Result<u16, ParseError> {
    let e = ParseError::new(&format!(
        "\"{:?}\" is not a valid 16-bit integer literal.",
        number_token
    ));

    let num = match number_token {
        NumberToken::Hex(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_'); // remove underscores
            u16::from_str_radix(&ns, 16).map_err(|_| e)?
        }
        NumberToken::Binary(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_');
            u16::from_str_radix(&ns, 2).map_err(|_| e)?
        }
        NumberToken::Decimal(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_');
            ns.as_str().parse::<i16>().map_err(|_| e)? as u16
        }
        NumberToken::HexFloat(_) => return Err(e),
    };

    Ok(num)
}

fn parse_u32_string(number_token: &NumberToken) -> Result<u32, ParseError> {
    let e = ParseError::new(&format!(
        "\"{:?}\" is not a valid 32-bit integer literal.",
        number_token
    ));

    let num = match number_token {
        NumberToken::Hex(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_'); // remove underscores
            u32::from_str_radix(&ns, 16).map_err(|_| e)?
        }
        NumberToken::Binary(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_');
            u32::from_str_radix(&ns, 2).map_err(|_| e)?
        }
        NumberToken::Decimal(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_');
            ns.as_str().parse::<i32>().map_err(|_| e)? as u32
        }
        NumberToken::HexFloat(_) => return Err(e),
    };

    Ok(num)
}

fn parse_u64_string(number_token: &NumberToken) -> Result<u64, ParseError> {
    let e = ParseError::new(&format!(
        "\"{:?}\" is not a valid 64-bit integer literal.",
        number_token
    ));

    let num = match number_token {
        NumberToken::Hex(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_'); // remove underscores
            u64::from_str_radix(&ns, 16).map_err(|_| e)?
        }
        NumberToken::Binary(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_');
            u64::from_str_radix(&ns, 2).map_err(|_| e)?
        }
        NumberToken::Decimal(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_');
            ns.as_str().parse::<i64>().map_err(|_| e)? as u64
        }
        NumberToken::HexFloat(_) => return Err(e),
    };

    Ok(num)
}

fn parse_f32_string(number_token: &NumberToken) -> Result<f32, ParseError> {
    let e = ParseError::new(&format!(
        "\"{:?}\" is not a valid 32-bit floating point literal.",
        number_token
    ));

    match number_token {
        NumberToken::HexFloat(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_'); // remove underscores
            hexfloat2::parse::<f32>(&ns).map_err(|_| e)
        }
        NumberToken::Decimal(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_');
            ns.as_str().parse::<f32>().map_err(|_| e)
        }
        NumberToken::Hex(_) => Err(e),
        NumberToken::Binary(_) => Err(e),
    }
}

fn parse_f64_string(number_token: &NumberToken) -> Result<f64, ParseError> {
    let e = ParseError::new(&format!(
        "\"{:?}\" is not a valid 64-bit floating point literal.",
        number_token
    ));

    match number_token {
        NumberToken::HexFloat(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_'); // remove underscores
            hexfloat2::parse::<f64>(&ns).map_err(|_| e)
        }
        NumberToken::Decimal(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_');
            ns.as_str().parse::<f64>().map_err(|_| e)
        }
        NumberToken::Hex(_) => Err(e),
        NumberToken::Binary(_) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use pretty_assertions::assert_eq;

    use ancvm_types::{
        opcode::Opcode, DataSectionType, DataType, ExternalLibraryType, MemoryDataType,
        ModuleShareType,
    };

    use crate::{
        ast::{
            BranchCase, DataKindNode, DataNode, ExternalFunctionNode, ExternalItem,
            ExternalLibraryNode, ExternalNode, FunctionNode, ImportDataNode, ImportFunctionNode,
            ImportItem, ImportModuleNode, ImportNode, InitedData, Instruction, LocalNode,
            ModuleElementNode, ModuleNode, ParamNode, UninitData,
        },
        lexer::lex,
        peekable_iterator::PeekableIterator,
        ParseError,
    };

    use super::parse;

    fn parse_from_str(s: &str) -> Result<ModuleNode, ParseError> {
        let mut chars = s.chars();
        let mut char_iter = PeekableIterator::new(&mut chars, 3);
        let mut tokens = lex(&mut char_iter)?.into_iter();
        let mut token_iter = PeekableIterator::new(&mut tokens, 2);
        parse(&mut token_iter)
    }

    fn parse_instructions_from_str(text: &str) -> Vec<Instruction> {
        let module_node = parse_from_str(text).unwrap();
        if let ModuleElementNode::FunctionNode(function_node) = &module_node.element_nodes[0] {
            function_node.code.clone()
        } else {
            panic!("Expect function node")
        }
    }

    fn noparams_nooperands(opcode: Opcode) -> Instruction {
        Instruction::NoParams {
            opcode,
            operands: vec![],
        }
    }

    #[test]
    fn test_parse_module_node() {
        assert_eq!(
            parse_from_str(r#"(module $app (runtime_version "1.0"))"#).unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                element_nodes: vec![]
            }
        );

        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.2")
                (constructor $abc)
                (destructor $xyz)
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 2,
                constructor_function_name_path: Some("abc".to_string()),
                destructor_function_name_path: Some("xyz".to_string()),
                element_nodes: vec![]
            }
        );

        // empty
        assert!(parse_from_str(r#"()"#).is_err());

        // missing essential nodes
        assert!(parse_from_str(r#"(module)"#).is_err());

        // missing node 'runtime'
        assert!(parse_from_str(r#"(module $app)"#).is_err());
        assert!(parse_from_str(r#"(module $app ())"#).is_err());

        // missing runtime number
        assert!(parse_from_str(r#"(module $app (runtime_version))"#).is_err());

        // error version number
        assert!(parse_from_str(r#"(module $app (runtime_version "1"))"#).is_err());
        assert!(parse_from_str(r#"(module $app (runtime_version "1a.2b"))"#).is_err());

        // define constructor or destructor in submodule
        assert!(parse_from_str(
            r#"(module $app::utils (runtime_version "1.0") (constructor $abc))"#
        )
        .is_err());
        assert!(parse_from_str(
            r#"(module $app::utils (runtime_version "1.0") (destructor $xyz))"#
        )
        .is_err());
    }

    #[test]
    fn test_parse_function_signature() {
        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (function $add (param $lhs i32) (param $rhs i64) (result i32) (result i64)
                    ;; no local variables
                    (code)
                )
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                element_nodes: vec![ModuleElementNode::FunctionNode(FunctionNode {
                    name: "add".to_owned(),
                    export: false,
                    params: vec![
                        ParamNode {
                            name: "lhs".to_owned(),
                            data_type: DataType::I32
                        },
                        ParamNode {
                            name: "rhs".to_owned(),
                            data_type: DataType::I64
                        }
                    ],
                    results: vec![DataType::I32, DataType::I64,],
                    locals: vec![],
                    code: vec![]
                })]
            }
        );

        // test multiple return values

        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (function $add (param $lhs i32) (param $rhs i64) (results i32 i64) (result f32) (result f64)
                    ;; no local variables
                    (code)
                )
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                element_nodes: vec![ModuleElementNode::FunctionNode(FunctionNode {
                    name: "add".to_owned(),
                    export: false,
                    params: vec![
                        ParamNode {
                            name: "lhs".to_owned(),
                            data_type: DataType::I32
                        },
                        ParamNode {
                            name: "rhs".to_owned(),
                            data_type: DataType::I64
                        }
                    ],
                    results: vec![DataType::I32, DataType::I64, DataType::F32, DataType::F64],
                    locals: vec![],
                    code: vec![]
                })]
            }
        );

        // test property 'export'

        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (function export $add (code))
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                element_nodes: vec![ModuleElementNode::FunctionNode(FunctionNode {
                    name: "add".to_owned(),
                    export: true,
                    params: vec![],
                    results: vec![],
                    locals: vec![],
                    code: vec![]
                })]
            }
        );

        // no function name
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (function (code))
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // function name contains path separator
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (function $a::b (code))
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // no function body
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (function $add)
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_parse_function_local_variables() {
        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (function $add
                    ;; no params and results
                    (local $sum i32) (local $count i64) (local $db (bytes 12 8)) (local $average f32)
                    (code)
                )
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                element_nodes: vec![ModuleElementNode::FunctionNode(FunctionNode {
                    name: "add".to_owned(),
                    export: false,
                    params: vec![],
                    results: vec![],
                    locals: vec![
                        LocalNode {
                            name: "sum".to_owned(),
                            memory_data_type: MemoryDataType::I32,
                            data_length: 4,
                            align: 4
                        },
                        LocalNode {
                            name: "count".to_owned(),
                            memory_data_type: MemoryDataType::I64,
                            data_length: 8,
                            align: 8
                        },
                        LocalNode {
                            name: "db".to_owned(),
                            memory_data_type: MemoryDataType::Bytes,
                            data_length: 12,
                            align: 8
                        },
                        LocalNode {
                            name: "average".to_owned(),
                            memory_data_type: MemoryDataType::F32,
                            data_length: 4,
                            align: 4
                        },
                    ],
                    code: vec![]
                })]
            }
        );

        // local vairable name contains path separator
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (function $test
                    (local $a::b i32)
                    (code)
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_parse_instructions_fundanmental() {
        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        nop
                        zero
                        (drop zero)
                        ;; (duplicate zero)
                        ;; (swap zero zero)
                        (select_nez zero zero zero)
                    )
                )
            )
            "#
            ),
            vec![
                noparams_nooperands(Opcode::nop),
                noparams_nooperands(Opcode::zero),
                Instruction::NoParams {
                    opcode: Opcode::drop,
                    operands: vec![noparams_nooperands(Opcode::zero),]
                },
                // Instruction::NoParams {
                //     opcode: Opcode::duplicate,
                //     operands: vec![noparams_nooperands(Opcode::zero),]
                // },
                // Instruction::NoParams {
                //     opcode: Opcode::swap,
                //     operands: vec![
                //         noparams_nooperands(Opcode::zero),
                //         noparams_nooperands(Opcode::zero)
                //     ]
                // },
                Instruction::NoParams {
                    opcode: Opcode::select_nez,
                    operands: vec![
                        noparams_nooperands(Opcode::zero),
                        noparams_nooperands(Opcode::zero),
                        noparams_nooperands(Opcode::zero),
                    ]
                }
            ]
        );

        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (i32.imm 11)
                        (i32.imm 0x13)
                        (i32.imm 17_19)
                        (i32.imm -23)
                        (i32.imm 0xaa_bb)
                        (i32.imm 0b0110_0101)    ;; 101

                        (i64.imm 31)
                        (i64.imm 0x37)
                        (i64.imm 41_43)
                        (i64.imm -47)
                        (i64.imm 0xaabb_ccdd)
                        (i64.imm 0b0110_0111)   ;; 103
                    )
                )
            )
            "#
            ),
            vec![
                Instruction::ImmI32(11),
                Instruction::ImmI32(0x13),
                Instruction::ImmI32(17_19),
                Instruction::ImmI32((-23i32) as u32),
                Instruction::ImmI32(0xaa_bb),
                Instruction::ImmI32(0b0110_0101),
                Instruction::ImmI64(31),
                Instruction::ImmI64(0x37),
                Instruction::ImmI64(41_43),
                Instruction::ImmI64((-47i64) as u64),
                Instruction::ImmI64(0xaabb_ccdd),
                Instruction::ImmI64(0b0110_0111),
            ]
        );

        // float consts:
        //
        // - PI     f32     0x40490fdb          3.1415927
        // - E      f32     0x402df854          2.7182817
        // - PI     f64     0x400921fb54442d18  3.141592653589793
        // - E      f64     0x4005bf0a8b145769  2.718281828459045

        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (f32.imm 3.1415927)
                        (f32.imm 0x1.921fb6p1)
                        (f32.imm 2.7182817)
                        (f32.imm 0x1.5bf0a8p1)

                        (f64.imm 3.141592653589793)
                        (f64.imm 0x1.921fb54442d18p1)
                        (f64.imm 2.718281828459045)
                        (f64.imm 0x1.5bf0a8b145769p1)
                    )
                )
            )
            "#
            ),
            vec![
                Instruction::ImmF32(std::f32::consts::PI),
                Instruction::ImmF32(std::f32::consts::PI),
                Instruction::ImmF32(std::f32::consts::E),
                Instruction::ImmF32(std::f32::consts::E),
                //
                Instruction::ImmF64(std::f64::consts::PI),
                Instruction::ImmF64(std::f64::consts::PI),
                Instruction::ImmF64(std::f64::consts::E),
                Instruction::ImmF64(std::f64::consts::E),
            ]
        );
    }

    #[test]
    fn test_parse_instructions_unaryop_and_binaryop() {
        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (i32.eqz (i32.imm 11))
                        (i32.inc 1 (i32.imm 13))
                        (i32.add (i32.imm 17) (i32.imm 19))
                        (i32.add
                            (i32.mul
                                (i32.imm 2)
                                (i32.imm 3)
                            )
                            (i32.imm 1)
                        )
                    )
                )
            )
            "#
            ),
            vec![
                // 11 == 0
                Instruction::UnaryOp {
                    opcode: Opcode::i32_eqz,
                    source: Box::new(Instruction::ImmI32(11))
                },
                // 13 + 1
                Instruction::UnaryOpWithImmI16 {
                    opcode: Opcode::i32_inc,
                    imm: 1,
                    source: Box::new(Instruction::ImmI32(13))
                },
                // 17 + 19
                Instruction::BinaryOp {
                    opcode: Opcode::i32_add,
                    left: Box::new(Instruction::ImmI32(17)),
                    right: Box::new(Instruction::ImmI32(19))
                },
                // (2 * 3) + 1
                Instruction::BinaryOp {
                    opcode: Opcode::i32_add,
                    left: Box::new(Instruction::BinaryOp {
                        opcode: Opcode::i32_mul,
                        left: Box::new(Instruction::ImmI32(2)),
                        right: Box::new(Instruction::ImmI32(3))
                    }),
                    right: Box::new(Instruction::ImmI32(1)),
                },
            ]
        );
    }

    #[test]
    fn test_parse_instructions_local() {
        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (local.load32_i32 $sum)
                        (local.load64_i64 $count 4)
                        (local.store32 $left (i32.imm 11))
                        (local.store64 $right 8 (i64.imm 13))
                        (local.long_load64_i64 $foo (i32.imm 17))
                        (local.long_store64 $bar (i32.imm 19) (i64.imm 23))
                    )
                )
            )
            "#
            ),
            vec![
                Instruction::LocalLoad {
                    opcode: Opcode::local_load32_i32,
                    name: "sum".to_owned(),
                    offset: 0
                },
                Instruction::LocalLoad {
                    opcode: Opcode::local_load64_i64,
                    name: "count".to_owned(),
                    offset: 4
                },
                //
                Instruction::LocalStore {
                    opcode: Opcode::local_store32,
                    name: "left".to_owned(),
                    offset: 0,
                    value: Box::new(Instruction::ImmI32(11))
                },
                //
                Instruction::LocalStore {
                    opcode: Opcode::local_store64,
                    name: "right".to_owned(),
                    offset: 8,
                    value: Box::new(Instruction::ImmI64(13))
                },
                //
                Instruction::LocalLongLoad {
                    opcode: Opcode::local_long_load64_i64,
                    name: "foo".to_owned(),
                    offset: Box::new(Instruction::ImmI32(17))
                },
                //
                Instruction::LocalLongStore {
                    opcode: Opcode::local_long_store64,
                    name: "bar".to_owned(),
                    offset: Box::new(Instruction::ImmI32(19)),
                    value: Box::new(Instruction::ImmI64(23))
                },
            ]
        );
    }

    #[test]
    fn test_parse_instructions_data() {
        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (data.load32_i32 $sum)
                        (data.load64_i64 $count 4)
                        (data.store32 $left (i32.imm 11))
                        (data.store64 $right 8 (i64.imm 13))
                        (data.long_load64_i64 $foo (i32.imm 17))
                        (data.long_store64 $bar (i32.imm 19) (i64.imm 23))
                    )
                )
            )
            "#
            ),
            vec![
                Instruction::DataLoad {
                    opcode: Opcode::data_load32_i32,
                    id: "sum".to_owned(),
                    offset: 0
                },
                Instruction::DataLoad {
                    opcode: Opcode::data_load64_i64,
                    id: "count".to_owned(),
                    offset: 4
                },
                //
                Instruction::DataStore {
                    opcode: Opcode::data_store32,
                    id: "left".to_owned(),
                    offset: 0,
                    value: Box::new(Instruction::ImmI32(11))
                },
                //
                Instruction::DataStore {
                    opcode: Opcode::data_store64,
                    id: "right".to_owned(),
                    offset: 8,
                    value: Box::new(Instruction::ImmI64(13))
                },
                //
                Instruction::DataLongLoad {
                    opcode: Opcode::data_long_load64_i64,
                    id: "foo".to_owned(),
                    offset: Box::new(Instruction::ImmI32(17))
                },
                //
                Instruction::DataLongStore {
                    opcode: Opcode::data_long_store64,
                    id: "bar".to_owned(),
                    offset: Box::new(Instruction::ImmI32(19)),
                    value: Box::new(Instruction::ImmI64(23))
                },
            ]
        );
    }

    #[test]
    fn test_parse_instructions_heap() {
        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (heap.load32_i32 (i32.imm 11))
                        (heap.load64_i64 4 (i32.imm 13))
                        (heap.store32 (i32.imm 17) (i32.imm 19))
                        (heap.store64 8 (i32.imm 23) (i32.imm 29))
                    )
                )
            )
            "#
            ),
            vec![
                Instruction::HeapLoad {
                    opcode: Opcode::heap_load32_i32,
                    offset: 0,
                    addr: Box::new(Instruction::ImmI32(11))
                },
                Instruction::HeapLoad {
                    opcode: Opcode::heap_load64_i64,
                    offset: 4,
                    addr: Box::new(Instruction::ImmI32(13))
                },
                //
                Instruction::HeapStore {
                    opcode: Opcode::heap_store32,
                    offset: 0,
                    addr: Box::new(Instruction::ImmI32(17)),
                    value: Box::new(Instruction::ImmI32(19))
                },
                //
                Instruction::HeapStore {
                    opcode: Opcode::heap_store64,
                    offset: 8,
                    addr: Box::new(Instruction::ImmI32(23)),
                    value: Box::new(Instruction::ImmI32(29))
                },
            ]
        );
    }

    #[test]
    fn test_parse_instructions_when() {
        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (when
                            (i32.eq (i32.imm 11) (i32.imm 13))
                            (nop)
                        )
                    )
                )
            )
            "#
            ),
            vec![Instruction::When {
                // locals: vec![],
                test: Box::new(Instruction::BinaryOp {
                    opcode: Opcode::i32_eq,
                    left: Box::new(Instruction::ImmI32(11)),
                    right: Box::new(Instruction::ImmI32(13))
                }),
                consequent: Box::new(noparams_nooperands(Opcode::nop))
            }]
        );

        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (when
                            zero
                            (do (local.load32_i32 $abc) (local.load32_i32 $xyz))
                        )
                    )
                )
            )
            "#
            ),
            vec![Instruction::When {
                test: Box::new(noparams_nooperands(Opcode::zero)),
                consequent: Box::new(Instruction::Do(vec![
                    Instruction::LocalLoad {
                        opcode: Opcode::local_load32_i32,
                        name: "abc".to_owned(),
                        offset: 0
                    },
                    Instruction::LocalLoad {
                        opcode: Opcode::local_load32_i32,
                        name: "xyz".to_owned(),
                        offset: 0
                    }
                ]))
            }]
        );

        // contains params
        assert!(matches!(
            parse_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (when (param $a i32) zero zero)
                    )
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // contains results
        assert!(matches!(
            parse_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (when (result i32) zero zero)
                    )
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // contains local vars
        assert!(matches!(
            parse_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (when (local $a i32) zero zero)
                    )
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_parse_instructions_if() {
        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (if
                            (i32.eq (i32.imm 11) (i32.imm 13))
                            nop
                            zero
                        )
                    )
                )
            )
            "#
            ),
            vec![Instruction::If {
                results: vec![],
                test: Box::new(Instruction::BinaryOp {
                    opcode: Opcode::i32_eq,
                    left: Box::new(Instruction::ImmI32(11)),
                    right: Box::new(Instruction::ImmI32(13))
                }),
                consequent: Box::new(noparams_nooperands(Opcode::nop)),
                alternate: Box::new(noparams_nooperands(Opcode::zero))
            }]
        );

        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (if
                            (result i32)
                            (i32.eq (local.load32_i32 $m) (local.load32_i32 $n))
                            (i32.add (i32.imm 11) (local.load32_i32 $x))
                            (i32.mul (i32.imm 13) (local.load32_i32 $x))
                        )
                    )
                )
            )
            "#
            ),
            vec![Instruction::If {
                results: vec![DataType::I32],
                test: Box::new(Instruction::BinaryOp {
                    opcode: Opcode::i32_eq,
                    left: Box::new(Instruction::LocalLoad {
                        opcode: Opcode::local_load32_i32,
                        name: "m".to_owned(),
                        offset: 0
                    }),
                    right: Box::new(Instruction::LocalLoad {
                        opcode: Opcode::local_load32_i32,
                        name: "n".to_owned(),
                        offset: 0
                    })
                }),
                consequent: Box::new(Instruction::BinaryOp {
                    opcode: Opcode::i32_add,
                    left: Box::new(Instruction::ImmI32(11)),
                    right: Box::new(Instruction::LocalLoad {
                        opcode: Opcode::local_load32_i32,
                        name: "x".to_owned(),
                        offset: 0
                    })
                }),
                alternate: Box::new(Instruction::BinaryOp {
                    opcode: Opcode::i32_mul,
                    left: Box::new(Instruction::ImmI32(13)),
                    right: Box::new(Instruction::LocalLoad {
                        opcode: Opcode::local_load32_i32,
                        name: "x".to_owned(),
                        offset: 0
                    })
                })
            }]
        );

        // contains params
        assert!(matches!(
            parse_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (if
                            (param $a i32)
                            zero zero zero
                        )
                    )
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // contains local vars
        assert!(matches!(
            parse_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (if
                            (local $a i32)
                            zero zero zero
                        )
                    )
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_parse_instructions_branch() {
        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (branch
                            (result i32)
                            (case
                                (i32.gt_s (local.load32_i32 $x) (i32.imm 11))
                                (i32.imm 13)
                            )
                            (case
                                (i32.not zero)
                                (i32.imm 17)
                            )
                            (default
                                (i32.imm 19)
                            )
                        )
                    )
                )
            )
            "#
            ),
            vec![Instruction::Branch {
                results: vec![DataType::I32],
                cases: vec![
                    BranchCase {
                        test: Box::new(Instruction::BinaryOp {
                            opcode: Opcode::i32_gt_s,
                            left: Box::new(Instruction::LocalLoad {
                                opcode: Opcode::local_load32_i32,
                                name: "x".to_owned(),
                                offset: 0
                            }),
                            right: Box::new(Instruction::ImmI32(11))
                        }),
                        consequent: Box::new(Instruction::ImmI32(13))
                    },
                    BranchCase {
                        test: Box::new(Instruction::UnaryOp {
                            opcode: Opcode::i32_not,
                            source: Box::new(noparams_nooperands(Opcode::zero))
                        }),
                        consequent: Box::new(Instruction::ImmI32(17))
                    }
                ],
                default: Some(Box::new(Instruction::ImmI32(19)))
            }]
        );

        // contains params
        assert!(matches!(
            parse_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (branch
                            (param $a i32)
                            (case zero zero)
                        )
                    )
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // contains local vars
        assert!(matches!(
            parse_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (branch
                            (local $a i32)
                            (case zero zero)
                        )
                    )
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_parse_instructions_for() {
        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        (for (param $sum i32) (param $n i32) (result i32) (local $temp i32)
                            (do
                                ;; n = n - 1
                                (local.store32 $n (i32.dec 1 (local.load32_i32 $n)))
                                (if
                                    ;; if n == 0
                                    (i32.eq (local.load32_i32 $n) zero)
                                    ;; then
                                    (break (local.load32_i32 $sum))
                                    ;; else
                                    (do
                                        ;; sum = sum + n
                                        (local.store32 $sum (i32.add
                                            (local.load32_i32 $sum)
                                            (local.load32_i32 $n)
                                        ))
                                        ;; recur (sum,n)
                                        (recur
                                            (local.load32_i32 $sum)
                                            (local.load32_i32 $n)
                                        )
                                    )
                                )
                            )
                        )
                    )
                )
            )
            "#
            ),
            vec![Instruction::For {
                params: vec![
                    ParamNode {
                        name: "sum".to_owned(),
                        data_type: DataType::I32
                    },
                    ParamNode {
                        name: "n".to_owned(),
                        data_type: DataType::I32
                    },
                ],
                results: vec![DataType::I32],
                locals: vec![LocalNode {
                    name: "temp".to_owned(),
                    memory_data_type: MemoryDataType::I32,
                    data_length: 4,
                    align: 4
                }],
                code: Box::new(Instruction::Do(vec![
                    Instruction::LocalStore {
                        opcode: Opcode::local_store32,
                        name: "n".to_owned(),
                        offset: 0,
                        value: Box::new(Instruction::UnaryOpWithImmI16 {
                            opcode: Opcode::i32_dec,
                            imm: 1,
                            source: Box::new(Instruction::LocalLoad {
                                opcode: Opcode::local_load32_i32,
                                name: "n".to_owned(),
                                offset: 0
                            })
                        })
                    },
                    Instruction::If {
                        results: vec![],
                        test: Box::new(Instruction::BinaryOp {
                            opcode: Opcode::i32_eq,
                            left: Box::new(Instruction::LocalLoad {
                                opcode: Opcode::local_load32_i32,
                                name: "n".to_owned(),
                                offset: 0
                            }),
                            right: Box::new(noparams_nooperands(Opcode::zero))
                        }),
                        consequent: Box::new(Instruction::Break(vec![Instruction::LocalLoad {
                            opcode: Opcode::local_load32_i32,
                            name: "sum".to_owned(),
                            offset: 0
                        }])),
                        alternate: Box::new(Instruction::Do(vec![
                            Instruction::LocalStore {
                                opcode: Opcode::local_store32,
                                name: "sum".to_owned(),
                                offset: 0,
                                value: Box::new(Instruction::BinaryOp {
                                    opcode: Opcode::i32_add,
                                    left: Box::new(Instruction::LocalLoad {
                                        opcode: Opcode::local_load32_i32,
                                        name: "sum".to_owned(),
                                        offset: 0
                                    }),
                                    right: Box::new(Instruction::LocalLoad {
                                        opcode: Opcode::local_load32_i32,
                                        name: "n".to_owned(),
                                        offset: 0
                                    })
                                })
                            },
                            Instruction::Recur(vec![
                                Instruction::LocalLoad {
                                    opcode: Opcode::local_load32_i32,
                                    name: "sum".to_owned(),
                                    offset: 0
                                },
                                Instruction::LocalLoad {
                                    opcode: Opcode::local_load32_i32,
                                    name: "n".to_owned(),
                                    offset: 0
                                }
                            ])
                        ]))
                    }
                ]))
            }]
        );
    }

    #[test]
    fn test_parse_instructions_return_and_rerun() {
        assert_eq!(
            parse_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test (param $sum i32) (param $n i32) (result i32)
                    (code
                        ;; n = n - 1
                        (local.store32 $n (i32.dec 1 (local.load32_i32 $n)))
                        (if
                            ;; if n == 0
                            (i32.eq (local.load32_i32 $n) zero)
                            ;; then
                            (return (local.load32_i32 $sum))
                            ;; else
                            (do
                                ;; sum = sum + n
                                (local.store32 $sum (i32.add
                                    (local.load32_i32 $sum)
                                    (local.load32_i32 $n)
                                ))
                                ;; recur (sum,n)
                                (rerun
                                    (local.load32_i32 $sum)
                                    (local.load32_i32 $n)
                                )
                            )
                        )
                    )
                )
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "lib".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                element_nodes: vec![ModuleElementNode::FunctionNode(FunctionNode {
                    name: "test".to_owned(),
                    export: false,
                    params: vec![
                        ParamNode {
                            name: "sum".to_owned(),
                            data_type: DataType::I32
                        },
                        ParamNode {
                            name: "n".to_owned(),
                            data_type: DataType::I32
                        },
                    ],
                    results: vec![DataType::I32],
                    locals: vec![],
                    code: vec![
                        Instruction::LocalStore {
                            opcode: Opcode::local_store32,
                            name: "n".to_owned(),
                            offset: 0,
                            value: Box::new(Instruction::UnaryOpWithImmI16 {
                                opcode: Opcode::i32_dec,
                                imm: 1,
                                source: Box::new(Instruction::LocalLoad {
                                    opcode: Opcode::local_load32_i32,
                                    name: "n".to_owned(),
                                    offset: 0
                                })
                            })
                        },
                        Instruction::If {
                            results: vec![],
                            test: Box::new(Instruction::BinaryOp {
                                opcode: Opcode::i32_eq,
                                left: Box::new(Instruction::LocalLoad {
                                    opcode: Opcode::local_load32_i32,
                                    name: "n".to_owned(),
                                    offset: 0
                                }),
                                right: Box::new(noparams_nooperands(Opcode::zero))
                            }),
                            consequent: Box::new(Instruction::Return(vec![
                                Instruction::LocalLoad {
                                    opcode: Opcode::local_load32_i32,
                                    name: "sum".to_owned(),
                                    offset: 0
                                }
                            ])),
                            alternate: Box::new(Instruction::Do(vec![
                                Instruction::LocalStore {
                                    opcode: Opcode::local_store32,
                                    name: "sum".to_owned(),
                                    offset: 0,
                                    value: Box::new(Instruction::BinaryOp {
                                        opcode: Opcode::i32_add,
                                        left: Box::new(Instruction::LocalLoad {
                                            opcode: Opcode::local_load32_i32,
                                            name: "sum".to_owned(),
                                            offset: 0
                                        }),
                                        right: Box::new(Instruction::LocalLoad {
                                            opcode: Opcode::local_load32_i32,
                                            name: "n".to_owned(),
                                            offset: 0
                                        })
                                    })
                                },
                                Instruction::Rerun(vec![
                                    Instruction::LocalLoad {
                                        opcode: Opcode::local_load32_i32,
                                        name: "sum".to_owned(),
                                        offset: 0
                                    },
                                    Instruction::LocalLoad {
                                        opcode: Opcode::local_load32_i32,
                                        name: "n".to_owned(),
                                        offset: 0
                                    }
                                ])
                            ]))
                        }
                    ]
                })]
            }
        );
    }

    #[test]
    fn test_parse_instructions_all_sorts_0f_calling() {
        // test 'call', 'dyncall', 'envcall', 'syscall' and 'extcall'
        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        ;; call: add(11, 13)
                        (call $add (i32.imm 11) (i32.imm 13))

                        ;; dyncall: filter(data)
                        (dyncall (local.load32_i32 $filter) (local.load64_i64 $data))

                        ;; envcall: runtime_name(buf)
                        (envcall 0x100 (local.load64_i64 $buf))

                        ;; syscall: write(1, msg, 7)
                        (syscall 2 (i32.imm 1) (local.load64_i64 $msg) (i32.imm 7))

                        ;; extcall: format(str, values)
                        (extcall $format (local.load64_i64 $str) (local.load64_i64 $values))

                        ;; get the public index of the specified function
                        (macro.get_function_public_index $add)
                    )
                )
            )
            "#
            ),
            vec![
                Instruction::Call {
                    id: "add".to_owned(),
                    args: vec![Instruction::ImmI32(11), Instruction::ImmI32(13),]
                },
                Instruction::DynCall {
                    public_index: Box::new(Instruction::LocalLoad {
                        opcode: Opcode::local_load32_i32,
                        name: "filter".to_owned(),
                        offset: 0
                    }),
                    args: vec![Instruction::LocalLoad {
                        opcode: Opcode::local_load64_i64,
                        name: "data".to_owned(),
                        offset: 0
                    }]
                },
                Instruction::EnvCall {
                    num: 0x100,
                    args: vec![Instruction::LocalLoad {
                        opcode: Opcode::local_load64_i64,
                        name: "buf".to_owned(),
                        offset: 0
                    }]
                },
                Instruction::SysCall {
                    num: 2,
                    args: vec![
                        Instruction::ImmI32(1),
                        Instruction::LocalLoad {
                            opcode: Opcode::local_load64_i64,
                            name: "msg".to_owned(),
                            offset: 0
                        },
                        Instruction::ImmI32(7),
                    ]
                },
                Instruction::ExtCall {
                    id: "format".to_owned(),
                    args: vec![
                        Instruction::LocalLoad {
                            opcode: Opcode::local_load64_i64,
                            name: "str".to_owned(),
                            offset: 0
                        },
                        Instruction::LocalLoad {
                            opcode: Opcode::local_load64_i64,
                            name: "values".to_owned(),
                            offset: 0
                        }
                    ]
                },
                Instruction::MacroGetFunctionPublicIndex {
                    id: "add".to_owned()
                },
            ]
        );
    }

    #[test]
    fn test_parse_instructions_host() {
        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (function $test
                    (code
                        panic
                        (unreachable 0x11)
                        (debug 0x13)
                        (host.addr_local $num 0x17)
                        (host.addr_local_long $sum (i32.imm 0x19))
                        (host.addr_data $msg 0x23)
                        (host.addr_data_long $title (i32.imm 0x29))
                        (host.addr_heap 0x31 (i32.imm 0x37))
                        (host.addr_function $add)
                        (host.copy_heap_to_memory
                            (i32.imm 0x41)
                            (i32.imm 0x43)
                            (i32.imm 0x47)
                        )
                        (host.copy_memory_to_heap
                            (i32.imm 0x53)
                            (i32.imm 0x59)
                            (i32.imm 0x61)
                        )
                    )
                )
            )
            "#
            ),
            vec![
                noparams_nooperands(Opcode::panic),
                Instruction::Unreachable { code: 0x11 },
                Instruction::Debug { code: 0x13 },
                Instruction::LocalLoad {
                    opcode: Opcode::host_addr_local,
                    name: "num".to_owned(),
                    offset: 0x17
                },
                Instruction::LocalLongLoad {
                    opcode: Opcode::host_addr_local_long,
                    name: "sum".to_owned(),
                    offset: Box::new(Instruction::ImmI32(0x19)),
                },
                Instruction::DataLoad {
                    opcode: Opcode::host_addr_data,
                    id: "msg".to_owned(),
                    offset: 0x23,
                },
                Instruction::DataLongLoad {
                    opcode: Opcode::host_addr_data_long,
                    id: "title".to_owned(),
                    offset: Box::new(Instruction::ImmI32(0x29)),
                },
                Instruction::HeapLoad {
                    opcode: Opcode::host_addr_heap,
                    offset: 0x31,
                    addr: Box::new(Instruction::ImmI32(0x37)),
                },
                Instruction::HostAddrFunction {
                    id: "add".to_owned(),
                },
                Instruction::NoParams {
                    opcode: Opcode::host_copy_heap_to_memory,
                    operands: vec![
                        Instruction::ImmI32(0x41),
                        Instruction::ImmI32(0x43),
                        Instruction::ImmI32(0x47),
                    ],
                },
                Instruction::NoParams {
                    opcode: Opcode::host_copy_memory_to_heap,
                    operands: vec![
                        Instruction::ImmI32(0x53),
                        Instruction::ImmI32(0x59),
                        Instruction::ImmI32(0x61),
                    ],
                },
            ]
        );
    }

    #[test]
    fn test_parse_data_read_only_and_read_write() {
        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (data $d0 (read_only i32 123))
                (data $d1 (read_only i64 123_456))
                (data $d2 (read_only f32 3.1415927))
                (data $d3 (read_only f64 2.718281828459045))
                (data $d4 (read_only i32 0xaabb_ccdd))
                (data $d5 (read_only f32 0x1.921fb6p1))
                (data $d6 (read_only i32 0b1010_0101))
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                element_nodes: vec![
                    ModuleElementNode::DataNode(DataNode {
                        name: "d0".to_owned(),
                        export: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4,
                            value: 123u32.to_le_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d1".to_owned(),
                        export: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::I64,
                            length: 8,
                            align: 8,
                            value: 123_456u64.to_le_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d2".to_owned(),
                        export: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::F32,
                            length: 4,
                            align: 4,
                            value: std::f32::consts::PI.to_le_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d3".to_owned(),
                        export: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::F64,
                            length: 8,
                            align: 8,
                            value: std::f64::consts::E.to_le_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d4".to_owned(),
                        export: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4,
                            value: 0xaabb_ccddu32.to_le_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d5".to_owned(),
                        export: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::F32,
                            length: 4,
                            align: 4,
                            value: std::f32::consts::PI.to_le_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d6".to_owned(),
                        export: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4,
                            value: 0b1010_0101u32.to_le_bytes().to_vec()
                        })
                    }),
                ]
            }
        );

        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (data $d0 (read_only string "Hello, World!"))
                (data $d1 (read_only cstring "Hello, World!"))
                (data $d2 (read_only (bytes 2) h"11-13-17-19"))
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                element_nodes: vec![
                    ModuleElementNode::DataNode(DataNode {
                        name: "d0".to_owned(),
                        export: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::Bytes,
                            length: 13,
                            align: 1,
                            value: "Hello, World!".as_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d1".to_owned(),
                        export: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::Bytes,
                            length: 14,
                            align: 1,
                            value: "Hello, World!\0".as_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d2".to_owned(),
                        export: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::Bytes,
                            length: 4,
                            align: 2,
                            value: [0x11, 0x13, 0x17, 0x19].to_vec()
                        })
                    }),
                ]
            }
        );

        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (data export $d0 (read_only i32 123))
                (data export $d1 (read_write i32 456))
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                element_nodes: vec![
                    ModuleElementNode::DataNode(DataNode {
                        name: "d0".to_owned(),
                        export: true,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4,
                            value: 123u32.to_le_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d1".to_owned(),
                        export: true,
                        data_kind: DataKindNode::ReadWrite(InitedData {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4,
                            value: 456u32.to_le_bytes().to_vec()
                        })
                    }),
                ]
            }
        );

        // unknown data kind
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (data $d0 (write_only i32 123))
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // missing data name
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (data (read_only i32 123))
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // data name contains path separator
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (data $a::b (read_only i32 123))
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // missing value
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (data $d0 (read_only i32))
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // 'bytes' should be written as a node
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (data $d0 (read_only bytes 2 h"11-13-17-19"))
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // missing align
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (data $d0 (read_only (bytes) h"11-13-17-19"))
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_parse_data_uninit() {
        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (data $d0 (uninit i32))
                (data $d1 (uninit i64))
                (data $d2 (uninit f32))
                (data $d3 (uninit f64))
                (data $d4 (uninit (bytes 12 4)))
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                element_nodes: vec![
                    ModuleElementNode::DataNode(DataNode {
                        name: "d0".to_owned(),
                        export: false,
                        data_kind: DataKindNode::Uninit(UninitData {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4,
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d1".to_owned(),
                        export: false,
                        data_kind: DataKindNode::Uninit(UninitData {
                            memory_data_type: MemoryDataType::I64,
                            length: 8,
                            align: 8,
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d2".to_owned(),
                        export: false,
                        data_kind: DataKindNode::Uninit(UninitData {
                            memory_data_type: MemoryDataType::F32,
                            length: 4,
                            align: 4,
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d3".to_owned(),
                        export: false,
                        data_kind: DataKindNode::Uninit(UninitData {
                            memory_data_type: MemoryDataType::F64,
                            length: 8,
                            align: 8,
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d4".to_owned(),
                        export: false,
                        data_kind: DataKindNode::Uninit(UninitData {
                            memory_data_type: MemoryDataType::Bytes,
                            length: 12,
                            align: 4,
                        })
                    }),
                ]
            }
        );

        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (data export $d0 (uninit i32))
                (data export $d1 (uninit i64))
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                element_nodes: vec![
                    ModuleElementNode::DataNode(DataNode {
                        name: "d0".to_owned(),
                        export: true,
                        data_kind: DataKindNode::Uninit(UninitData {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4,
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d1".to_owned(),
                        export: true,
                        data_kind: DataKindNode::Uninit(UninitData {
                            memory_data_type: MemoryDataType::I64,
                            length: 8,
                            align: 8,
                        })
                    }),
                ]
            }
        );

        // contains value
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (data $d0 (uninit i32 123))
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // missing align
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (data $d0 (uninit (bytes 12)))
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_parse_external_node() {
        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (external (library share "math.so.1")
                    (function $add "add" (param i32) (param i32) (result i32))
                    (function $sub_i32 "sub" (params i32 i32) (result i32))
                    (function $pause "pause_1s")
                )
                (external (library system "libc.so.6")
                    (function $getuid "getuid" (result i32))
                    (function $getenv "getenv" (param (;name;) i64) (result i64))
                )
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                element_nodes: vec![
                    ModuleElementNode::ExternalNode(ExternalNode {
                        external_library_node: ExternalLibraryNode {
                            external_library_type: ExternalLibraryType::Share,
                            name: "math.so.1".to_owned()
                        },
                        external_items: vec![
                            ExternalItem::ExternalFunction(ExternalFunctionNode {
                                id: "add".to_owned(),
                                name: "add".to_owned(),
                                params: vec![DataType::I32, DataType::I32,],
                                results: vec![DataType::I32]
                            }),
                            ExternalItem::ExternalFunction(ExternalFunctionNode {
                                id: "sub_i32".to_owned(),
                                name: "sub".to_owned(),
                                params: vec![DataType::I32, DataType::I32,],
                                results: vec![DataType::I32]
                            }),
                            ExternalItem::ExternalFunction(ExternalFunctionNode {
                                id: "pause".to_owned(),
                                name: "pause_1s".to_owned(),
                                params: vec![],
                                results: vec![]
                            })
                        ]
                    }),
                    ModuleElementNode::ExternalNode(ExternalNode {
                        external_library_node: ExternalLibraryNode {
                            external_library_type: ExternalLibraryType::System,
                            name: "libc.so.6".to_owned()
                        },
                        external_items: vec![
                            ExternalItem::ExternalFunction(ExternalFunctionNode {
                                id: "getuid".to_owned(),
                                name: "getuid".to_owned(),
                                params: vec![],
                                results: vec![DataType::I32]
                            }),
                            ExternalItem::ExternalFunction(ExternalFunctionNode {
                                id: "getenv".to_owned(),
                                name: "getenv".to_owned(),
                                params: vec![DataType::I64],
                                results: vec![DataType::I64]
                            })
                        ]
                    }),
                ]
            }
        );

        // missing library node
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (external
                    (function $add "add")
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // unsupported library type
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (external
                    (library custom "libabc.so")
                    (function $add "add")
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // missing fn identifier
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (external
                    (library user "libabc.so")
                    (function "add")
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // fn name contains path separator
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (external
                    (library user "libabc.so")
                    (function $a::b "add")
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // missing fn symbol
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (external
                    (library user "libabc.so")
                    (function $add)
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_parse_import_node() {
        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (import (module share "math" "1.0")
                    (function $add "add" (param i32) (param i32) (result i32))
                    (function $add_wrap "wrap::add" (params i32 i32) (results i32))
                )
                (import (module user "format" "1.2")
                    (data $msg "msg" i32 read_only)
                    (data $sum "sum" i64 read_write)
                    (data $buf "utils::buf" bytes uninit)
                )
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                constructor_function_name_path: None,
                destructor_function_name_path: None,
                element_nodes: vec![
                    ModuleElementNode::ImportNode(ImportNode {
                        import_module_node: ImportModuleNode {
                            module_share_type: ModuleShareType::Share,
                            name: "math".to_owned(),
                            version_major: 1,
                            version_minor: 0
                        },
                        import_items: vec![
                            ImportItem::ImportFunction(ImportFunctionNode {
                                id: "add".to_owned(),
                                name_path: "add".to_owned(),
                                params: vec![DataType::I32, DataType::I32,],
                                results: vec![DataType::I32]
                            }),
                            ImportItem::ImportFunction(ImportFunctionNode {
                                id: "add_wrap".to_owned(),
                                name_path: "wrap::add".to_owned(),
                                params: vec![DataType::I32, DataType::I32,],
                                results: vec![DataType::I32]
                            }),
                        ]
                    }),
                    ModuleElementNode::ImportNode(ImportNode {
                        import_module_node: ImportModuleNode {
                            module_share_type: ModuleShareType::User,
                            name: "format".to_owned(),
                            version_major: 1,
                            version_minor: 2
                        },
                        import_items: vec![
                            ImportItem::ImportData(ImportDataNode {
                                id: "msg".to_owned(),
                                name_path: "msg".to_owned(),
                                memory_data_type: MemoryDataType::I32,
                                data_section_type: DataSectionType::ReadOnly
                            }),
                            ImportItem::ImportData(ImportDataNode {
                                id: "sum".to_owned(),
                                name_path: "sum".to_owned(),
                                memory_data_type: MemoryDataType::I64,
                                data_section_type: DataSectionType::ReadWrite
                            }),
                            ImportItem::ImportData(ImportDataNode {
                                id: "buf".to_owned(),
                                name_path: "utils::buf".to_owned(),
                                memory_data_type: MemoryDataType::Bytes,
                                data_section_type: DataSectionType::Uninit
                            })
                        ]
                    }),
                ]
            }
        );

        // missing library node
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (external
                    (function $add "add")
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // unsupported library type
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (external
                    (library custom "libabc.so")
                    (function $add "add")
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // missing fn identifier
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (external
                    (library user "libabc.so")
                    (function "add")
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // fn name contains path separator
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (external
                    (library user "libabc.so")
                    (function $a::b "add")
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));

        // missing fn symbol
        assert!(matches!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (external
                    (library user "libabc.so")
                    (function $add)
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }
}
