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

@ R0 = dst
@ R1 = src
@ R2 = cnt
ENTRY(memmove)
	movs r2, r2
	bxeq lr

	@ TODO: Have memmove4 for 4 byte aligned versions	

	@ If DST < SRC: Check for overlap
	cmp r0, r1
	bxeq lr	@ if DST==SRC, return early
	blt __aeabi_memcpy
	@ SRC < DST: Possible slow path
	add r3, r1, r2
	cmp r3, r0
	@ If SRC + CNT <= DST, go the fast path
	ble __aeabi_memcpy
	@ Reverse slow path
	add r1, r2
	add r0, r2
1:
	LDRB r3, [r1, #-1]!
	STRB r3, [r0, #-1]!
	SUBS r2, r2, #1
	BNE 1b
	BX lr

ENTRY(memset)
	.fnstart
	movs r2, r2
	beq 2f
1:
	strb r1, [r0], #1
	subs r2, #1
	bne 1b
2:
	bx lr
	.fnend

#if 1
ENTRY(memcpy)
	movs r2, r2
	bxeq lr
1:
	LDRB r3, [r1], #1
	STRB r3, [r0], #1
	SUBS r2, r2, #1
	BGT 1b
	BX lr
#else
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
	.fnstart
	movs r1, r1
	beq 2f
1:
	strb r2, [r0], #1
	subs r1, #1
	bne 1b
2:
	bx lr
	.fnend
ENTRY(__aeabi_memclr)
	MOV r2, #0
	b __aeabi_memset


@ 64-bit divide/modulo
ENTRY(__aeabi_uldivmod)
	push {r4-r7,lr}
	@ R1:R0 = Numerator
	@ R3:R2 = Denominator
	
	@ 1. Find the largest power of two factor of n and d
	@let mut add = 1;
	mov r6,#1
	mov r7,#0
	@ -- Save n/2 in r5:r4
	lsrs r5, r0,#1
	lsr r4, r0,#1
	@while n / 2 >= d && add != 0 {
1:
	cmp r9,r3
	blt 2f
	cmp r8,r2
	blt 2f
	@  d <<= 1;
	lsls r2, r2, #1
	lsl r3, r3, #1
	addcs r3, r3, #1
	@  add <<= 1;
	lsls r6, r6, #1
	lsl r7, r7, #1
	addcs r7, r7, #1
	bcs _div0
	@}
	b 1b
2:
	@let mut ret = 0;
	mov r4,#0
	mov r5,#0
	@while add > 0 {
1:
	movs r7,r7
	moveqs r6,r6
	beq 3f
	@  if n >= d {
	cmp r1,r3
	blt 2f
	cmp r0,r2
	blt 2f
	@    ret += add;
	orr r4, r4,r6
	orr r5, r5,r7
	@    n -= d;
	subs r0,r2
	sbc r1,r3
	@  }
2:
	@  add >>= 1;
	lsrs r7, r7, #1
	rrx r6, r6
	@  d >>= 1;
	lsrs r3, r3, #1
	rrx r2, r2
	@}
	b 1b
3:
	@ r2:r3 = Remainder (n)
	mov r2, r0
	mov r3, r1
	@ r0:r1 = Quotient (ret)
	mov r0, r4
	mov r1, r5
_div0:
	pop {r4-r7,pc}
@ 64-bit signed divide/modulo
ENTRY(__aeabi_ldivmod)
	push {r4, lr}
	mov r4, #0
	@ if num < 0 { sign ^= 1; num = -num; }
	tst r1,#0x80000000
	beq 1f
	eor r4, #1
	mvn r0,r0
	adds r0, #1
	mvn r1,r1
	adc r1, #0
1:
	@ if den < 0 { sign ^= 1; den = -den; }
	tst r3,#0x80000000
	beq 1f
	eor r4, #1
	mvn r2,r2
	adds r2, #1
	mvn r3,r3
	adc r3, #0
1:
	bl __aeabi_uldivmod
	@ if sign != 0 {
	movs r4, r4
	beq 1f
	@   quo = -quo
	mvn r0,r0
	mvn r1,r1
	adds r0, #1
	adc r1, #0
	@   rem = -rem
	mvn r2,r2
	mvn r3,r3
	adds r2, #1
	adc r3, #0
	@ }
1:
	pop {r4, pc}
@ 32-bit divide/modulo
ENTRY(__aeabi_uidivmod)
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
	push {r4, lr}
	mov r4, #0
	@ if num < 0 { sign ^= 1; num = -num; }
	tst r0,#0x80000000
	beq 1f
	eor r4, #1
	mvn r0,r0
	add r0, #1
1:
	@ if den < 0 { sign ^= 1; den = -den; }
	tst r1,#0x80000000
	beq 1f
	eor r4, #1
	mvn r1,r1
	add r1, #1
1:
	bl __aeabi_uidivmod
	@ if sign != 0 {
	movs r4, r4
	beq 1f
	@   quo = -quo
	mvn r0,r0
	add r0, #1
	@   rem = -rem
	mvn r1,r1
	add r1, #1
	@ }
1:
	pop {r4, pc}
@@ 32-bit unsigned divide
@ENTRY(__aeabi_uidiv)
@	b __aeabi_uidivmod
@@ 32-bit unsigned modulo
@ENTRY(__umoddi3)
@	push {lr}
@	bl __aeabi_uidivmod
@	mov r0, r1
@	pop {pc}
@@ 32-bit signed modulo
@ENTRY(__umodsi3)
@	push {lr}
@	bl __aeabi_idivmod
@	mov r0, r1
@	pop {pc}

#endif

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
	movlo r0, #-1
	moveq r0, #0
	pop {r4}
	mov pc, lr
EXIDX memcmp, 0x80A0B0B0

ENTRY(__aeabi_unwind_cpp_pr0)
ENTRY_(__aeabi_unwind_cpp_pr1)
	b .


#if 0
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
#endif
EXIDX __aeabi_unwind_cpp_pr0, 0x1

ENTRY(_Unwind_Resume)
	b .

@ vim: ft=armasm
