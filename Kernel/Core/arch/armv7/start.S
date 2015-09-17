@
@
@

.section VECTORS
ivt_reset:      b start	@ 0x00 Reset
ivt_undef:      b .	@ 0x04 #UD
ivt_svc:        b .	@ 0x08 SVC (used to be called SWI)
ivt_prefetch:   b .	@ 0x0C Prefetch abort
ivt_data:       b .	@ 0x10 Data abort
ivt_unused:     b .	@ 0x14 Not Used
ivt_irq:        b .	@ 0x18 IRQ
ivt_fiq:        b .	@ 0x1C FIQ (Fast interrupt)

.section .text

.globl start
.extern kmain
start:
	mov r4, pc
	
	ldr sp, =init_stack
	@ldr r0, =0x10009000
	@mov r1, #10 ; str r1, [r0]
	@mov r1, #'A'; str r1, [r0]
	@mov r1, #'R'; str r1, [r0]
	@mov r1, #'M'; str r1, [r0]
	@mov r1, #10 ; str r1, [r0]
	@ldr r0, =start
	@mov r1, #0
	@bl _ZN4arch4puth20h58831096f39b07d4bhpE
	@mov r0, r4
	@mov r1, #0
	@bl _ZN4arch4puth20h58831096f39b07d4bhpE
	
	ldr r0, =0x10009000
	mov r1, #10 ; str r1, [r0]
	
@	mov r1, #0
@	ldr r0, =__bss_start
@	ldr r2, =__bss_len
@1:
@	str r1, [r0], #4
@	sub r2, r2, #4
@	bne 1b
	
	bl kmain
	b .

.globl __aeabi_memcpy4
.globl __aeabi_memcpy8
__aeabi_memcpy8:
__aeabi_memcpy4:
1:
	LDR r3, [r1], #4
	STR r3, [r0], #4
	SUBS r2, r2, #4
	BGE 1b
	BX lr

.globl __aeabi_memset4
.globl __aeabi_memset8
__aeabi_memset4:
__aeabi_memset8:
	LSL r3, r1, #8
	ORR r1, r1, r3
	LSL r3, r1, #16
	ORR r1, r1, r3
1:
	STR r1, [r0], #0
	SUBS r2, r2, #4
	BGE 1b
	BX lr
.globl __aeabi_memclr4
.globl __aeabi_memclr8
__aeabi_memclr4:
__aeabi_memclr8:
	MOV r1, #0
	b 1b

.globl __aeabi_memset
__aeabi_memset:
	b .
.globl __aeabi_memclr
__aeabi_memclr:
	b .

.globl __aeabi_memcpy
__aeabi_memcpy:
	b .


.globl memcmp
memcmp:
	b .

.globl __aeabi_unwind_cpp_pr0
.globl __aeabi_unwind_cpp_pr1
__aeabi_unwind_cpp_pr0:
__aeabi_unwind_cpp_pr1:
	b .


#define ENTRY(v)	.globl v; v:

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

.section .bss
	.space 0x2000, 0
init_stack:
