// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use anc_assembler::utils::helper_make_single_module_app;
use anc_context::resource::Resource;
use anc_isa::ForeignValue;
use anc_processor::{
    handler::Handler, in_memory_resource::InMemoryResource, process::process_function,
    HandleErrorType, HandlerError,
};
use pretty_assertions::assert_eq;

#[test]
fn test_assemble_control_flow_block() {
    // fn () -> (i32, i32, i32, i32)    ;; type idx 0
    //     imm_i32(11)
    //     imm_i32(13)
    //     block () -> ()               ;; type idx 1
    //         imm_i32(17)
    //         imm_i32(19)
    //     end
    //     imm_i32(23)
    //     imm_i32(29)
    // end
    //
    // expect (11, 13, 23, 29)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test()->(i32, i32, i32, i32)
        {
            imm_i32(11)
            imm_i32(13)
            block {
                imm_i32(17)
                imm_i32(19)
            }
            imm_i32(23)
            imm_i32(29)
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::U32(11),
            ForeignValue::U32(13),
            ForeignValue::U32(23),
            ForeignValue::U32(29),
        ]
    );
}

#[test]
fn test_assemble_control_flow_block_with_args_and_results() {
    // fn () -> (i32, i32, i32)
    //     imm_i32(11)
    //     imm_i32(13)
    //     block (i32) -> (i32)
    //         local_load(0)
    //         imm_i32(17)
    //         add_i32()
    //     end
    //     imm_i32(19)
    // end
    //
    // expect (11, 30, 19)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test()->(i32, i32, i32)
        {
            imm_i32(11)
            block(a:i32=imm_i32(13)) -> i32 {
                add_i32(
                    local_load_i32_s(a)
                    imm_i32(17)
                )
            }
            imm_i32(19)
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::U32(11),
            ForeignValue::U32(30),
            ForeignValue::U32(19),
        ]
    );
}

#[test]
fn test_assemble_control_flow_block_with_local_variables() {
    // fn (a/0:i32, b/1:i32) -> (i32,i32,i32,i32,i32,i32,i32,i32)
    //     [local c/2:i32, d/3:i32]
    //     c=a+1                            ;; 20
    //     d=b+1                            ;; 12
    //     block () -> (i32, i32, i32,i32)  ;; type idx 1
    //         [local p/0:i32, q/1:i32]
    //         a=a-1                        ;; 18
    //         b=b-1                        ;; 10
    //         p=c+d                        ;; 32
    //         q=c-d                        ;; 8
    //         load c
    //         load d
    //         block (x/0:i32, y/1:i32) -> (i32,i32)    ;; type idx 2
    //             d=d+1                    ;; 13
    //             q=q-1                    ;; 7
    //             x+q                      ;; 27 (ret #0)
    //             y+p                      ;; 44 (ret #1)
    //         end
    //         load p (ret #2)
    //         load q (ret #3)
    //     end
    //     load a (ret #4)
    //     load b (ret #5)
    //     load c (ret #6)
    //     load d (ret #7)
    // end
    //
    // expect (19, 11) -> (27, 44, 32, 7, 18, 10, 20, 13)

    let binary0 = helper_make_single_module_app(
        r#"
            fn test(a:i32,b:i32) ->
                (
                i32, i32, i32, i32
                i32, i32, i32, i32
                )
                [c:i32, d:i32]
            {
                local_store_i32(c
                    add_imm_i32(1, local_load_i32_s(a)))
                local_store_i32(d
                    add_imm_i32(1, local_load_i32_s(b)))

                block()->(i32, i32, i32, i32)
                    [p:i32, q:i32]
                {
                    local_store_i32(a
                        sub_imm_i32(1, local_load_i32_s(a)))
                    local_store_i32(b
                        sub_imm_i32(1, local_load_i32_s(b)))

                    local_store_i32(p
                        add_i32(
                            local_load_i32_s(c)
                            local_load_i32_s(d)
                        )
                    )
                    local_store_i32(q
                        sub_i32(
                            local_load_i32_s(c)
                            local_load_i32_s(d)
                        )
                    )

                    block(
                        x:i32=local_load_i32_s(c)
                        y:i32=local_load_i32_s(d)
                        ) -> (i32, i32)
                    {
                        local_store_i32(d
                            add_imm_i32(1, local_load_i32_s(d)))
                        local_store_i32(q
                            sub_imm_i32(1, local_load_i32_s(q)))

                        add_i32(
                            local_load_i32_s(x)
                            local_load_i32_s(q)
                        )
                        add_i32(
                            local_load_i32_s(y)
                            local_load_i32_s(p)
                        )
                    }

                    local_load_i32_s(p)
                    local_load_i32_s(q)
                }

                local_load_i32_s(a)
                local_load_i32_s(b)
                local_load_i32_s(c)
                local_load_i32_s(d)
            }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(19), ForeignValue::U32(11)],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::U32(27),
            ForeignValue::U32(44),
            ForeignValue::U32(32),
            ForeignValue::U32(7),
            ForeignValue::U32(18),
            ForeignValue::U32(10),
            ForeignValue::U32(20),
            ForeignValue::U32(13),
        ]
    );
}

