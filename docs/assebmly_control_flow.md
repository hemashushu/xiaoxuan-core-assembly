# XiaoXuan Core Assembly Control Flow

<!-- @import "[TOC]" {cmd="toc" depthFrom=1 depthTo=6 orderedList=false} -->

(when TEST (local...) CONSEQUENT)

(if TEST (result...) (local...)
    CONSEQUENT ALTERNATE)

(branch (result...) (local...)
    (case TEST_0 CONSEQUENT_0)
    ...
    (case TEST_N CONSEQUENT_N)
    (default CONSEQUENT_DEFAULT) ;; optional
    )

(for (param...) (result...) (local...) INSTRUCTION)

