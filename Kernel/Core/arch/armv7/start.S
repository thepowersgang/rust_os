@
@
@
#define KERNEL_BASE	0x80000000
#define UART_BASE	0x10009000
//#define UART_BASE	0x09000000

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
	ldr r0, =UART_BASE
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
	

	@ Populate the first HWMapping address with the UART's base
	ldr r0, =UART_BASE + 0x13
	ldr r1, =hwmap_table_0+0
	str r0, [r1]

	ldr sp, =init_stack

.globl hexdump
	@mov r0, #0
	@mov r1, #1024
	@bl hexdump
	@ldr r0, =0xFFFFF000
	@mov r1, #1024
	@bl hexdump
	
	ldr pc, =kmain
	b .

.section .text
.globl thread_trampoline
thread_trampoline:
	b .
.globl task_switch
@ R0: Old stack save location
@ R1: New stack
@ R2: New TTBR0
@ R3: New Thread pointer
task_switch:
	push {r4-r12,lr}

	@ Save SP
	str sp, [r0]

	@ Only update TTBR0 if the task has an explicit address space
	tst r2, r2
	mcrne p15, 0, r1, c2, c0, 0	@ Set TTBR0 to r0
	@mov r1, #1
	@mcrne p15, 0, r1, c8, c7, 0	@ TLBIALL - Invalid user space

	@ Set new thread pointer
	mcr p15, 0, r4, c13,c0,4	@ TPIDRPRW

	@ Set new SP
	mov sp, r1

	@ Restore state
	pop {r4-r12,pc}

.globl drop_to_user
@ pub fn drop_to_user(entry: usize, stack: usize, cmdline_len: usize) -> !;
@ R0: entry
@ R1: stack
@ R2: cmdline_len
drop_to_user:
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
	@ Page fractals and vectored exceptions
	.long 0, 0, 0, 0
	.long kernel_exception_map + 0x000 - KERNEL_BASE + 1
	.long kernel_exception_map + 0x400 - KERNEL_BASE + 1
	.long kernel_exception_map + 0x800 - KERNEL_BASE + 1
	.long kernel_exception_map + 0xC00 - KERNEL_BASE + 1

.globl hwmap_table_0
hwmap_table_0:
	.long 0	@ 0x10009000 + 0x13
	.rept 1023
		.long 0
	.endr
.globl kernel_exception_map
kernel_exception_map:
	@ First 1008 entries are empty (for use with kernel-side page tables)
	.rept 1024-16
		.long 0
	.endr
	.long 0x00000000 + 0x212	@ Exceptions at 0xFFFF0000, re-map first page (TODO: Mangle this with load base instead)
	.rept 8-1
		.long 0
	.endr
	.long kernel_table0 + 0x0000 - KERNEL_BASE + 0x13 @ Base fractal (0xFFFF8000)
	.long kernel_table0 + 0x1000 - KERNEL_BASE + 0x13
	.long kernel_table0 + 0x2000 - KERNEL_BASE + 0x13
	.long kernel_table0 + 0x3000 - KERNEL_BASE + 0x13
	.long kernel_exception_map - KERNEL_BASE + 0x13	@ Self fractal (0xFFFF9000)
	.long 0
	.long 0
	.long 0