#[test]
fn test_assemble_control_flow_break_function() {
    // fn () -> (i32, i32)
    //     imm_i32(11)
    //     imm_i32(13)
    //     break(0)
    //     imm_i32(17)
    //     imm_i32(19)
    // end
    //
    // expect (11, 13)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test()->(i32, i32)
        {
            break_fn(
                imm_i32(11)
                imm_i32(13)
            )
            imm_i32(23)
            imm_i32(29)
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();
    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![ForeignValue::U32(11), ForeignValue::U32(13),]
    );
}

#[test]
fn test_assemble_control_flow_break_block() {
    // fn () -> (i32, i32, i32, i32)
    //     imm_i32(11)
    //     imm_i32(13)
    //     block () -> (i32, i32)
    //         imm_i32(17)
    //         imm_i32(19)
    //         break(0)
    //         imm_i32(23)
    //         imm_i32(29)
    //     end
    //     imm_i32(31)
    //     imm_i32(37)
    // end
    //
    // expect (17, 19, 31, 37)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test()->(i32, i32, i32, i32)
        {
            imm_i32(11)
            imm_i32(13)
            block()->(i32, i32) {
                break(
                    imm_i32(17)
                    imm_i32(19)
                )
                imm_i32(23)
                imm_i32(29)
            }
            imm_i32(31)
            imm_i32(37)
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::U32(17),
            ForeignValue::U32(19),
            ForeignValue::U32(31),
            ForeignValue::U32(37),
        ]
    );
}

#[test]
fn test_assemble_control_flow_break_block_to_function() {
    // fn () -> (i32, i32)
    //     imm_i32 11()
    //     imm_i32 13()
    //     block () -> (i32 i32)
    //         imm_i32(17)
    //         imm_i32(19)
    //         break(1)
    //         imm_i32(23)
    //         imm_i32(29)
    //     end
    //     imm_i32(31)
    //     imm_i32(37)
    // end
    //
    // expect (17, 19)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test()->(i32, i32)
        {
            imm_i32(11)
            imm_i32(13)
            block()->(i32, i32) {
                break_fn(
                    imm_i32(17)
                    imm_i32(19)
                )
                imm_i32(23)
                imm_i32(29)
            }
            imm_i32(31)
            imm_i32(37)
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[]);
    assert_eq!(
        result0.unwrap(),
        vec![ForeignValue::U32(17), ForeignValue::U32(19),]
    );
}

#[test]
fn test_assemble_control_flow_structure_when() {
    // fn max (left/0:i32, right/1:i32) -> (i32)    ;; type idx 0
    //     [local ret/2 i32]
    //
    //     local_load32(0, 0)
    //     local_store_i32(0, 2)
    //
    //     local_load32(0, 0)
    //     local_load32(0, 1)
    //     lt_i32_u
    //     block_nez ()->()                         ;; type idx 1
    //          local_load32(1, 1)
    //          local_store_i32(1, 2)
    //     end
    //     local_load32(0, 2)
    // end
    //
    // assert (11, 13) -> (13)
    // assert (19, 17) -> (19)

    let binary0 = helper_make_single_module_app(
        r#"
        fn max(a:i32, b:i32)->i32
            [ret:i32]
        {
            local_store_i32(ret
                local_load_i32_s(a))

            when lt_i32_u(
                    local_load_i32_s(a)
                    local_load_i32_s(b)
                ){
                local_store_i32(ret
                    local_load_i32_s(b))
            }

            local_load_i32_s(ret)
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(11), ForeignValue::U32(13)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(13)]);

    let result1 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(19), ForeignValue::U32(17)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::U32(19)]);
}

