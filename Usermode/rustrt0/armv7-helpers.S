@
@ MAGIC MACRO
@
.macro EXIDX method handle
.long EXIDX_\method
.pushsection .ARM.exidx.\method, #exidx
.globl EXIDX_\method
EXIDX_\method: .long \method - . - 0x80000000, \handle
.popsection
.endm

#define ENTRY_(v)	.globl v ; v:

.macro USER_LOG message
	ldr r0, =9f
	mov r1, #(10f - 9f)
	mov r12, #0
	svc #0
.pushsection .rodata
9:	.ascii "\message"
10:
.popsection
.endm

.macro TODO_STUB v
ENTRY(\v)
	USER_LOG "TODO: \v"
	b .
.endm

.section .text

TODO_STUB __aeabi_memmove4
EXIDX __aeabi_memmove4, 0x80B0B0B0

ENTRY(__aeabi_memcpy4)
ENTRY_(__aeabi_memcpy8)
	movs r2, r2
	bxeq lr
1:
	LDR r3, [r1], #4
	STR r3, [r0], #4
	SUBS r2, r2, #4
	BGT 1b
	BX lr
EXIDX __aeabi_memcpy4, 0x80B0B0B0

ENTRY(__aeabi_memcpy)
	movs r2, r2
	bxeq lr
1:
	LDRB r3, [r1], #1
	STRB r3, [r0], #1
	SUBS r2, r2, #1
	BGT 1b
	BX lr
ENTRY(memcpy)
	b __aeabi_memcpy
EXIDX __aeabi_memcpy, 0x80B0B0B0

ENTRY(__aeabi_memset4)
ENTRY_(__aeabi_memset8)
	movs r1, r1
	bxeq lr

	LSL r3, r2, #8
	ORR r2, r2, r3
	LSL r3, r2, #16
	ORR r2, r2, r3
1:
	STR r2, [r0], #4
	SUBS r1, r1, #4
	BGT 1b
	BX lr
EXIDX __aeabi_memset4, 0x80B0B0B0

ENTRY(__aeabi_memclr4)
ENTRY_(__aeabi_memclr8)
	MOV r2, #0
	b 1b

ENTRY(__aeabi_memset)
	movs r1, r1
	beq 2f
1:
	strb r2, [r0], #1
	subs r1, #1
	bne 1b
2:
	bx lr
ENTRY(memset)
	b __aeabi_memset
EXIDX __aeabi_memset, 0x80B0B0B0

ENTRY(__aeabi_memclr)
	MOV r2, #0
	b __aeabi_memset


@ 64-bit divide/modulo
ENTRY(__aeabi_uldivmod)
	USER_LOG "TODO: __aeabi_uldivmod"
	b .
@ 64-bit signed divide/modulo
ENTRY(__aeabi_ldivmod)
	USER_LOG "TODO: __aeabi_ldivmod"
	b .
@ 32-bit divide/modulo
ENTRY(__aeabi_uidivmod)
	USER_LOG "TODO: __aeabi_uidivmod"
	b .
 	push	{r4, lr}
 	lsr	r2, r0, #1
 	mov	ip, #0
 	mov	r4, #1
 	cmp	r2, r1
 	bcs	1f
 	mov	r3, #1
 	mov	r2, #0
 	b	2f
1: 	mov	lr, #0
1: 	lsl	r3, r4, #1
 	lsl	r1, r1, #1
 	cmp	lr, r4, lsl #1
 	beq	1f
 	mov	r4, r3
 	cmp	r2, r1
 	bcs	1b
1:	mov	r2, #0
 	cmp	r3, #0
 	beq	3f
2:	cmp	r0, r1
 	lsr	r4, r3, #1
 	subcs	r0, r0, r1
 	addcs	r2, r2, r3
 	cmp	ip, r3, lsr #1
 	lsr	r1, r1, #1
 	mov	r3, r4
 	bne	2b
3:
	mov	r1, r0	@ R1 is remainder
	mov	r0, r2	@ R0 is quotient
	pop	{r4, pc}
@ 32-bit signed divide/modulo
ENTRY(__aeabi_idivmod)
	USER_LOG "TODO: __aeabi_idivmod"
	b .
@ 32-bit unsigned divide
ENTRY(__aeabi_uidiv)
	b __aeabi_uidivmod
@ 32-bit unsigned modulo
ENTRY(__umoddi3)
	push {lr}
	bl __aeabi_uidivmod
	mov r0, r1
	pop {pc}
@ 32-bit signed modulo
ENTRY(__umodsi3)
	push {lr}
	bl __aeabi_idivmod
	mov r0, r1
	pop {pc}

// A, B, num
ENTRY(memcmp)
	push {r4}
	movs r2,r2
	mov r3, #0
	mov r4, #0
	beq 2f
1:
	ldrb r3, [r0], #1
	ldrb r4, [r1], #1
	cmp r4, r3
	bne 2f
	subs r2, #1
	bne 1b
2:
	movhs r0, #1
	moveq r0, #0
	movlo r0, #-1
	pop {r4}
	mov pc, lr
EXIDX memcmp, 0x80A0B0B0

ENTRY(__aeabi_unwind_cpp_pr0)
ENTRY_(__aeabi_unwind_cpp_pr1)
	b .


TODO_STUB __aeabi_dcmplt
TODO_STUB __aeabi_dcmple
TODO_STUB __aeabi_dcmpeq
TODO_STUB __aeabi_dcmpge
TODO_STUB __aeabi_dcmpgt
TODO_STUB __aeabi_fcmplt
TODO_STUB __aeabi_fcmple
TODO_STUB __aeabi_fcmpeq
TODO_STUB __aeabi_fcmpge
TODO_STUB __aeabi_fcmpgt
	b .
EXIDX __aeabi_unwind_cpp_pr0, 0x1

ENTRY(_Unwind_Resume)
	b .

@ vim: ft=armasm
