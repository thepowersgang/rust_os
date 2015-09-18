@
@
@
#define KERNEL_BASE	0x80000000

.section VECTORS
ivt_reset:      b rst_start	@ 0x00 Reset
ivt_undef:      b .	@ 0x04 #UD
ivt_svc:        b .	@ 0x08 SVC (used to be called SWI)
ivt_prefetch:   b .	@ 0x0C Prefetch abort
ivt_data:       b .	@ 0x10 Data abort
ivt_unused:     b .	@ 0x14 Not Used
ivt_irq:        b .	@ 0x18 IRQ
ivt_fiq:        b .	@ 0x1C FIQ (Fast interrupt)

rst_start:
	b .

//.section .inittext
.section .text

.globl start
.extern kmain
start:
	// 0. Print a '\n' to the serial port
	ldr r0, =0x10009000
	mov r1, #10 ; str r1, [r0]
	
	// 1. Prepare VMSA State
	ldr r0, =kernel_table0-KERNEL_BASE
	mcr p15, 0, r0, c2, c0, 1	@ Set TTBR1 to r0
	mcr p15, 0, r0, c2, c0, 0	@ Set TTBR0 to r0 too (for identity)
	mov r0, #1
	mcr p15, 0, r0, c2, c0, 2	@ Set TTCR to 1 (50/50 split)
	mov r0, #3
	mcr p15, 0, r0, c3, c0, 0	@ Set Domain 0 to Manager
	@ Enable VMSA
	mrc p15, 0, r0, c1, c0, 0
	orr r0, r0, #1
	orr r0, r0, #1 << 23
	mcr p15, 0, r0, c1, c0, 0
	
	mov r0, #1
	mcr p15, 0, r0, c13, c0, 1	@ HACK: Set ASID to non zero
	mov r0, #0x55	@ 01010101b
	mcr p15, 0, r0, c3, c0, 0	@ Enable access faults on domains 0 & 1
	
	ldr sp, =init_stack
	
	// 2. Clear .bss
@	mov r1, #0
@	ldr r0, =__bss_start
@	ldr r2, =__bss_len
@1:
@	str r1, [r0], #4
@	sub r2, r2, #4
@	bne 1b
	
	ldr pc, =kmain
	b .

.section .text
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


// Page Aligned data
.section .padata
.globl kernel_table0

kernel_table0:
	.long 0x00000402	@ Identity map the first 1 MiB
	.rept 0x800 - 1
		.long 0
	.endr
	@ 0x80000000 - User/Kernel split
	.long 0x00000000 + 0x402	@ Map first 4 MiB to 2GiB (KRW only)
	.long 0x00100000 + 0x402 	@ 
	.long 0x00200000 + 0x402	@ 
	.long 0x00300000 + 0x402	@ 
	.rept 0xF00 - 0x800 - 4
		.long 0
	.endr
	.rept 16
		.long 0
	.endr
	.long hwmap_table_0 + 0x000 - KERNEL_BASE + 1
	.long hwmap_table_0 + 0x400 - KERNEL_BASE + 1
	.long hwmap_table_0 + 0x800 - KERNEL_BASE + 1
	.long hwmap_table_0 + 0xC00 - KERNEL_BASE + 1
	.rept 0xFF8 - 0xF00 - 16 - 4
		.long 0
	.endr
	@ Page fractals
	.long 0, 0, 0, 0
	.long kernel_exception_map + 0x000 - KERNEL_BASE + 1
	.long kernel_exception_map + 0x400 - KERNEL_BASE + 1
	.long kernel_exception_map + 0x800 - KERNEL_BASE + 1
	.long kernel_exception_map + 0xC00 - KERNEL_BASE + 1

.globl hwmap_table_0
hwmap_table_0:
	.long 0x10009000 + 0x13
	.rept 1023
		.long 0
	.endr
.globl kernel_exception_map
kernel_exception_map:
	@ First 1008 entries are empty
	.rept 1024-16
		.long 0
	.endr
	.long 0x00000000 + 0x212	@ Exceptions at 0xFFFF0000, re-map first page (TODO: Mangle this with load base instead)
	.rept 16-1-2
		.long 0
	.endr
	.long 0
	.long 0