#[test]
fn test_assemble_control_flow_break_crossing_block() {
    // cross block breaking
    //
    // fn (/0:i32) -> (i32 i32 i32 i32)     ;; type idx 0
    //     imm_i32(11)
    //     imm_i32(13)
    //     block () -> (i32 i32)            ;; type idx 1
    //         imm_i32(17)
    //         imm_i32(19)
    //         local_load_i32_u(1, 0)       ;; == true
    //         block_nez () -> (i32 i32)    ;; type idx 2
    //             imm_i32(23)
    //             imm_i32(29)
    //             break(1)
    //             imm_i32(31)
    //             imm_i32(37)
    //         end
    //         imm_i32(41)
    //         imm_i32(43)
    //     end
    //     imm_i32(51)
    //     imm_i32(53)
    // end
    //
    // expect (1) -> (23, 29, 51, 53)
    // expect (0) -> (41, 43, 51, 53)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(a:i32)->(i32, i32, i32, i32)
        {
            imm_i32(11)
            imm_i32(13)
            block ()->(i32, i32) {
                imm_i32(17)
                imm_i32(19)
                when local_load_i32_s(a) {
                    break(
                        imm_i32(23)
                        imm_i32(29)
                    )
                    imm_i32(31)
                    imm_i32(37)
                }
                imm_i32(41)
                imm_i32(43)
            }
            imm_i32(51)
            imm_i32(53)
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(1)],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::U32(23),
            ForeignValue::U32(29),
            ForeignValue::U32(51),
            ForeignValue::U32(53),
        ]
    );

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(0)],
    );
    assert_eq!(
        result0.unwrap(),
        vec![
            ForeignValue::U32(41),
            ForeignValue::U32(43),
            ForeignValue::U32(51),
            ForeignValue::U32(53),
        ]
    );
}

#[test]
fn test_assemble_control_flow_structure_if() {
    // fn max (i32, i32) -> (i32)
    //     local_load32(0, 0)
    //     local_load32(0, 1)
    //     gt_i32_u
    //     block_alt ()->(i32)
    //         local_load32(1, 0)
    //     break_alt
    //         local_load32(1, 1)
    //     end
    // end
    //
    // assert (11, 13) -> (13)
    // assert (19, 17) -> (19)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(a:i32, b:i32) -> i32
        {
            if -> i32
                gt_i32_u(
                    local_load_i32_s(a)
                    local_load_i32_s(b)
                )
                local_load_i32_s(a)
                local_load_i32_s(b)
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(11), ForeignValue::U32(13)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(13)]);

    let result1 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(19), ForeignValue::U32(17)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::U32(19)]);
}

