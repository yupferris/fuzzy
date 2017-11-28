    .section .text
    .align 1

    .global _start

_start:
    /* Wait for WRAM reset */
    movea 0x2000, r0, r1
wait_for_wram_loop:
    add -1, r1
    bnz wait_for_wram_loop

    /* Clear PSW */
    ldsr r0, psw

    /* Disable cache */
    ldsr r0, chcw

    /* initiallize .data section */
    movhi   hi(__data_start), r0, r7
    movea   lo(__data_start), r7, r7
    movhi   hi(__data_end),   r0, r1
    movea   lo(__data_end),   r1, r1
    movhi   hi(__data_vma),   r0, r6
    movea   lo(__data_vma),   r6, r6
    jr      end_init_data

top_init_data:
    ld.b    0[r7], r8
    st.b    r8,    0[r6]
    add     1,     r7
    add     1,     r6
end_init_data:
    cmp     r1,    r6
    blt     top_init_data

/* clear .bss section and unintialized RAM */
    movhi   hi(__bss_start), r0, r1
    movea   lo(__bss_start), r1, r1
    movhi   hi(__bss_end),   r0, r7
    movea   lo(__bss_end),   r7, r7
    jr      end_init_bss
top_init_bss:
    st.h    r0, 0[r1]
    add     1,  r1
end_init_bss:
    cmp     r7, r1
    blt     top_init_bss

/* clear .sram section and unintialized SRAM */
    movhi   hi(__sram_start),   r0, r1
    movea   lo(__sram_start),   r1, r1
    movhi   hi(__sram_end),     r0, r7
    movea   lo(__sram_end),     r7, r7
    jr      end_init_sram
top_init_sram:
    st.b    r0, 0[r1]
    add     1,  r1
end_init_sram:
    cmp     r7, r1
    blt     top_init_sram

    /* Clear PSW */
    ldsr r0, psw

    /* Disable cache */
    ldsr r0, chcw

    /* Setup sp, fp, gp, and tp */
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

    /* Fun stuff happens if main returns.. :D */

    .section ".vbvectors", "ax"
    .align 1

.global _rom_title

/* Rom info table (07FFFDE0h) */
_rom_title:
    .ascii "change this title   "     /* Game Title          */
    .byte 0x00,0x00,0x00,0x00,0x00    /* Reserved            */
    .ascii "MFGMID"                   /* Manufacture/Game ID */
    .byte 0x01                        /* Rom Version         */

/* Hardware Interupt Vectors */
interrupt_table:

    /* INTKEY (7FFFE00h) - Controller Interrupt */
    reti
    .fill   0x0E

    /* INTTIM (7FFFE10h) - Timer Interrupt */
    reti
    .fill   0x0E

    /* INTCRO (7FFFE20h) - Expansion Port Interrupt */
    reti
    .fill   0x0E

    /* INTCOM (7FFFE30h) - Link Port Interrupt */
    reti
    .fill   0x0E

    /* INTVPU (7FFFE40h) - Video Retrace Interrupt */
    reti
    .fill   0x0E

    /* Unused vectors (7FFFE50h-7FFFF5Fh) */
    .fill   0x010F

    /* (7FFFF60h) - Float exception */
    reti
    .fill   0x0E

    /* Unused vector */
    .fill   0x10

    /* (7FFFF80h) - Divide by zero exception */
    reti
    .fill   0x0E

    /* (7FFFF90h) - Invalid Opcode exception */
    reti
    .fill   0x0E

    /* (7FFFFA0h) - Trap 0 exception */
    reti
    .fill   0x0E

    /* (7FFFFB0h) - Trap 1 exception */
    reti
    .fill   0x0E

    /* (7FFFFC0h) - Trap Address exception */
    reti
    .fill   0x0E

    /* (7FFFFD0h) - NMI/Duplex exception */
    reti
    .fill   0x0F

    /* Unused vector */
    .fill   0x10

    /* Reset Vector (7FFFFF0h) - This is how the ROM boots */
    movhi   hi(_start), r0, r1
    movea   lo(_start), r1, r1
    jmp     [r1]
    .fill   0x06
