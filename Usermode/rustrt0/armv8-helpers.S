.macro EXIDX method handle
/*
.long EXIDX_\method
.section .ARM.exidx.\method, #exidx
.globl EXIDX_\method
EXIDX_\method: .long \method - . - 0x80000000, \handle
.section .text
*/
.endm

.section .text
ENTRY(__aeabi_memcpy4)
ENTRY(__aeabi_memcpy8)
	tst x2,x2
	beq 2f
1:
	LDR w3, [x1], #4
	STR w3, [x0], #4
	SUBS x2, x2, #4
	BGT 1b
2:
	ret
EXIDX __aeabi_memcpy4, 0x80B0B0B0

ENTRY(__aeabi_memcpy)
	tst x2,x2
	beq 2f
1:
	LDRB w3, [x1], #1
	STRB w3, [x0], #1
	SUBS x2, x2, #1
	BGT 1b
2:
	ret
ENTRY(memcpy)
	b __aeabi_memcpy
EXIDX __aeabi_memcpy, 0x80B0B0B0

ENTRY(memmove)
	tst x2,x2
	beq 2f
	cmp x0,x1
	blt __aeabi_memcpy	// DST < SRC, memcpy
	add x3,x1,x2
	cmp x0,x3
	bgt __aeabi_memcpy	// DST > SRC+NUM, memcpy
	add x0,x0,x2
	add x1,x1,x2
1:	// Reverse copy
	LDRB w3, [x1,#-1]!
	STRB w3, [x0,#-1]!
	SUBS x2, x2, #1
	BGT 1b
2:
	ret
EXIDX memmove, 0x80B0B0B0

ENTRY(__aeabi_memset8)
	//.fnstart
	tst x1, x1
	beq 2f

	LSL x3, x2, #8
	ORR x2, x2, x3
	LSL x3, x2, #16
	ORR x2, x2, x3
	LSL x3, x2, #32
	ORR x2, x2, x3
1:
	STR x2, [x0], #8
	SUBS x1, x1, #8
	BGT 1b
2:
	ret
	//.fnend
ENTRY(__aeabi_memset4)
	//.fnstart
	tst x1, x1
	beq 2f

	LSL w3, w2, #8
	ORR w2, w2, w3
	LSL w3, w2, #16
	ORR w2, w2, w3
1:
	STR w2, [x0], #4
	SUBS x1, x1, #4
	BGT 1b
2:
	ret
	//.fnend

ENTRY(__aeabi_memclr4)
ENTRY(__aeabi_memclr8)
	MOV x2, #0
	b 1b
ENTRY(__aeabi_memset)
	tst x1, x1
	beq 2f
1:
	strb w2, [x0], #1
	subs x1,x1, #1
	bne 1b
2:
	ret
ENTRY(memset)
	//.fnstart
	tst x2, x2
	beq 2f
1:
	strb w1, [x0], #1
	subs x2,x2, #1
	bne 1b
2:
	ret
	//.fnend

ENTRY(__aeabi_memclr)
	MOV x2, #0
	b __aeabi_memset

// A, B, num
ENTRY(memcmp)
	//.fnstart
	tst x2,x2
	beq 2f
1:
	ldrb w3, [x0], #1
	ldrb w4, [x1], #1
	cmp w4, w3
	bne 2f
	subs x2,x2, #1
	bne 1b
2:
	mov x0, #0
	beq 3f
	cinv x0,x0, lo
	cinc x0,x0, hs
3:
	ret


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