#[test]
fn test_assemble_control_flow_structure_if_nested() {
    // fn level (0/:i32) -> (i32)
    //     local_load32(0, 0)
    //     imm_i32(85)
    //     gt_i32_u
    //     block_alt ()->(i32)              ;; type idx 1
    //         imm_i32(65)                  ;; 'A' (85, 100]
    //     break_alt
    //         local_load32(1, 0)
    //         imm_i32(70)
    //         gt_i32_u
    //         block_alt ()->(i32)          ;; block 2 2
    //             imm_i32(66)              ;; 'B' (70,85]
    //         break_alt
    //             local_load32(2, 0)
    //             imm_i32(55)
    //             gt_i32_u
    //             block_alt ()->(i32)      ;; block 3 3
    //                 imm_i32(67)          ;; 'C' (55, 70]
    //             break_alt
    //                 imm_i32(68)          ;; 'D' [0, 55]
    //             end
    //         end
    //     end
    // end
    //
    // assert (90) -> (65) 'A'
    // assert (80) -> (66) 'B'
    // assert (70) -> (67) 'C'
    // assert (60) -> (67) 'C'
    // assert (50) -> (68) 'D'
    // assert (40) -> (68) 'D'

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(a:i32) -> i32
        {
            if -> i32
                gt_i32_u(
                    local_load_i32_s(a)
                    imm_i32(85)
                )
                imm_i32(65)            // 'A'
                if -> i32
                    gt_i32_u(
                        local_load_i32_s(a)
                        imm_i32(70)
                    )
                    imm_i32(66)        // 'B'
                    if -> i32
                        gt_i32_u(
                            local_load_i32_s(a)
                            imm_i32(55)
                        )
                        imm_i32(67)    // 'C'
                        imm_i32(68)    // 'D'
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(90)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(65)]);

    let result1 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(80)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::U32(66)]);

    let result2 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(70)],
    );
    assert_eq!(result2.unwrap(), vec![ForeignValue::U32(67)]);

    let result3 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(60)],
    );
    assert_eq!(result3.unwrap(), vec![ForeignValue::U32(67)]);

    let result4 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(50)],
    );
    assert_eq!(result4.unwrap(), vec![ForeignValue::U32(68)]);

    let result5 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(40)],
    );
    assert_eq!(result5.unwrap(), vec![ForeignValue::U32(68)]);
}

#[test]
fn test_assemble_control_flow_structure_branch() {
    // fn $level (i32) -> (i32)
    //     (block 1 1) ()->(i32)        ;; block 1 1
    //                                  ;; case 1
    //         (local_load32 0 0)
    //         (i32_imm 85)
    //         i32_gt_u
    //         (block_nez 2) ()->()     ;; block 2 2
    //             (i32_imm 65)         ;; 'A' (85, 100]
    //             (break 1)
    //         end
    //                                  ;; case 2
    //         (local_load32 0 0)
    //         (i32_imm 70)
    //         i32_gt_u
    //         (block_nez 3) ()->()     ;; block 3 3
    //             (i32_imm 66)         ;; 'B' (70,85]
    //             (break 1)
    //         end
    //                                  ;; case 3
    //         (local_load32 0 0)
    //         (i32_imm 55)
    //         i32_gt_u
    //         (block_nez 4) ()->()     ;; block 4 4
    //             (i32_imm 67)         ;; 'C' (55, 70]
    //             (break 1)
    //         end
    //                                  ;; default
    //         (i32_imm 68)             ;; 'D' [0, 55]
    //     end
    // end
    //
    // assert (90) -> (65) 'A'
    // assert (80) -> (66) 'B'
    // assert (70) -> (67) 'C'
    // assert (60) -> (67) 'C'
    // assert (50) -> (68) 'D'
    // assert (40) -> (68) 'D'

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(a:i32) -> i32
        {
            block()->i32 {
                when gt_i32_u(
                        local_load_i32_s(a)
                        imm_i32(85)
                    ) {
                    break(imm_i32(65))  // 'A'
                }

                when gt_i32_u(
                        local_load_i32_s(a)
                        imm_i32(70)
                    ){
                    break(imm_i32(66))  // 'B'
                }

                when gt_i32_u(
                        local_load_i32_s(a)
                        imm_i32(55)
                    ){
                    break(imm_i32(67))  // 'C'
                }

                imm_i32(68)             // 'D'
            }
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(90)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(65)]);

    let result1 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(80)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::U32(66)]);

    let result2 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(70)],
    );
    assert_eq!(result2.unwrap(), vec![ForeignValue::U32(67)]);

    let result3 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(60)],
    );
    assert_eq!(result3.unwrap(), vec![ForeignValue::U32(67)]);

    let result4 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(50)],
    );
    assert_eq!(result4.unwrap(), vec![ForeignValue::U32(68)]);

    let result5 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(40)],
    );
    assert_eq!(result5.unwrap(), vec![ForeignValue::U32(68)]);
}

