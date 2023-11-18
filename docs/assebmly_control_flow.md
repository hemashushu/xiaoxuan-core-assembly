# XiaoXuan Core Assembly Control Flow

<!-- @import "[TOC]" {cmd="toc" depthFrom=1 depthTo=6 orderedList=false} -->

(when TEST CONSEQUENT)

(if (result...)
    TEST CONSEQUENT ALTERNATE
)

(branch (result...)
    (case TEST_0 CONSEQUENT_0)
    ...
    (case TEST_N CONSEQUENT_N)
    (default CONSEQUENT_DEFAULT) ;; optional
)

(for (param...) (result...)
    (local...)
    INSTRUCTION
)

TEST, CONSEQUENT, ALTERNATE, INSTRUCTION can be:

- instructions, e.g. `(i32.eq ...)`
- structures, e.g. `(if ...)`, `(for ...)`
- statements, e.g. `(do ...)`, `(return ...)`

statements:

break the nearest 'for'
(break VALUE_0 VALUE_1 ... VALUE_N )

recur to the nearest 'for'
(recur VALUE_0 VALUE_1 ... VALUE_N )

break all blocks and the function, and return values to the function caller
(return VALUE_0 VALUE_1 ... VALUE_N )

re-run the function with new args
(rerun VALUE_0 VALUE_1 ... VALUE_N )