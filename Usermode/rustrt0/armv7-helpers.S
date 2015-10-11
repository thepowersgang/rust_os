@
@ MAGIC MACRO
@
.macro EXIDX method handle
.long EXIDX_\method
.section .ARM.exidx.\method, #exidx
.globl EXIDX_\method
EXIDX_\method: .long \method - . - 0x80000000, \handle
.section .text
.endm

.section .text
ENTRY(__aeabi_memcpy4)
ENTRY(__aeabi_memcpy8)
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
ENTRY(__aeabi_memset8)
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
ENTRY(__aeabi_memclr8)
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


ENTRY(__aeabi_uldivmod)
.extern __aeabi_uldivmod_
	push {lr}
	sub sp, #16
	push {sp}
	bl __aeabi_uldivmod_
	add sp, #4
	pop {r0, r1, r2, r3}
	pop {pc}
ENTRY(__aeabi_uidiv)
	b .
ENTRY(__umodsi3)
	b .
ENTRY(__umoddi3)
	b .
ENTRY(__aeabi_uldivmod_)
	b .

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
ENTRY(__aeabi_unwind_cpp_pr1)
	b .


ENTRY(__aeabi_dcmplt)
ENTRY(__aeabi_dcmple)
ENTRY(__aeabi_dcmpeq)
ENTRY(__aeabi_dcmpge)
ENTRY(__aeabi_dcmpgt)
	b .
ENTRY(__aeabi_fcmplt)
ENTRY(__aeabi_fcmple)
ENTRY(__aeabi_fcmpeq)
ENTRY(__aeabi_fcmpge)
ENTRY(__aeabi_fcmpgt)
	b .
EXIDX __aeabi_unwind_cpp_pr0, 0x1

ENTRY(_Unwind_Resume)
	b .

@ vim: ft=armasm
