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
//    (fn $name (param $lhs i32) (param $rhs i32) (result i32)
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

use ancvm_types::{opcode::Opcode, DataType, ExternalLibraryType, MemoryDataType};

use crate::{
    ast::{
        BranchCase, DataKindNode, DataNode, ExternNode, ExternalFuncNode, ExternalItem,
        ExternalLibraryNode, FuncNode, ImmF32, ImmF64, InitedData, Instruction, LocalNode,
        ModuleElementNode, ModuleNode, ParamNode, UninitData,
    },
    instruction_kind::{init_instruction_kind_table, InstructionKind, INSTRUCTION_KIND_TABLE},
    lexer::{NumberToken, Token},
    peekable_iterator::PeekableIterator,
    ParseError,
};

pub fn parse(iter: &mut PeekableIterator<Token>) -> Result<ModuleNode, ParseError> {
    // initialize the instruction kind table
    init_instruction_kind_table();

    // there is only one node 'module' in a assembly text
    parse_module_node(iter)
}

pub fn parse_module_node(iter: &mut PeekableIterator<Token>) -> Result<ModuleNode, ParseError> {
    // (module ...) ...  //
    // ^            ^____// to here
    // |_________________// current token, i.e. the value of 'iter.peek(0)'

    // the node 'module' syntax:
    //
    // (module $name (runtime_version "1.0") ...)  ;; base
    // (module $name (runtime_version "1.0")
    //                                          ;; optional parameters
    //      (constructor $func_name)            ;; similar to GCC '__attribute__((constructor))', run before main()
    //      (destructor $func_name)             ;; similar to GCC '__attribute__((destructor))', run after main()
    //      ...
    // )

    consume_left_paren(iter, "module")?;
    consume_symbol(iter, "module")?;

    let name = expect_identifier(iter, "module name")?;
    let (runtime_version_major, runtime_version_minor) = parse_module_runtime_version_node(iter)?;

    // optional parameters
    if exist_child_node(iter, "constructor") {
        todo!()
    }

    if exist_child_node(iter, "destructor") {
        todo!()
    }

    let mut element_nodes: Vec<ModuleElementNode> = vec![];

    // parse module elements
    while iter.look_ahead_equals(0, &Token::LeftParen) {
        if let Some(Token::Symbol(child_node_name)) = iter.peek(1) {
            let element_node = match child_node_name.as_str() {
                "fn" => parse_func_node(iter)?,
                "data" => parse_data_node(iter)?,
                "extern" => parse_extern_node(iter)?,
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
        name_path: name,
        runtime_version_major,
        runtime_version_minor,
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

    consume_left_paren(iter, "module runtime version")?;
    consume_symbol(iter, "runtime_version")?;
    let ver_string = expect_string(iter, "module runtime version")?;
    consume_right_paren(iter)?;

    let vers: Vec<&str> = ver_string.split('.').collect();
    if vers.len() != 2 {
        return Err(ParseError::new(&format!(
            "Error runtime version number, expect: \"major.minor\", actual: \"{}\"",
            ver_string
        )));
    }

    let major = vers[0].parse::<u16>().map_err(|_| {
        ParseError::new(&format!(
            "Major version '{}' is not a valid number.",
            vers[0]
        ))
    })?;

    let minor = vers[1].parse::<u16>().map_err(|_| {
        ParseError::new(&format!(
            "Minor version '{}' is not a valid number.",
            vers[1]
        ))
    })?;

    Ok((major, minor))
}

fn parse_func_node(iter: &mut PeekableIterator<Token>) -> Result<ModuleElementNode, ParseError> {
    // (fn ...) ...  //
    // ^        ^____// to here
    // |_____________// current token

    // the node 'fn' syntax:
    //
    // (fn $name (param $param_0 DATA_TYPE) ...
    //           (result DATA_TYPE) ...
    //           (local $local_variable_name LOCAL_DATA_TYPE) ...
    //           (code ...)
    //)

    // e.g.
    //
    // (fn $add (param $lhs i32) (param $rhs i32) (result i32) ...)     ;; signature
    // (fn $add (param $lhs i32) (result i32) (result i32) ...)         ;; signature with multiple return values
    // (fn $add (param $lhs i32) (results i32 i32) ...)                 ;; signature with multiple return values
    // (fn $add
    //     (local $sum i32)             ;; local variable with identifier and data type
    //     (local $db (bytes 12 4))     ;; bytes-type local variable
    //     ...
    // )
    //
    // (fn $add
    //     (code ...)                   ;; the function body, the instructions sequence, sholud be written inside the node '(code)'
    // )

    // function with 'exported' annotation
    // (fn $add exported ...)

    consume_left_paren(iter, "fn")?;
    consume_symbol(iter, "fn")?;
    let name = expect_identifier(iter, "fn")?;
    let exported = expect_specified_symbol_optional(iter, "exported");
    let (params, results) = parse_optional_signature(iter)?;
    let locals: Vec<LocalNode> = parse_optional_local_variables(iter)?;
    let code = parse_code_node(iter)?;
    consume_right_paren(iter)?;

    // function's code implies an instruction 'end' at the end.
    // instructions.push(Instruction::NoParams(Opcode::end));

    let func_node = FuncNode {
        name,
        exported,
        params,
        results,
        locals,
        code,
    };

    Ok(ModuleElementNode::FuncNode(func_node))
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

    let data_type_name = expect_symbol(iter, "data type")?;
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

    // also
    // (local $name (bytes DATA_LENGTH_NUMBER:i32 ALIGN_NUMBER:i16))

    consume_left_paren(iter, "local")?;
    consume_symbol(iter, "local")?;
    let name = expect_identifier(iter, "local")?;
    let (memory_data_type, data_length, align) = parse_memory_data_type(iter)?;

    consume_right_paren(iter)?;

    Ok(LocalNode {
        name,
        memory_data_type,
        data_length,
        align,
    })
}

// return '(MemoryDataType, data length, align)'
fn parse_memory_data_type(
    iter: &mut PeekableIterator<Token>,
) -> Result<(MemoryDataType, u32, u16), ParseError> {
    // (local $name i32) ...  //
    //              ^  ^______// to here
    //              |_________// current token

    // also
    // (local $name (bytes DATA_LENGTH_NUMBER:i32 ALIGN_NUMBER:i16))

    if iter.look_ahead_equals(0, &Token::LeftParen) {
        parse_memory_data_type_bytes(iter)
    } else {
        parse_memory_data_type_primitive(iter)
    }
}

// return '(MemoryDataType, data length, align)'
fn parse_memory_data_type_primitive(
    iter: &mut PeekableIterator<Token>,
) -> Result<(MemoryDataType, u32, u16), ParseError> {
    // i32 ...  //
    // i64 ...  //
    // f32 ...  //
    // f64 ...  //
    // ^   ^____// to here
    // |________// current token

    let memory_data_type_name = expect_symbol(iter, "memory data type")?;
    let memory_data_type_detail = match memory_data_type_name.as_str() {
        "i32" => (MemoryDataType::I32, 4, 4),
        "i64" => (MemoryDataType::I64, 8, 8),
        "f32" => (MemoryDataType::F32, 4, 4),
        "f64" => (MemoryDataType::F64, 8, 8),
        _ => {
            return Err(ParseError::new(&format!(
                "Unknown memory data type: {}",
                memory_data_type_name
            )))
        }
    };

    Ok(memory_data_type_detail)
}

// return '(MemoryDataType, data length, align)'
fn parse_memory_data_type_bytes(
    iter: &mut PeekableIterator<Token>,
) -> Result<(MemoryDataType, u32, u16), ParseError> {
    // (bytes DATA_LENGTH_NUMBER:i32 ALIGN_NUMBER:i16)) ...  //
    // ^                                                ^____// to here
    // |_____________________________________________________// current token

    consume_left_paren(iter, "bytes")?;
    consume_symbol(iter, "bytes")?;

    let length_number_token = expect_number(iter, "the length of memory data type bytes")?;
    let align_number_token = expect_number(iter, "the align of memory data type bytes")?;

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

    Ok((MemoryDataType::BYTES, length, align))
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
    // - (do ...) ...  //
    // ^          ^____// to here
    // |_______________// current token

    // other sequence nodes:
    //
    // - (break ...)
    // - (recur ...)
    // - (return ...)
    // - (rerun ...)

    consume_left_paren(iter, node_name)?;
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

fn parse_next_instruction_operand(
    iter: &mut PeekableIterator<Token>,
    for_which_inst: &str,
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
                    "Expect operand for instruction \"{}\", actual {:?}",
                    for_which_inst, token
                )))
            }
        }
    } else {
        return Err(ParseError::new(&format!(
            "Missing operand for instruction \"{}\"",
            for_which_inst
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
    // also maybe:
    //
    // (inst_name PARAM0 PARAM1 ...)
    // (inst_name OPERAND0 OPERAND1 ...)
    // (inst_name PARAM0 ... OPERAND0 ...)
    // (pesudo_inst_name ...)

    if let Some(Token::Symbol(child_node_name)) = iter.peek(1) {
        let owned_name = child_node_name.to_owned();
        let inst_name = owned_name.as_str();
        let instruction = if let Some(kind) = get_instruction_kind(inst_name) {
            match *kind {
                InstructionKind::NoParams(opcode, operand_count) => {
                    parse_instruction_kind_no_params(iter, inst_name, opcode, operand_count)?
                }
                //
                InstructionKind::LocalLoad(opcode) => {
                    parse_instruction_kind_local_load(iter, inst_name, opcode, true)?
                }
                InstructionKind::LocalStore(opcode) => {
                    parse_instruction_kind_local_store(iter, inst_name, opcode, true)?
                }
                InstructionKind::LocalLongLoad(opcode) => {
                    parse_instruction_kind_local_long_load(iter, inst_name, opcode, true)?
                }
                InstructionKind::LocalLongStore(opcode) => {
                    parse_instruction_kind_local_long_store(iter, inst_name, opcode, true)?
                }
                InstructionKind::DataLoad(opcode) => {
                    parse_instruction_kind_local_load(iter, inst_name, opcode, false)?
                }
                InstructionKind::DataStore(opcode) => {
                    parse_instruction_kind_local_store(iter, inst_name, opcode, false)?
                }
                InstructionKind::DataLongLoad(opcode) => {
                    parse_instruction_kind_local_long_load(iter, inst_name, opcode, false)?
                }
                InstructionKind::DataLongStore(opcode) => {
                    parse_instruction_kind_local_long_store(iter, inst_name, opcode, false)?
                }
                //
                InstructionKind::HeapLoad(opcode) => {
                    parse_instruction_kind_heap_load(iter, inst_name, opcode)?
                }
                InstructionKind::HeapStore(opcode) => {
                    parse_instruction_kind_heap_store(iter, inst_name, opcode)?
                }
                //
                InstructionKind::UnaryOp(opcode) => {
                    parse_instruction_kind_unary_op(iter, inst_name, opcode)?
                }
                InstructionKind::UnaryOpParamI16(opcode) => {
                    parse_instruction_kind_unary_op_param_i16(iter, inst_name, opcode)?
                }
                InstructionKind::BinaryOp(opcode) => {
                    parse_instruction_kind_binary_op(iter, inst_name, opcode)?
                }
                //
                InstructionKind::ImmI32 => parse_instruction_kind_imm_i32(iter)?,
                InstructionKind::ImmI64 => parse_instruction_kind_imm_i64(iter)?,
                InstructionKind::ImmF32 => parse_instruction_kind_imm_f32(iter)?,
                InstructionKind::ImmF64 => parse_instruction_kind_imm_f64(iter)?,
                //
                InstructionKind::When => parse_instruction_kind_when(iter)?,
                InstructionKind::If => parse_instruction_kind_if(iter)?,
                InstructionKind::Branch => parse_instruction_kind_branch(iter)?,
                InstructionKind::For => parse_instruction_kind_for(iter)?,

                InstructionKind::Sequence(node_name) => {
                    parse_instruction_sequence_node(iter, node_name)?
                }
                //
                InstructionKind::Call => parse_instruction_kind_call_by_name(iter, "call", true)?,
                InstructionKind::DynCall => parse_instruction_kind_call_by_operand_num(iter)?,
                InstructionKind::EnvCall => {
                    parse_instruction_kind_call_by_num(iter, "envcall", true)?
                }
                InstructionKind::SysCall => {
                    parse_instruction_kind_call_by_num(iter, "syscall", false)?
                }
                InstructionKind::ExtCall => {
                    parse_instruction_kind_call_by_name(iter, "extcall", false)?
                }
                // macro
                InstructionKind::MacroGetFuncPubIndex => {
                    parse_instruction_kind_get_func_pub_index(iter)?
                }
                InstructionKind::Debug => parse_instruction_kind_debug(iter)?,
                InstructionKind::Unreachable => parse_instruction_kind_unreachable(iter)?,
                InstructionKind::HostAddrFunc => parse_instruction_kind_host_addr_func(iter)?,
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
            InstructionKind::NoParams(opcode, operand_cound) => {
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
    // ^                             ^____// to here
    // |__________________________________// current token

    let mut operands = vec![];

    consume_left_paren(iter, "instruction")?;
    consume_symbol(iter, inst_name)?;

    // operands
    for _ in 0..operand_count {
        let operand = parse_next_instruction_operand(iter, inst_name)?;
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

    consume_left_paren(iter, "i32.imm")?;
    consume_symbol(iter, "i32.imm")?;
    let number_token = expect_number(iter, "i32.imm")?;
    consume_right_paren(iter)?;

    Ok(Instruction::ImmI32(parse_u32_string(&number_token)?))
}

fn parse_instruction_kind_imm_i64(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (i64.imm 123) ... //
    // ^             ^___// to here
    // |_________________// current token

    consume_left_paren(iter, "i64.imm")?;
    consume_symbol(iter, "i64.imm")?;
    let number_token = expect_number(iter, "i64.imm")?;
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
    // (f32.imm 0x40490fdb)
    // (f32.imm 0b0_000_0011)
    // the hex string is the little-endian bytes in the memory

    consume_left_paren(iter, "f32.imm")?;
    consume_symbol(iter, "f32.imm")?;
    let number_token = expect_number(iter, "f32.imm")?;
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
    // (f64.imm 0x400921fb54442d18)
    // (f64.imm 0b0_000_0011)
    // the hex string is the little-endian bytes in the memory

    consume_left_paren(iter, "f64.imm")?;
    consume_symbol(iter, "f64.imm")?;
    let number_token = expect_number(iter, "f64.imm")?;
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
    // (local.load64_i64 $name OFFSET_NUMBER:i16)

    consume_left_paren(iter, "instruction")?;
    consume_symbol(iter, inst_name)?;
    let name = expect_identifier(iter, inst_name)?;
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
            name_path: name,
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
    // (local.store $name OPERAND) ... //
    // ^                           ^___// to here
    // |_______________________________// current token
    //
    // also:
    // (local.store $name OFFSET_NUMBER:i16 OPERAND)

    consume_left_paren(iter, "instruction")?;
    consume_symbol(iter, inst_name)?;
    let name = expect_identifier(iter, inst_name)?;
    let offset = if let Some(offset_number_token) = expect_number_optional(iter) {
        parse_u16_string(&offset_number_token)?
    } else {
        0
    };

    let operand = parse_next_instruction_operand(iter, inst_name)?;
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
            name_path: name,
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
    // (local.long_load $name OPERAND_FOR_OFFSET:i32) ... //
    // ^                                              ^___// to here
    // |__________________________________________________// current token

    consume_left_paren(iter, "instruction")?;
    consume_symbol(iter, inst_name)?;
    let name = expect_identifier(iter, inst_name)?;
    let offset = parse_next_instruction_operand(iter, inst_name)?;
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
            name_path: name,
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
    // (local.long_store $name OPERAND_FOR_OFFSET:i32 OPERAND) ... //
    // ^                                                       ^___// to here
    // |___________________________________________________________// current token

    consume_left_paren(iter, "instruction")?;
    consume_symbol(iter, inst_name)?;
    let name = expect_identifier(iter, inst_name)?;
    let offset = parse_next_instruction_operand(iter, inst_name)?;
    let operand = parse_next_instruction_operand(iter, inst_name)?;
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
            name_path: name,
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
    // (heap.load OPERAND_FOR_ADDR) ... //
    // ^                            ^___// to here
    // |________________________________// current token
    //
    // also:
    // (heap.load OFFSET_NUMBER:i16 OPERAND_FOR_ADDR)

    consume_left_paren(iter, "instruction")?;
    consume_symbol(iter, inst_name)?;

    let offset = if let Some(offset_token_number) = expect_number_optional(iter) {
        parse_u16_string(&offset_token_number)?
    } else {
        0
    };

    let addr = parse_next_instruction_operand(iter, inst_name)?;
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
    // (heap.store OPERAND_FOR_ADDR OPERAND) ... //
    // ^                                     ^___// to here
    // |_________________________________________// current token
    //
    // also:
    // (heap.store OFFSET_NUMBER:i16 OPERAND_FOR_ADDR OPERAND)

    consume_left_paren(iter, "instruction")?;
    consume_symbol(iter, inst_name)?;

    let offset = if let Some(offset_number_token) = expect_number_optional(iter) {
        parse_u16_string(&offset_number_token)?
    } else {
        0
    };

    let addr = parse_next_instruction_operand(iter, inst_name)?;
    let operand = parse_next_instruction_operand(iter, inst_name)?;

    consume_right_paren(iter)?;

    Ok(Instruction::HeapStore {
        opcode,
        offset,
        addr: Box::new(addr),
        value: Box::new(operand),
    })
}

fn parse_instruction_kind_unary_op(
    iter: &mut PeekableIterator<Token>,
    inst_name: &str,
    opcode: Opcode,
) -> Result<Instruction, ParseError> {
    // (i32.not OPERAND) ... //
    // ^                 ^___// to here
    // |_____________________// current token

    consume_left_paren(iter, "instruction")?;
    consume_symbol(iter, inst_name)?;
    let operand = parse_next_instruction_operand(iter, inst_name)?;
    consume_right_paren(iter)?;

    Ok(Instruction::UnaryOp {
        opcode,
        number: Box::new(operand),
    })
}

fn parse_instruction_kind_unary_op_param_i16(
    iter: &mut PeekableIterator<Token>,
    inst_name: &str,
    opcode: Opcode,
) -> Result<Instruction, ParseError> {
    // (i32.inc NUM:i16 OPERAND) ... //
    // ^                         ^___// to here
    // |_____________________________// current token

    consume_left_paren(iter, "instruction")?;
    consume_symbol(iter, inst_name)?;
    let number_token = expect_number(iter, inst_name)?;
    let param_i16 = parse_u16_string(&number_token)?;
    let operand = parse_next_instruction_operand(iter, inst_name)?;
    consume_right_paren(iter)?;

    Ok(Instruction::UnaryOpParamI16 {
        opcode,
        amount: param_i16,
        number: Box::new(operand),
    })
}

fn parse_instruction_kind_binary_op(
    iter: &mut PeekableIterator<Token>,
    inst_name: &str,
    opcode: Opcode,
) -> Result<Instruction, ParseError> {
    // (i32.add OPERAND_LHS OPERAND_RHS) ... //
    // ^                                 ^___// to here
    // |_____________________________________// current token

    consume_left_paren(iter, "instruction")?;
    consume_symbol(iter, inst_name)?;
    let left = parse_next_instruction_operand(iter, inst_name)?;
    let right = parse_next_instruction_operand(iter, inst_name)?;
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

    consume_left_paren(iter, "when")?;
    consume_symbol(iter, "when")?;
    let test = parse_next_instruction_operand(iter, "when.test")?;
    // let locals = parse_optional_local_variables(iter)?;
    let consequent = parse_next_instruction_operand(iter, "when.consequent")?;
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

    consume_left_paren(iter, "if")?;
    consume_symbol(iter, "if")?;
    let results = parse_optional_signature_results_only(iter)?;
    let test = parse_next_instruction_operand(iter, "if.test")?;
    // let locals = parse_optional_local_variables(iter)?;
    let consequent = parse_next_instruction_operand(iter, "if.consequent")?;
    let alternate = parse_next_instruction_operand(iter, "if.alternate")?;
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

    consume_left_paren(iter, "branch")?;
    consume_symbol(iter, "branch")?;
    let results = parse_optional_signature_results_only(iter)?;
    // let locals = parse_optional_local_variables(iter)?;
    let mut cases = vec![];

    while exist_child_node(iter, "case") {
        consume_left_paren(iter, "case")?;
        consume_symbol(iter, "case")?;
        let test = parse_next_instruction_operand(iter, "case")?;
        let consequent = parse_next_instruction_operand(iter, "case")?;
        consume_right_paren(iter)?;

        cases.push(BranchCase {
            test: Box::new(test),
            consequent: Box::new(consequent),
        });
    }

    let mut default = None;
    if exist_child_node(iter, "default") {
        consume_left_paren(iter, "default")?;
        consume_symbol(iter, "default")?;
        let consequent = parse_next_instruction_operand(iter, "default")?;
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

    consume_left_paren(iter, "for")?;
    consume_symbol(iter, "for")?;
    let (params, results) = parse_optional_signature(iter)?;
    let locals = parse_optional_local_variables(iter)?;
    let code = parse_next_instruction_operand(iter, "for (code)")?;
    consume_right_paren(iter)?;

    Ok(Instruction::For {
        params,
        results,
        locals,
        code: Box::new(code),
    })
}

fn parse_instruction_kind_call_by_name(
    iter: &mut PeekableIterator<Token>,
    node_name: &str,
    is_call: bool,
) -> Result<Instruction, ParseError> {
    // (call/extcall $name ...) ...  //
    // ^                        ^____// to here
    // ______________________________// current token

    consume_left_paren(iter, node_name)?;
    consume_symbol(iter, node_name)?;
    let name = expect_identifier(iter, node_name)?;

    let mut args = vec![];
    while let Some(arg) = parse_next_instruction_optional(iter)? {
        args.push(arg);
    }

    consume_right_paren(iter)?;

    let instruction = if is_call {
        Instruction::Call { name_path: name, args }
    } else {
        Instruction::ExtCall { name, args }
    };

    Ok(instruction)
}

fn parse_instruction_kind_call_by_num(
    iter: &mut PeekableIterator<Token>,
    node_name: &str,
    is_envcall: bool,
) -> Result<Instruction, ParseError> {
    // (envcall/syscall num ...) ...  //
    // ^                         ^____// to here
    // _______________________________// current token

    consume_left_paren(iter, node_name)?;
    consume_symbol(iter, node_name)?;
    let number_token = expect_number(iter, node_name)?;
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

fn parse_instruction_kind_call_by_operand_num(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (dyncall OPERAND_FOR_NUM ...) ...  //
    // ^                             ^____// to here
    // ___________________________________// current token

    consume_left_paren(iter, "dyncall")?;
    consume_symbol(iter, "dyncall")?;

    let num = parse_next_instruction_operand(iter, "dyncall")?;

    let mut args = vec![];
    while let Some(arg) = parse_next_instruction_optional(iter)? {
        args.push(arg);
    }

    consume_right_paren(iter)?;

    Ok(Instruction::DynCall {
        num: Box::new(num),
        args,
    })
}

fn parse_instruction_kind_get_func_pub_index(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (macro.get_func_pub_index name ...) ...  //
    // ^                                   ^____// to here
    // _________________________________________// current token

    consume_left_paren(iter, "macro.get_func_pub_index")?;
    consume_symbol(iter, "macro.get_func_pub_index")?;
    let name = expect_identifier(iter, "macro.get_func_pub_index")?;
    consume_right_paren(iter)?;

    Ok(Instruction::MacroGetFuncPubIndex(name))
}

fn parse_instruction_kind_debug(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (debug num ...) ...  //
    // ^               ^____// to here
    // _____________________// current token

    consume_left_paren(iter, "debug")?;
    consume_symbol(iter, "debug")?;
    let number_token = expect_number(iter, "debug")?;
    let num = parse_u32_string(&number_token)?;
    consume_right_paren(iter)?;

    Ok(Instruction::Debug(num))
}

fn parse_instruction_kind_unreachable(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (unreachable num ...) ...  //
    // ^                     ^____// to here
    // ___________________________// current token

    consume_left_paren(iter, "unreachable")?;
    consume_symbol(iter, "unreachable")?;
    let number_token = expect_number(iter, "unreachable")?;
    let num = parse_u32_string(&number_token)?;
    consume_right_paren(iter)?;

    Ok(Instruction::Unreachable(num))
}

fn parse_instruction_kind_host_addr_func(
    iter: &mut PeekableIterator<Token>,
) -> Result<Instruction, ParseError> {
    // (host.addr_func name ...) ...  //
    // ^                         ^____// to here
    // _______________________________// current token

    consume_left_paren(iter, "host.addr_func")?;
    consume_symbol(iter, "host.addr_func")?;
    let name = expect_identifier(iter, "host.addr_func")?;
    consume_right_paren(iter)?;

    Ok(Instruction::HostAddrFunc(name))
}

fn parse_data_node(iter: &mut PeekableIterator<Token>) -> Result<ModuleElementNode, ParseError> {
    // (data $name (read_only i32 123)) ...  //
    // ^                                ^____// to here
    // |_____________________________________// current token

    // also
    // (data $name (read_only string "Hello, World!"))    ;; UTF-8 encoding string
    // (data $name (read_only cstring "Hello, World!"))   ;; type `cstring` will append '\0' at the end of string
    // (data $name (read_only (bytes 2) d"11-13-17-19"))

    // other sections than 'read_only'
    //
    // read-write section:
    // (data $name (read_write i32 123))
    //
    // uninitialized section:
    // (data $name (uninit i32))
    // (data $name (uninit (bytes 12 4)))

    // with 'exported' annotation
    // (data $name exported (read_only i32 123))

    consume_left_paren(iter, "data")?;
    consume_symbol(iter, "data")?;

    let name = expect_identifier(iter, "data")?;
    let exported = expect_specified_symbol_optional(iter, "exported");
    let data_kind = parse_data_kind_node(iter)?;

    consume_right_paren(iter)?;

    let data_node = DataNode {
        name,
        exported,
        data_kind,
    };

    Ok(ModuleElementNode::DataNode(data_node))
}

fn parse_data_kind_node(iter: &mut PeekableIterator<Token>) -> Result<DataKindNode, ParseError> {
    match iter.peek(1) {
        Some(Token::Symbol(kind)) => match kind.as_str() {
            "read_only" => parse_data_kind_node_read_only(iter),
            "read_write" => parse_data_kind_node_read_write(iter),
            "uninit" => parse_data_kind_node_uninit(iter),
            _ => Err(ParseError::new(&format!(
                "Unknown data kind: {}, only supports \"read_only\", \"read_write\", \"uninit\"",
                kind
            ))),
        },
        _ => Err(ParseError::new("Missing data kind node")),
    }
}

fn parse_data_kind_node_read_only(
    iter: &mut PeekableIterator<Token>,
) -> Result<DataKindNode, ParseError> {
    // (read_only i32 123) ...  //
    // ^                   ^____// to here
    // |________________________// current token

    // also
    // (read_only string "Hello, World!")    ;; UTF-8 encoding string
    // (read_only cstring "Hello, World!")   ;; type `cstring` will append '\0' at the end of string
    // (read_only (bytes ALIGN_NUMBER:i16) d"11-13-17-19")

    consume_left_paren(iter, "read_only")?;
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

    // also
    // (read_write string "Hello, World!")    ;; UTF-8 encoding string
    // (read_write cstring "Hello, World!")   ;; type `cstring` will append '\0' at the end of string
    // (read_write (bytes ALIGN_NUMBER:i16) d"11-13-17-19")

    consume_left_paren(iter, "read_write")?;
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

    // also
    // (uninit (bytes 12 4))

    consume_left_paren(iter, "uninit")?;
    consume_symbol(iter, "uninit")?;

    let (memory_data_type, data_length, align) = parse_memory_data_type(iter)?;
    let inited_data = UninitData {
        memory_data_type,
        length: data_length,
        align,
    };
    consume_right_paren(iter)?;

    let data_kind_node = DataKindNode::Uninit(inited_data);
    Ok(data_kind_node)
}

fn parse_inited_data(iter: &mut PeekableIterator<Token>) -> Result<InitedData, ParseError> {
    // (read_only i32 123) ...  //
    //            ^      ^______// to here
    //            |_____________// current token

    // also
    // (read_write string "Hello, World!")    ;; UTF-8 encoding string
    // (read_write cstring "Hello, World!")   ;; type `cstring` will append '\0' at the end of string
    // (read_write (bytes ALIGN_NUMBER:i16) d"11-13-17-19")

    let inited_data = match iter.next() {
        Some(Token::Symbol(inited_data_type)) => match inited_data_type.as_str() {
            "i32" => {
                let value_token = expect_number(iter, "data")?;
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
                let value_token = expect_number(iter, "data")?;
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
                let value_token = expect_number(iter, "data")?;
                let value_imm = parse_f32_string(&value_token)?;
                let bytes = match value_imm {
                    ImmF32::Float(value) => value.to_le_bytes().to_vec(),
                    ImmF32::Hex(value) => value.to_be_bytes().to_vec(),
                };

                InitedData {
                    memory_data_type: MemoryDataType::F32,
                    length: 4,
                    align: 4,
                    value: bytes,
                }
            }
            "f64" => {
                let value_token = expect_number(iter, "data")?;
                let value_imm = parse_f64_string(&value_token)?;
                let bytes = match value_imm {
                    ImmF64::Float(value) => value.to_le_bytes().to_vec(),
                    ImmF64::Hex(value) => value.to_be_bytes().to_vec(),
                };

                InitedData {
                    memory_data_type: MemoryDataType::F64,
                    length: 8,
                    align: 8,
                    value: bytes,
                }
            }
            "string" => {
                let value = expect_string(iter, "data")?;
                let bytes = value.as_bytes().to_vec();
                InitedData {
                    memory_data_type: MemoryDataType::BYTES,
                    length: bytes.len() as u32,
                    align: 1,
                    value: bytes,
                }
            }
            "cstring" => {
                let value = expect_string(iter, "data")?;
                let mut bytes = value.as_bytes().to_vec();
                bytes.push(0); // append '\0'

                InitedData {
                    memory_data_type: MemoryDataType::BYTES,
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
            // (bytes ALIGH_NUMBER:i16) DATA ...  //
            //  ^                            ^____//
            //  |_________________________________//

            consume_symbol(iter, "bytes")?;
            let align_token = expect_number(iter, "bytes")?;
            let align = parse_u16_string(&align_token)?;
            consume_right_paren(iter)?;

            let bytes = expect_bytes(iter, "data")?;

            InitedData {
                memory_data_type: MemoryDataType::BYTES,
                length: bytes.len() as u32,
                align,
                value: bytes,
            }
        }
        _ => return Err(ParseError::new("Missing the value of data item")),
    };

    Ok(inited_data)
}

fn parse_extern_node(iter: &mut PeekableIterator<Token>) -> Result<ModuleElementNode, ParseError> {
    // (extern
    //     (library shared "math.so.1")
    //     (fn $add "add" (param i32) (param i32) (result i32)
    //     ) ...  //
    // ^     ^____// to here
    // |__________// current token

    consume_left_paren(iter, "extern")?;
    consume_symbol(iter, "extern")?;

    let external_library_node = parse_external_library_node(iter)?;

    let mut external_items: Vec<ExternalItem> = vec![];

    // parse external items
    while iter.look_ahead_equals(0, &Token::LeftParen) {
        if let Some(Token::Symbol(child_node_name)) = iter.peek(1) {
            let element_node = match child_node_name.as_str() {
                "fn" => parse_external_func_node(iter)?,
                _ => {
                    return Err(ParseError::new(&format!(
                        "Unknown external item: {}",
                        child_node_name
                    )))
                }
            };
            external_items.push(element_node);
        } else {
            break;
        }
    }

    consume_right_paren(iter)?;

    let extern_node = ExternNode {
        external_library_node,
        external_items,
    };

    Ok(ModuleElementNode::ExternNode(extern_node))
}

fn parse_external_library_node(
    iter: &mut PeekableIterator<Token>,
) -> Result<ExternalLibraryNode, ParseError> {
    // (library shared "math.so.1") ...  //
    // ^                            ^____// to here
    // |_________________________________// current token

    // also:
    // (library system "libc.so.6")
    // (library user "lib-test-0.so.1")

    consume_left_paren(iter, "library")?;
    consume_symbol(iter, "library")?;

    let external_library_type_str = expect_symbol(iter, "library")?;
    let external_library_type = match external_library_type_str.as_str() {
        "shared" => ExternalLibraryType::Shared,
        "system" => ExternalLibraryType::System,
        "user" => ExternalLibraryType::User,
        _ => {
            return Err(ParseError {
                message: format!("Unknown share library type: {}", external_library_type_str),
            })
        }
    };

    let name = expect_string(iter, "library")?;
    consume_right_paren(iter)?;

    Ok(ExternalLibraryNode {
        external_library_type,
        name,
    })
}

fn parse_external_func_node(
    iter: &mut PeekableIterator<Token>,
) -> Result<ExternalItem, ParseError> {
    // (fn $add "add" (param i32) (param i32) (result i32)
    // (fn $add "add" (params i32 i32) (result i32))
    // parse_optional_external_signature

    consume_left_paren(iter, "extern.fn)")?;
    consume_symbol(iter, "fn")?;

    let name = expect_identifier(iter, "extern.fn")?;
    let symbol = expect_string(iter, "extern.fn")?;
    let (params, results) = parse_optional_simplified_signature(iter)?;

    consume_right_paren(iter)?;

    Ok(ExternalItem::ExternalFunc(ExternalFuncNode {
        name,
        symbol,
        params,
        results,
    }))
}

fn parse_optional_simplified_signature(
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
                    let data_type = parse_simplified_param_node(iter)?;
                    params.push(data_type);
                }
                "params" => {
                    let mut data_types = parse_simplified_params_node(iter)?;
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

fn parse_simplified_param_node(iter: &mut PeekableIterator<Token>) -> Result<DataType, ParseError> {
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

fn parse_simplified_params_node(
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
    let opt_token = iter.next();
    if let Some(token) = opt_token {
        if token == Token::LeftParen {
            Ok(())
        } else {
            Err(ParseError::new(&format!("Expect a node for {}", for_what)))
        }
    } else {
        Err(ParseError::new(&format!(
            "Missing a node for: {}",
            for_what
        )))
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
        Some(token) => {
            if let Token::Number(number_token) = token {
                let number_token_clone = number_token.to_owned();
                iter.next();
                Some(number_token_clone)
            } else {
                None
            }
        }
        None => None,
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

// fn expect_identifier_optional(iter: &mut PeekableIterator<Token>) -> Option<String> {
//     match iter.peek(0) {
//         Some(Token::Identifier(s)) => {
//             let id = s.clone();
//             iter.next();
//             Some(id)
//         }
//         _ => None,
//     }
// }

fn exist_child_node(iter: &mut PeekableIterator<Token>, child_node_name: &str) -> bool {
    if let Some(Token::LeftParen) = iter.peek(0) {
        matches!(iter.peek(1), Some(Token::Symbol(n)) if n == child_node_name)
    } else {
        false
    }
}

fn get_instruction_kind(inst_name: &str) -> Option<&InstructionKind> {
    unsafe {
        if let Some(table_ref) = &INSTRUCTION_KIND_TABLE {
            table_ref.get(inst_name)
        } else {
            panic!("The instruction table is not initialized yet.")
        }
    }
}

fn parse_u16_string(number_token: &NumberToken) -> Result<u16, ParseError> {
    let e = ParseError::new(&format!(
        "\"{:?}\" is not a valid integer number.",
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
    };

    Ok(num)
}

fn parse_u32_string(number_token: &NumberToken) -> Result<u32, ParseError> {
    let e = ParseError::new(&format!(
        "\"{:?}\" is not a valid integer number.",
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
    };

    Ok(num)
}

fn parse_u64_string(number_token: &NumberToken) -> Result<u64, ParseError> {
    let e = ParseError::new(&format!(
        "\"{:?}\" is not a valid integer number.",
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
    };

    Ok(num)
}

fn parse_f32_string(number_token: &NumberToken) -> Result<ImmF32, ParseError> {
    let e = ParseError::new(&format!(
        "\"{:?}\" is not a valid float number.",
        number_token
    ));

    let imm_f32 = match number_token {
        NumberToken::Hex(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_'); // remove underscores
            let value = u32::from_str_radix(&ns, 16).map_err(|_| e)?;
            ImmF32::Hex(value)
        }
        NumberToken::Binary(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_');
            let value = u32::from_str_radix(&ns, 2).map_err(|_| e)?;
            ImmF32::Hex(value)
        }
        NumberToken::Decimal(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_');
            let value = ns.as_str().parse::<f32>().map_err(|_| e)?;
            ImmF32::Float(value)
        }
    };

    Ok(imm_f32)
}

fn parse_f64_string(number_token: &NumberToken) -> Result<ImmF64, ParseError> {
    let e = ParseError::new(&format!(
        "\"{:?}\" is not a valid float number.",
        number_token
    ));

    let imm_f64 = match number_token {
        NumberToken::Hex(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_'); // remove underscores
            let value = u64::from_str_radix(&ns, 16).map_err(|_| e)?;
            ImmF64::Hex(value)
        }
        NumberToken::Binary(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_');
            let value = u64::from_str_radix(&ns, 2).map_err(|_| e)?;
            ImmF64::Hex(value)
        }
        NumberToken::Decimal(ns_ref) => {
            let mut ns = ns_ref.to_owned();
            ns.retain(|c| c != '_');
            let value = ns.as_str().parse::<f64>().map_err(|_| e)?;
            ImmF64::Float(value)
        }
    };

    Ok(imm_f64)
}

#[cfg(test)]
mod tests {
    use std::vec;

    use pretty_assertions::assert_eq;

    use ancvm_types::{opcode::Opcode, DataType, ExternalLibraryType, MemoryDataType};

    use crate::{
        ast::{
            BranchCase, DataKindNode, DataNode, ExternNode, ExternalFuncNode, ExternalItem,
            ExternalLibraryNode, FuncNode, ImmF32, ImmF64, InitedData, Instruction, LocalNode,
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
        if let ModuleElementNode::FuncNode(func_node) = &module_node.element_nodes[0] {
            func_node.code.clone()
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
    fn test_parse_empty_module() {
        assert_eq!(
            parse_from_str(r#"(module $app (runtime_version "1.2"))"#).unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 2,
                element_nodes: vec![]
            }
        );

        assert!(parse_from_str(r#"()"#).is_err());
        assert!(parse_from_str(r#"(module)"#).is_err());
        assert!(parse_from_str(r#"(module $app)"#).is_err());
        assert!(parse_from_str(r#"(module $app ())"#).is_err());
        assert!(parse_from_str(r#"(module $app (runtime_version))"#).is_err());
        assert!(parse_from_str(r#"(module $app (runtime_version "1"))"#).is_err());
        assert!(parse_from_str(r#"(module $app (runtime_version "1a.2b"))"#).is_err());
    }

    #[test]
    fn test_parse_function_node_signature() {
        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (fn $add (param $lhs i32) (param $rhs i64) (result i32) (result i64)
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
                element_nodes: vec![ModuleElementNode::FuncNode(FuncNode {
                    name: "add".to_owned(),
                    exported: false,
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
                (fn $add (param $lhs i32) (param $rhs i64) (results i32 i64) (result f32) (result f64)
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
                element_nodes: vec![ModuleElementNode::FuncNode(FuncNode {
                    name: "add".to_owned(),
                    exported: false,
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

        // test property 'exported'

        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (fn $add exported (code))
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                element_nodes: vec![ModuleElementNode::FuncNode(FuncNode {
                    name: "add".to_owned(),
                    exported: true,
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
                (fn (code))
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
                (fn $add)
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_parse_function_node_local_variables() {
        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (fn $add
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
                element_nodes: vec![ModuleElementNode::FuncNode(FuncNode {
                    name: "add".to_owned(),
                    exported: false,
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
                            memory_data_type: MemoryDataType::BYTES,
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
    }

    #[test]
    fn test_parse_instructions_fundanmental() {
        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (fn $test
                    (code
                        nop
                        zero
                        (drop zero)
                        (duplicate zero)
                        (swap zero zero)
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
                Instruction::NoParams {
                    opcode: Opcode::duplicate,
                    operands: vec![noparams_nooperands(Opcode::zero),]
                },
                Instruction::NoParams {
                    opcode: Opcode::swap,
                    operands: vec![
                        noparams_nooperands(Opcode::zero),
                        noparams_nooperands(Opcode::zero)
                    ]
                },
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
                (fn $test
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
                (fn $test
                    (code
                        (f32.imm 3.1415927)
                        (f32.imm 0x40490fdb)
                        (f32.imm 2.7182817)
                        (f32.imm 0x402df854)

                        (f64.imm 3.141592653589793)
                        (f64.imm 0x400921fb54442d18)
                        (f64.imm 2.718281828459045)
                        (f64.imm 0x4005bf0a8b145769)
                    )
                )
            )
            "#
            ),
            vec![
                Instruction::ImmF32(ImmF32::Float(std::f32::consts::PI)),
                Instruction::ImmF32(ImmF32::Hex(0x40490fdb)),
                Instruction::ImmF32(ImmF32::Float(std::f32::consts::E)),
                Instruction::ImmF32(ImmF32::Hex(0x402df854)),
                //
                Instruction::ImmF64(ImmF64::Float(std::f64::consts::PI)),
                Instruction::ImmF64(ImmF64::Hex(0x400921fb54442d18)),
                Instruction::ImmF64(ImmF64::Float(std::f64::consts::E)),
                Instruction::ImmF64(ImmF64::Hex(0x4005bf0a8b145769)),
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
                (fn $test
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
                    number: Box::new(Instruction::ImmI32(11))
                },
                // 13 + 1
                Instruction::UnaryOpParamI16 {
                    opcode: Opcode::i32_inc,
                    amount: 1,
                    number: Box::new(Instruction::ImmI32(13))
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
                (fn $test
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
                (fn $test
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
                    name_path: "sum".to_owned(),
                    offset: 0
                },
                Instruction::DataLoad {
                    opcode: Opcode::data_load64_i64,
                    name_path: "count".to_owned(),
                    offset: 4
                },
                //
                Instruction::DataStore {
                    opcode: Opcode::data_store32,
                    name_path: "left".to_owned(),
                    offset: 0,
                    value: Box::new(Instruction::ImmI32(11))
                },
                //
                Instruction::DataStore {
                    opcode: Opcode::data_store64,
                    name_path: "right".to_owned(),
                    offset: 8,
                    value: Box::new(Instruction::ImmI64(13))
                },
                //
                Instruction::DataLongLoad {
                    opcode: Opcode::data_long_load64_i64,
                    name_path: "foo".to_owned(),
                    offset: Box::new(Instruction::ImmI32(17))
                },
                //
                Instruction::DataLongStore {
                    opcode: Opcode::data_long_store64,
                    name_path: "bar".to_owned(),
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
                (fn $test
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
                (fn $test
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
                (fn $test
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
                (fn $test
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
                (fn $test
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
                (fn $test
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
                (fn $test
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
                (fn $test
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
                (fn $test
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
                (fn $test
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
                (fn $test
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
                            number: Box::new(noparams_nooperands(Opcode::zero))
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
                (fn $test
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
                (fn $test
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
                (fn $test
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
                        value: Box::new(Instruction::UnaryOpParamI16 {
                            opcode: Opcode::i32_dec,
                            amount: 1,
                            number: Box::new(Instruction::LocalLoad {
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
                (fn $test (param $sum i32) (param $n i32) (result i32)
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
                element_nodes: vec![ModuleElementNode::FuncNode(FuncNode {
                    name: "test".to_owned(),
                    exported: false,
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
                            value: Box::new(Instruction::UnaryOpParamI16 {
                                opcode: Opcode::i32_dec,
                                amount: 1,
                                number: Box::new(Instruction::LocalLoad {
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
    fn test_parse_instructions_calling() {
        // test 'call', 'dyncall', 'envcall', 'syscall' and 'extcall'
        assert_eq!(
            parse_instructions_from_str(
                r#"
            (module $lib
                (runtime_version "1.0")
                (fn $test
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
                        (macro.get_func_pub_index $add)
                    )
                )
            )
            "#
            ),
            vec![
                Instruction::Call {
                    name_path: "add".to_owned(),
                    args: vec![Instruction::ImmI32(11), Instruction::ImmI32(13),]
                },
                Instruction::DynCall {
                    num: Box::new(Instruction::LocalLoad {
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
                    name: "format".to_owned(),
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
                Instruction::MacroGetFuncPubIndex("add".to_owned()),
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
                (fn $test
                    (code
                        panic
                        (unreachable 0x11)
                        (debug 0x13)
                        (host.addr_local $num 0x17)
                        (host.addr_local_long $sum (i32.imm 0x19))
                        (host.addr_data $msg 0x23)
                        (host.addr_data_long $title (i32.imm 0x29))
                        (host.addr_heap 0x31 (i32.imm 0x37))
                        (host.addr_func $add)
                        (host.copy_from_heap
                            (i32.imm 0x41)
                            (i32.imm 0x43)
                            (i32.imm 0x47)
                        )
                        (host.copy_to_heap
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
                Instruction::Unreachable(0x11),
                Instruction::Debug(0x13),
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
                    name_path: "msg".to_owned(),
                    offset: 0x23,
                },
                Instruction::DataLongLoad {
                    opcode: Opcode::host_addr_data_long,
                    name_path: "title".to_owned(),
                    offset: Box::new(Instruction::ImmI32(0x29)),
                },
                Instruction::HeapLoad {
                    opcode: Opcode::host_addr_heap,
                    offset: 0x31,
                    addr: Box::new(Instruction::ImmI32(0x37)),
                },
                Instruction::HostAddrFunc("add".to_owned(),),
                Instruction::NoParams {
                    opcode: Opcode::host_copy_from_heap,
                    operands: vec![
                        Instruction::ImmI32(0x41),
                        Instruction::ImmI32(0x43),
                        Instruction::ImmI32(0x47),
                    ],
                },
                Instruction::NoParams {
                    opcode: Opcode::host_copy_to_heap,
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
    fn test_parse_data_node_inited() {
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
                (data $d5 (read_only f32 0xdb0f_4940))
                (data $d6 (read_only i32 0b1010_0101))
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                element_nodes: vec![
                    ModuleElementNode::DataNode(DataNode {
                        name: "d0".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4,
                            value: 123u32.to_le_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d1".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::I64,
                            length: 8,
                            align: 8,
                            value: 123_456u64.to_le_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d2".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::F32,
                            length: 4,
                            align: 4,
                            value: std::f32::consts::PI.to_le_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d3".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::F64,
                            length: 8,
                            align: 8,
                            value: std::f64::consts::E.to_le_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d4".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4,
                            value: 0xaabb_ccddu32.to_le_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d5".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::F32,
                            length: 4,
                            align: 4,
                            value: std::f32::consts::PI.to_le_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d6".to_owned(),
                        exported: false,
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
                (data $d2 (read_only (bytes 2) d"11-13-17-19"))
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                element_nodes: vec![
                    ModuleElementNode::DataNode(DataNode {
                        name: "d0".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::BYTES,
                            length: 13,
                            align: 1,
                            value: "Hello, World!".as_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d1".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::BYTES,
                            length: 14,
                            align: 1,
                            value: "Hello, World!\0".as_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d2".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::BYTES,
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
                (data $d0 exported (read_only i32 123))
                (data $d1 exported (read_write i32 456))
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                element_nodes: vec![
                    ModuleElementNode::DataNode(DataNode {
                        name: "d0".to_owned(),
                        exported: true,
                        data_kind: DataKindNode::ReadOnly(InitedData {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4,
                            value: 123u32.to_le_bytes().to_vec()
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d1".to_owned(),
                        exported: true,
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
                (data $d0 (read_only bytes 2 d"11-13-17-19"))
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
                (data $d0 (read_only (bytes) d"11-13-17-19"))
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }

    #[test]
    fn test_parse_data_node_uninit() {
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
                element_nodes: vec![
                    ModuleElementNode::DataNode(DataNode {
                        name: "d0".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::Uninit(UninitData {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4,
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d1".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::Uninit(UninitData {
                            memory_data_type: MemoryDataType::I64,
                            length: 8,
                            align: 8,
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d2".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::Uninit(UninitData {
                            memory_data_type: MemoryDataType::F32,
                            length: 4,
                            align: 4,
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d3".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::Uninit(UninitData {
                            memory_data_type: MemoryDataType::F64,
                            length: 8,
                            align: 8,
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d4".to_owned(),
                        exported: false,
                        data_kind: DataKindNode::Uninit(UninitData {
                            memory_data_type: MemoryDataType::BYTES,
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
                (data $d0 exported (uninit i32))
                (data $d1 exported (uninit i64))
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                element_nodes: vec![
                    ModuleElementNode::DataNode(DataNode {
                        name: "d0".to_owned(),
                        exported: true,
                        data_kind: DataKindNode::Uninit(UninitData {
                            memory_data_type: MemoryDataType::I32,
                            length: 4,
                            align: 4,
                        })
                    }),
                    ModuleElementNode::DataNode(DataNode {
                        name: "d1".to_owned(),
                        exported: true,
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
    fn test_parse_extern_node() {
        assert_eq!(
            parse_from_str(
                r#"
            (module $app
                (runtime_version "1.0")
                (extern (library shared "math.so.1")
                    (fn $add "add" (param i32) (param i32) (result i32))
                    (fn $sub_i32 "sub" (params i32 i32) (result i32))
                    (fn $pause "pause_1s")
                )
                (extern (library system "libc.so.6")
                    (fn $getuid "getuid" (result i32))
                    (fn $getenv "getenv" (param (;name;) i64) (result i64))
                )
            )
            "#
            )
            .unwrap(),
            ModuleNode {
                name_path: "app".to_owned(),
                runtime_version_major: 1,
                runtime_version_minor: 0,
                element_nodes: vec![
                    ModuleElementNode::ExternNode(ExternNode {
                        external_library_node: ExternalLibraryNode {
                            external_library_type: ExternalLibraryType::Shared,
                            name: "math.so.1".to_owned()
                        },
                        external_items: vec![
                            ExternalItem::ExternalFunc(ExternalFuncNode {
                                name: "add".to_owned(),
                                symbol: "add".to_owned(),
                                params: vec![DataType::I32, DataType::I32,],
                                results: vec![DataType::I32]
                            }),
                            ExternalItem::ExternalFunc(ExternalFuncNode {
                                name: "sub_i32".to_owned(),
                                symbol: "sub".to_owned(),
                                params: vec![DataType::I32, DataType::I32,],
                                results: vec![DataType::I32]
                            }),
                            ExternalItem::ExternalFunc(ExternalFuncNode {
                                name: "pause".to_owned(),
                                symbol: "pause_1s".to_owned(),
                                params: vec![],
                                results: vec![]
                            })
                        ]
                    }),
                    ModuleElementNode::ExternNode(ExternNode {
                        external_library_node: ExternalLibraryNode {
                            external_library_type: ExternalLibraryType::System,
                            name: "libc.so.6".to_owned()
                        },
                        external_items: vec![
                            ExternalItem::ExternalFunc(ExternalFuncNode {
                                name: "getuid".to_owned(),
                                symbol: "getuid".to_owned(),
                                params: vec![],
                                results: vec![DataType::I32]
                            }),
                            ExternalItem::ExternalFunc(ExternalFuncNode {
                                name: "getenv".to_owned(),
                                symbol: "getenv".to_owned(),
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
                (extern
                    (fn $add "add")
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
                (extern
                    (library custom "libabc.so")
                    (fn $add "add")
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
                (extern
                    (library user "libabc.so")
                    (fn "add")
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
                (extern
                    (library user "libabc.so")
                    (fn $add)
                )
            )
            "#
            ),
            Err(ParseError { message: _ })
        ));
    }
}
