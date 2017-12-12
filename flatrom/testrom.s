    .section .entry
    .align 1

    .global _entry

_entry:
    /* Jump into stub */
    /*  This ensures the vxe's entry point is always at the beginning of the file */
    /*  r30 is used as it's not a user-provided initial value */
    movhi hi(_stub), r0, r30
    movea lo(_stub), r30, r30
    jmp [r30]

    .section .text
    .align 1

    .global _stub

_stub:
    /* Disable cache */
    ldsr r0, chcw

    /* Clear .bss section and uninitialized RAM */
    movhi hi(__bss_start), r0, r1
    movea lo(__bss_start), r1, r1
    movhi hi(__bss_end), r0, r7
    movea lo(__bss_end), r7, r7
    jr end_init_bss
top_init_bss:
    st.h r0, 0[r1]
    add 1, r1
end_init_bss:
    cmp r7, r1
    blt top_init_bss

    /* Clear .sram section and uninitialized SRAM */
    movhi hi(__sram_start), r0, r1
    movea lo(__sram_start), r1, r1
    movhi hi(__sram_end), r0, r7
    movea lo(__sram_end), r7, r7
    jr end_init_sram
top_init_sram:
    st.b r0, 0[r1]
    add 1, r1
end_init_sram:
    cmp r7, r1
    blt top_init_sram

    /* Set up sp, fp, gp, and tp */
    movhi hi(__stack), r0, sp
    movea lo(__stack), sp, sp

    movhi hi(__gp), r0, gp
    movea lo(__gp), gp, gp

    movhi hi(__tp), r0, tp
    movea lo(__tp), tp, tp

    /* Jump to c main */
    movhi hi(_main), r0, r1
    movea lo(_main), r1, r1
    jmp [r1]