#[test]
fn test_assemble_control_flow_structure_branch_without_default_arm() {
    // note
    // this test requires the instruction 'panic'

    // fn level (i32) -> (i32)
    //     block ()->(i32)              ;; type idx 1
    //                                  ;; case 1
    //         local_load32(0, 0)
    //         imm_i32(85)
    //         gt_i32_u
    //         block_nez ()->()         ;; type idx 2
    //             imm_i32(65)          ;; 'A' (85, 100]
    //             break(1)
    //         end
    //                                  ;; case 2
    //         local_load32(0, 0)
    //         imm_i32(70)
    //         gt_i32_u
    //         block_nez ()->()         ;; type idx 3
    //             imm_i32(66)          ;; 'B' (70,85]
    //             break(1)
    //         end
    //         panic
    //     end
    // end
    //
    // assert (90) -> (65) 'A'
    // assert (80) -> (66) 'B'
    // assert (70) -> panic
    // assert (60) -> panic

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(a:i32) -> i32
        {
            block()-> i32 {
                when gt_i32_u(
                        local_load_i32_s(a)
                        imm_i32(85)
                    ) {
                    break(imm_i32(65))    // 'A'
                }

                when gt_i32_u(
                        local_load_i32_s(a)
                        imm_i32(70)
                    ) {
                    break(imm_i32(66))    // 'B'
                }

                panic(0x100)
            }
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);

    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(90)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(65)]);

    let result1 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(80)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::U32(66)]);

    let result2 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(70)],
    );
    assert!(matches!(
        result2,
        Err(HandlerError {
            error_type: HandleErrorType::Panic(0x100)
        })
    ));

    let result3 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(60)],
    );
    assert!(matches!(
        result3,
        Err(HandlerError {
            error_type: HandleErrorType::Panic(0x100)
        })
    ));
}

#[test]
fn test_assemble_control_flow_structure_loop() {
    // fn accu (n/0:i32) -> (i32)
    //     [local sum/1:i32]
    //     block ()->()
    //                                  ;; break if n==0
    //         local_load32(1, 0)
    //         eqz_i32
    //         block_nez
    //             break(1)
    //         end
    //                                  ;; sum = sum + n
    //         local_load32(1, 0)
    //         local_load32(1, 1)
    //         add_i32
    //         local_store_i32(1, 1)
    //                                  ;; n = n - 1
    //         local_load32(1, 0)
    //         sub_imm_i32(1)
    //         local_store_i32(1, 0)
    //                                  ;; recur
    //         (recur 0)
    //     end
    //     (local_load32 0 1)
    // end
    //
    // assert (10) -> (55)
    // assert (100) -> (5050)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(n:i32) -> i32
            [sum:i32]
        {
            block {
                when eqz_i32(local_load_i32_s(n))
                    break()

                local_store_i32(sum
                    add_i32(
                        local_load_i32_s(sum)
                        local_load_i32_s(n)
                    )
                )

                local_store_i32(n
                    sub_imm_i32(
                        1
                        local_load_i32_s(n)
                    )
                )

                recur()
            }
            local_load_i32_s(sum)
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(10)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(55)]);

    let result1 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(100)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::U32(5050)]);
}

#[test]
fn test_assemble_control_flow_structure_loop_with_block_parameters() {
    // fn accu (count/0:i32) -> (i32)
    //     imm_i32(0)                   ;; sum
    //     local_load32(0, 0)           ;; count
    //     block                        ;; (sum/0:i32, n/1:i32)->(i32)
    //                                  ;; break if n==0
    //         local_load32(0, 1)
    //         eqz_i32
    //         block_nez
    //             local_load32(0, 1)
    //             break(1)
    //         end
    //                                  ;; sum + n
    //         local_load32(0, 0)
    //         local_load32(0, 1)
    //         add_i32
    //                                  ;; n - 1
    //         local_load32(0, 1)
    //         sub_imm_i32(1)
    //                                  ;; recur
    //         recur(0)
    //     end
    // end
    //
    // assert (10) -> (55)
    // assert (100) -> (5050)

    let binary0 = helper_make_single_module_app(
        r#"
        fn test(count:i32) -> i32
        {
            block(
                sum:i32 = imm_i32(0)
                n:i32 = local_load_i32_s(count)
                )->i32 {

                when eqz_i32(local_load_i32_s(n))
                    break(local_load_i32_s(sum))

                recur(
                    add_i32(
                        local_load_i32_s(sum)
                        local_load_i32_s(n)
                    )

                    sub_imm_i32(
                        1
                        local_load_i32_s(n)
                    )
                )
            }
        }
        "#,
    );

    let handler = Handler::new();
    let resource0 = InMemoryResource::new(vec![binary0]);
    let process_context0 = resource0.create_process_context().unwrap();
    let mut thread_context0 = process_context0.create_thread_context();

    let result0 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(10)],
    );
    assert_eq!(result0.unwrap(), vec![ForeignValue::U32(55)]);

    let result1 = process_function(
        &handler,
        &mut thread_context0,
        0,
        0,
        &[ForeignValue::U32(100)],
    );
    assert_eq!(result1.unwrap(), vec![ForeignValue::U32(5050)]);
}

