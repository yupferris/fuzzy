    .lcomm preserveStackPointer, 4

    .section .text
    .align 1

    .global _executeHarness

_executeHarness:
    /* Save out current reg values */
    addi -124, sp, sp
    st.w r1, 0[sp]
    st.w r2, 4[sp]
    st.w r4, 8[sp]
    st.w r5, 12[sp]
    st.w r6, 16[sp]
    st.w r7, 20[sp]
    st.w r8, 24[sp]
    st.w r9, 28[sp]
    st.w r10, 32[sp]
    st.w r11, 36[sp]
    st.w r12, 40[sp]
    st.w r13, 44[sp]
    st.w r14, 48[sp]
    st.w r15, 52[sp]
    st.w r16, 56[sp]
    st.w r17, 60[sp]
    st.w r18, 64[sp]
    st.w r19, 68[sp]
    st.w r20, 72[sp]
    st.w r21, 76[sp]
    st.w r22, 80[sp]
    st.w r23, 84[sp]
    st.w r24, 88[sp]
    st.w r25, 92[sp]
    st.w r26, 96[sp]
    st.w r27, 100[sp]
    st.w r28, 104[sp]
    st.w r29, 108[sp]
    st.w r30, 112[sp]
    st.w r31, 116[sp]
    stsr psw, r1
    st.w r1, 120[sp]

    /* Preserve sp */
    movhi hi(preserveStackPointer), r0, r1
    movea lo(preserveStackPointer), r1, r1
    st.w sp, 0[r1]

    /* Load jump addr arg into r30. Unfortunately this means no initial value for r30, but that's ok. */
    mov r6, r30

    /* Load initial reg values, minus r30 and r31 */
    initialRegValues = 0x0001e000
    movhi hi(initialRegValues), r0, r31
    movea lo(initialRegValues), r31, r31

    ld.w 0[r31], r0
    ld.w 4[r31], r1
    ld.w 8[r31], r2
    ld.w 12[r31], r3
    ld.w 16[r31], r4
    ld.w 20[r31], r5
    ld.w 24[r31], r6
    ld.w 28[r31], r7
    ld.w 32[r31], r8
    ld.w 36[r31], r9
    ld.w 40[r31], r10
    ld.w 44[r31], r11
    ld.w 48[r31], r12
    ld.w 52[r31], r13
    ld.w 56[r31], r14
    ld.w 60[r31], r15
    ld.w 64[r31], r16
    ld.w 68[r31], r17
    ld.w 72[r31], r18
    ld.w 76[r31], r19
    ld.w 80[r31], r20
    ld.w 84[r31], r21
    ld.w 88[r31], r22
    ld.w 92[r31], r23
    ld.w 96[r31], r24
    ld.w 100[r31], r25
    ld.w 104[r31], r26
    ld.w 108[r31], r27
    ld.w 112[r31], r28
    ld.w 116[r31], r29

    /* Clear PSW */
    ldsr r0, psw

    /* Actual test stuff :) */

    /* Set link reg by hand because we only have relative JAL */
    movhi hi(executeRet), r0, r31
    movea lo(executeRet), r31, r31

    jmp [r30]

executeRet:
    /* Output result reg values (minus r31 but including psw) */
    stsr psw, r31
    ldsr r31, fepsw

    resultRegValues = initialRegValues + 32 * 4
    movhi hi(resultRegValues), r0, r31
    movea lo(resultRegValues), r31, r31

    st.w r0, 0[r31]
    st.w r1, 4[r31]
    st.w r2, 8[r31]
    st.w r3, 12[r31]
    st.w r4, 16[r31]
    st.w r5, 20[r31]
    st.w r6, 24[r31]
    st.w r7, 28[r31]
    st.w r8, 32[r31]
    st.w r9, 36[r31]
    st.w r10, 40[r31]
    st.w r11, 44[r31]
    st.w r12, 48[r31]
    st.w r13, 52[r31]
    st.w r14, 56[r31]
    st.w r15, 60[r31]
    st.w r16, 64[r31]
    st.w r17, 68[r31]
    st.w r18, 72[r31]
    st.w r19, 76[r31]
    st.w r20, 80[r31]
    st.w r21, 84[r31]
    st.w r22, 88[r31]
    st.w r23, 92[r31]
    st.w r24, 96[r31]
    st.w r25, 100[r31]
    st.w r26, 104[r31]
    st.w r27, 108[r31]
    st.w r28, 112[r31]
    st.w r29, 116[r31]
    st.w r30, 120[r31]

    stsr fepsw, r1
    st.w r1, 124[r31]

    /* Restore reg values */
    movhi hi(preserveStackPointer), r0, r1
    movea lo(preserveStackPointer), r1, r1
    ld.w 0[r1], sp

    ld.w 120[sp], r1
    ldsr r1, psw
    ld.w 0[sp], r1
    ld.w 4[sp], r2
    ld.w 8[sp], r4
    ld.w 12[sp], r5
    ld.w 16[sp], r6
    ld.w 20[sp], r7
    ld.w 24[sp], r8
    ld.w 28[sp], r9
    ld.w 32[sp], r10
    ld.w 36[sp], r11
    ld.w 40[sp], r12
    ld.w 44[sp], r13
    ld.w 48[sp], r14
    ld.w 52[sp], r15
    ld.w 56[sp], r16
    ld.w 60[sp], r17
    ld.w 64[sp], r18
    ld.w 68[sp], r19
    ld.w 72[sp], r20
    ld.w 76[sp], r21
    ld.w 80[sp], r22
    ld.w 84[sp], r23
    ld.w 88[sp], r24
    ld.w 92[sp], r25
    ld.w 96[sp], r26
    ld.w 100[sp], r27
    ld.w 104[sp], r28
    ld.w 108[sp], r29
    ld.w 112[sp], r30
    ld.w 116[sp], r31
    addi 124, sp, sp

    /* Return! */
    jmp [r31]
