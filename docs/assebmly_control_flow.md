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

TEST, CONSEQUENT, ALTERNATE, INSTRUCTION can be `(do INST_0 INST_1 ... INST_N)`
