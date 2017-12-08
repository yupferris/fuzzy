    .section .text
    .align 1

    .global _entry

_entry:
    movea 0xffff, r0, r1
    add 1, r1

    movhi hi(0xfadebabe), r0, r6
    movea lo(0xfadebabe), r6, r6

    /* Return to test harness */
    jmp [r31]