// #[test]
// fn test_assemble_control_flow_structure_loop_with_if() {
//     // fn $accu (count/0:i32) -> (i32)
//     //     zero                     ;; sum
//     //     (local_load32 0 0)       ;; count
//     //     (block 1 1) (sum/0:i32, n/1:i32)->(i32)
//     //                              ;; if n==0
//     //         (local_load32 0 1)
//     //         i32_eqz
//     //         (block_alt)
//     //             (local_load32 0 1)
//     //             (break 1)
//     //         (break 0)
//     //                              ;; sum + n
//     //             (local_load32 0 0)
//     //             (local_load32 0 1)
//     //             i32_add
//     //                              ;; n - 1
//     //             (local_load32 0 1)
//     //             (i32_dec 1)
//     //                              ;; recur
//     //             (recur 0)
//     //         end
//     //     end
//     // end
//     //
//     // assert (10) -> (55)
//     // assert (100) -> (5050)
//
//     let binary0 = helper_make_single_module_app(
//         r#"
//         (module $app
//             (runtime_version "1.0")
//             (function $test
//                 (param $count i32)
//                 (results i32)
//                 (code
//                     zero                        // for arg 'sum'
//                     local_load_i32_s(count)   // for arg 'n'
//                     (for
//                         (param $sum i32)
//                         (param $n i32)
//                         (result i32)
//                         (do
//                             (if
//                                 (i32.eqz local_load_i32_s(n))
//                                 (break local_load_i32_s(sum))
//                                 (recur
//                                     (add_i32
//                                         local_load_i32_s(sum)
//                                         local_load_i32_s(n)
//                                     )
//
//                                     (i32.dec
//                                         local_load_i32_s(n)
//                                         1
//                                     )
//                                 )
//                             )
//                         )
//                     )
//                 )
//             )
//         )
//         "#,
//     );
//
//     let handler = Handler::new();
//     let resource0 = InMemoryResource::new(vec![binary0]);
//     let process_context0 = resource0.create_process_context().unwrap();
//
//     let function_entry = process_context0.module_images[0]
//         .get_function_section()
//         .get_function_entry(0);
//
//     let bytecode_text = format_bytecode_as_text(&function_entry.code);
//     // println!("{}", bytecode_text);
//
//     assert_eq!(
//         bytecode_text,
//         "\
// 0x0000  01 01                       zero
// 0x0002  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
// 0x000a  00 01                       nop
// 0x000c  01 0a 00 00  01 00 00 00    block             type:1   local:1
//         01 00 00 00
// 0x0018  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
// 0x0020  00 06                       i32.eqz
// 0x0022  00 01                       nop
// 0x0024  05 0a 00 00  02 00 00 00    block_alt         type:2   local:2   off:0x28
//         02 00 00 00  28 00 00 00
// 0x0034  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
// 0x003c  02 0a 01 00  3c 00 00 00    break             rev:1   off:0x3c
// 0x0044  02 0a 00 00  32 00 00 00    break             rev:0   off:0x32
// 0x004c  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
// 0x0054  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
// 0x005c  00 07                       add_i32
// 0x005e  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
// 0x0066  08 07 01 00                 i32.dec           1
// 0x006a  00 01                       nop
// 0x006c  03 0a 01 00  54 00 00 00    recur             rev:1   off:0x54
// 0x0074  00 0a                       end
// 0x0076  00 0a                       end
// 0x0078  00 0a                       end"
//     );
//
//     let mut thread_context0 = process_context0.create_thread_context();
//
//     let result0 = process_function(&handler, &mut thread_context0, 0, 0, &[ForeignValue::U32(10)]);
//     assert_eq!(result0.unwrap(), vec![ForeignValue::U32(55)]);
//
//     let result1 = process_function(&mut thread_context0, 0, 0, &[ForeignValue::U32(100)]);
//     assert_eq!(result1.unwrap(), vec![ForeignValue::U32(5050)]);
// }
//
// #[test]
// fn test_assemble_control_flow_function_tail_call() {
//     // fn $accu (sum/0:i32, n/1:i32) -> (i32)
//     //                              ;; sum = sum + n
//     //     (local_load32 0 0)
//     //     (local_load32 0 1)
//     //     i32_add
//     //     (local_store32 0 0)
//     //                              ;; n = n - 1
//     //     (local_load32 0 1)
//     //     (i32_dec 1)
//     //     (local_store32 0 1)
//     //                              ;; if n > 0 recur (sum,n)
//     //     (local_load32 0 1)
//     //     zero
//     //     i32_gt_u
//     //     (block_nez 1) () -> ()
//     //         (local_load32 0 0)
//     //         (local_load32 0 1)
//     //         (recur 1)
//     //     end
//     //     (local_load32 0 0)       ;; load sum
//     // end
//     //
//     // assert (0, 10) -> (55)
//     // assert (0, 100) -> (5050)
//
//     let binary0 = helper_make_single_module_app(
//         r#"
//         (module $app
//             (runtime_version "1.0")
//             (function $test
//                 (param $sum i32)
//                 (param $n i32)
//                 (results i32)
//                 (code
//                     local_store_i32(sum
//                         (add_i32
//                             local_load_i32_s(sum)
//                             local_load_i32_s(n)
//                         )
//                     )
//                     local_store_i32(n
//                         (i32.dec
//                             local_load_i32_s(n)
//                             1
//                         )
//                     )
//                     (when
//                         gt_i32_u(
//                             local_load_i32_s(n)
//                             zero
//                         )
//                         (fnrecur
//                             local_load_i32_s(sum)
//                             local_load_i32_s(n)
//                         )
//                     )
//                     local_load_i32_s(sum)
//                 )
//             )
//         )
//         "#,
//     );
//
//     let handler = Handler::new();
//     let resource0 = InMemoryResource::new(vec![binary0]);
//     let process_context0 = resource0.create_process_context().unwrap();
//
//     let function_entry = process_context0.module_images[0]
//         .get_function_section()
//         .get_function_entry(0);
//
//     let bytecode_text = format_bytecode_as_text(&function_entry.code);
//     // println!("{}", bytecode_text);
//
//     assert_eq!(
//         bytecode_text,
//         "\
// 0x0000  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
// 0x0008  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
// 0x0010  00 07                       add_i32
// 0x0012  09 02 00 00  00 00 00 00    local.store32     rev:0   off:0x00  idx:0
// 0x001a  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
// 0x0022  08 07 01 00                 i32.dec           1
// 0x0026  09 02 00 00  00 00 01 00    local.store32     rev:0   off:0x00  idx:1
// 0x002e  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
// 0x0036  01 01                       zero
// 0x0038  07 06                       i32.gt_u
// 0x003a  00 01                       nop
// 0x003c  04 0a 00 00  01 00 00 00    block_nez         local:1   off:0x26
//         26 00 00 00
// 0x0048  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
// 0x0050  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
// 0x0058  03 0a 01 00  00 00 00 00    recur             rev:1   off:0x00
// 0x0060  00 0a                       end
// 0x0062  02 02 00 00  00 00 00 00    local.load32_i32  rev:0   off:0x00  idx:0
// 0x006a  00 0a                       end"
//     );
//
//     let mut thread_context0 = process_context0.create_thread_context();
//
//     let result0 = process_function(
//         &handler,
//         &mut thread_context0,
//         0,
//         0,
//         &[ForeignValue::U32(0), ForeignValue::U32(10)],
//     );
//     assert_eq!(result0.unwrap(), vec![ForeignValue::U32(55)]);
//
//     let result1 = process_function(
//         &mut thread_context0,
//         0,
//         0,
//         &[ForeignValue::U32(0), ForeignValue::U32(100)],
//     );
//     assert_eq!(result1.unwrap(), vec![ForeignValue::U32(5050)]);
// }
//
// #[test]
// fn test_assemble_control_flow_function_tail_call_with_if() {
//     // fn $accu (sum:i32, n:i32) -> (i32)
//     //     (local_load32 0 1)               ;; load n
//     //     i32_eqz
//     //     (block_alt 1 1) () -> (i32)      ;; if n == 0
//     //         (local_load32 1 0)           ;; then sum
//     //     (break 0)                        ;; else
//     //                                      ;; sum + n
//     //         (local_load32 1 0)
//     //         (local_load32 1 1)
//     //         i32_add
//     //                                      ;; n - 1
//     //         (local_load32 1 1)
//     //         (i32_dec 1)
//     //         (recur 1)                    ;; recur
//     //     end
//     // end
//     //
//     // assert (0, 10) -> (55)
//     // assert (0, 100) -> (5050)
//
//     let binary0 = helper_make_single_module_app(
//         r#"
//         (module $app
//             (runtime_version "1.0")
//             (function $test
//                 (param $sum i32)
//                 (param $n i32)
//                 (results i32)
//                 (code
//                     (if
//                         (result i32)
//                         (i32.eqz
//                             local_load_i32_s(n)
//                         )
//                         local_load_i32_s(sum)
//                         (fnrecur
//                             (add_i32
//                                 local_load_i32_s(sum)
//                                 local_load_i32_s(n)
//                             )
//                             (i32.dec
//                                 local_load_i32_s(n)
//                                 1
//                             )
//
//                         )
//                     )
//                 )
//             )
//         )
//         "#,
//     );
//
//     let handler = Handler::new();
//     let resource0 = InMemoryResource::new(vec![binary0]);
//     let process_context0 = resource0.create_process_context().unwrap();
//
//     let function_entry = process_context0.module_images[0]
//         .get_function_section()
//         .get_function_entry(0);
//
//     let bytecode_text = format_bytecode_as_text(&function_entry.code);
//     // println!("{}", bytecode_text);
//
//     assert_eq!(
//         bytecode_text,
//         "\
// 0x0000  02 02 00 00  00 00 01 00    local.load32_i32  rev:0   off:0x00  idx:1
// 0x0008  00 06                       i32.eqz
// 0x000a  00 01                       nop
// 0x000c  05 0a 00 00  01 00 00 00    block_alt         type:1   local:1   off:0x20
//         01 00 00 00  20 00 00 00
// 0x001c  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
// 0x0024  02 0a 00 00  32 00 00 00    break             rev:0   off:0x32
// 0x002c  02 02 01 00  00 00 00 00    local.load32_i32  rev:1   off:0x00  idx:0
// 0x0034  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
// 0x003c  00 07                       add_i32
// 0x003e  02 02 01 00  00 00 01 00    local.load32_i32  rev:1   off:0x00  idx:1
// 0x0046  08 07 01 00                 i32.dec           1
// 0x004a  00 01                       nop
// 0x004c  03 0a 01 00  00 00 00 00    recur             rev:1   off:0x00
// 0x0054  00 0a                       end
// 0x0056  00 0a                       end"
//     );
//
//     let mut thread_context0 = process_context0.create_thread_context();
//
//     let result0 = process_function(
//         &handler,
//         &mut thread_context0,
//         0,
//         0,
//         &[ForeignValue::U32(0), ForeignValue::U32(10)],
//     );
//     assert_eq!(result0.unwrap(), vec![ForeignValue::U32(55)]);
//
//     let result1 = process_function(
//         &mut thread_context0,
//         0,
//         0,
//         &[ForeignValue::U32(0), ForeignValue::U32(100)],
//     );
//     assert_eq!(result1.unwrap(), vec![ForeignValue::U32(5050)]);
// }
