@
@
@
#define KERNEL_BASE	0x80000000

#define ENTRY(v)	.globl v; .type v,"function"; v:
#define GLOBAL(v)	.globl v; v:

#if 1 || defined(PLATFORM_qemuvirt)
# define UART_BASE	0x09000000
# define RAM_START	0x40000000
#elif defined(PLATFORM_realviewpb)
# define UART_BASE	0x10009000
# define RAM_START	0x00000000
#endif

.section VECTORS
ivt_reset:      b rst_start	@ 0x00 Reset
ivt_undef:      ldr pc, =ud_abort	@ 0x04 #UD
ivt_svc:        ldr pc, =svc_handler	@ 0x08 SVC (used to be called SWI)
ivt_prefetch:   ldr pc, =prefetch_abort	@ 0x0C Prefetch abort
ivt_data:       ldr pc, =data_abort	@ 0x10 Data abort
ivt_unused:     b .	@ 0x14 Not Used
ivt_irq:        b .	@ 0x18 IRQ
ivt_fiq:        b .	@ 0x1C FIQ (Fast interrupt)

rst_start:
	ldr pc, = start-KERNEL_BASE

//.section .inittext
.section .text

.extern hexdump
.extern kmain
.globl start
start:
	ldr r0, =0x1badb002
	teq r0, r13
	beq 1f
	@ TODO: What to do if we weren't loaded by our loader
	@ - For now, we return
	mov pc,lr
	b .
1:
	@ R9: UART Address
	@ R10: FDT base address
	@ R11: Symbol information base
	@ R12: End of used RAM
	@ R13: Magic
	
	// 0. Print a '\n' to the serial port
	mov r1, #'T' ; str r1, [r9]
	mov r1, #'i' ; str r1, [r9]
	mov r1, #'f' ; str r1, [r9]
	mov r1, #'f' ; str r1, [r9]
	mov r1, #'l' ; str r1, [r9]
	mov r1, #'i' ; str r1, [r9]
	mov r1, #'n' ; str r1, [r9]
	mov r1, #10 ; str r1, [r9]
	
	// To get RAM start: subtract linked address of current instruction from real address
	ldr r0, =(get_ram_base-0x80000000+4)
	sub r8, pc, r0
get_ram_base:
	ldr r0, =(kernel_phys_start - KERNEL_BASE)
	add r0, r8
	str r8, [r0]
	ldr r0, =(dt_phys_base - KERNEL_BASE)
	add r0, r8
	str r10, [r0]
	ldr r0, =(symbol_info_phys - KERNEL_BASE)
	add r0, r8
	str r11, [r0]
	ldr r0, =(ram_first_free - KERNEL_BASE)
	add r0, r8
	str r12, [r0]
	
	mov r12, r8
	
prep_page_tables:
	// 1. Prepare VMSA State
	ldr r0, =(kernel_table0-KERNEL_BASE)
	add r0, r12

	// - Prepare page tables (offset with RAM base)
	mov r4, r0
	ldr r5, =kernel_maps_len
1:
	ldr r3, [r4]
	cmp r3, #0	@ 0x1000
	beq 2f
	add r3, r12
2:
	str r3, [r4], #4
	subs r5, #4
	bne 1b
	
	orr r1, r12, #0x400
	orr r1, r1, #0x002
	lsr r2, r12, #20
	lsl r2, r2, #2
	ldr r3, [r0,r2]
	cmp r3, #0
	bne ram_mapping_collision
	str r1, [r0,r2]
	
vmsa_setup:
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
	
	// NOTE: VMSA is active here, so virtual addresses can be used

	@
	@ Check for security extensions
	@
	mrc p15, 0, r0, c0, c1, 1
	and r0, #0xF0
	beq 1f
	@ - Present
	ldr r0,=0xFFFF0000
	mcr p15, 0, r0, c12, c0, 0      @ Set the VBAR (brings exceptions into high memory)
	b 2f
1:
	@ - Absent
	mrc p15, 0, r0, c1, c0, 0       @ Set SCTLR.V
	orr r0, #0x2000
	mcr p15, 0, r0, c1, c0, 0
2:


	@ Populate the first HWMapping address with the UART's base
	add r0, r9, #0x13
	ldr r1, =hwmap_table_0+0
	str r0, [r1]

	cps #23	@ Switch to 'abort' mode
	ldr sp, =abort_stack
	cps #19	@ Back to supervisor
	
	ldr sp, =init_stack
	ldr pc, =kmain

@ If the start of RAM fell on an occupied section of the virtual address space
ram_mapping_collision:
	mov r0, #'R'; str r0, [r9]
	mov r0, #'A'; str r0, [r9]
	mov r0, #'M'; str r0, [r9]
	mov r0, #'!'; str r0, [r9]
	mov r0, #'\n'; str r0, [r9]
	b .

@
@ MAGIC MACRO
@
@ TODO: use https://sourceware.org/binutils/docs/as/ARM-Directives.html
.macro EXIDX method handle
.long EXIDX_\method
.section .ARM.exidx.\method, #exidx
.globl EXIDX_\method
EXIDX_\method: .long \method - . - 0x80000000, \handle
.section .text
.endm


.section .text
ENTRY(thread_trampoline)
	.fnstart
	.cantunwind
	pop {r1}	@ "thread_root" (generic over closure type)
	pop {r0}	@ Pop pointer to the closure
	bx r1
	.fnend
@ R0: Old stack save location
@ R1: New stack
@ R2: New TTBR0
@ R3: New Thread pointer
ENTRY(task_switch)
	.fnstart
	.cantunwind
	push {r4-r12,lr}

	@ Save SP
	str sp, [r0]

	@ Only update TTBR0 if the task has an explicit address space
	tst r2, r2
	mcrne p15,0, r2, c2,c0,0	@ Set TTBR0 to r2
	@mov r2, #1
	@mcrne p15,0, r2, c8,c7,0	@ TLBIALL - Invalid user space

	@ Set new thread pointer
	mcr p15, 0, r3, c13,c0,4	@ TPIDRPRW

	@ Set new SP
	mov sp, r1

	@ Restore state
	pop {r4-r12,pc}
	.fnend

@ pub fn drop_to_user(entry: usize, stack: usize, cmdline_len: usize) -> !;
@ R0: entry
@ R1: stack
@ R2: cmdline_len
ENTRY(drop_to_user)
	.fnstart
	.cantunwind
	cps #0x1F	@ Switch to "System" to set user SP
	mov sp, r1
	cps #0x13 	@ 0x13 = supervisor (kernel)
	mov r1, #0x10	@ -
	push {r0,r1}	@ Push user entrypoint and operating mode (r1)
	mov r0, r2	@ Set R0 = commandline length
	rfefd sp!
	.fnend


ENTRY(prefetch_abort)
	srsfd sp!, #0x17	@ Save state, using 'abort' mode stack
	push {r0-r12}	@ SP, LR, and PC not pushed
	
	bl get_abort_sp_lr
	push {r1,r2}	@ Source SP and LR
	
	ldr r0, [sp, #4*(2+13)]
	sub r0, #8
	mov r1, sp
	mrc p15,0, r2, c5,c0,1
	bl prefetch_abort_handler

	add sp, #8	@ Undo saving of SP/LR to stack
	pop {r0-r12}
	rfefd sp!
	.long EXIDX_prefetch_abort
EXIDX prefetch_abort, prefetch_abort_EXTAB - . - 0x80000000
.section .ARM.extab.prefetch_abort
GLOBAL(prefetch_abort_EXTAB)
	.long	0x81028600	@ POP {SP, LR}
	.long	0x81FFB10F	@ POP {r4-r12}, POP {r0-r3}
	.long	0x02B0B0B0	@ VSP+=12, END
.section .text

ENTRY(data_abort)
	.fnstart
	.cantunwind
	srsfd sp!, #0x17	@ Save state, using 'abort' mode stack
	push {r0-r12}	@ SP, LR, and PC not pushed
	
	mov r3, lr
	bl get_abort_sp_lr
	push {r1,r2}	@ Source SP and LR
	
	ldr r0, [sp, #4*(2+13)]
	sub r0, #8
	mov r1, sp
	mrc p15,0, r2, c6,c0,0
	mrc p15,0, r3, c5,c0,0
	bl data_abort_handler

	add sp, #8	@ Undo saving of SP/LR to stack
	pop {r0-r12}
	rfefd sp!
	.fnend

ENTRY(ud_abort)
	.fnstart
	.cantunwind
	srsfd sp!, #0x17	@ Save state, using 'abort' mode stack ([0]=LR_curr, [1]=SPSR)
	cps #0x17	@ UD Abort is mode 0x1b, but that's an extra stack...
	push {r0-r12}	@ SP, LR, and PC not pushed
	
	bl get_abort_sp_lr
	push {r1,r2}	@ Source SP and LR
	
	ldr r0, [sp, #4*(2+13)]
	sub r0, #4
	mov r1, sp
	bl ud_abort_handler
	
	add sp, #8	@ Undo saving of SP/LR to stack
	pop {r0-r12}
	rfefd sp!
	.fnend

get_abort_sp_lr:
	ldr r0, [sp, #4*(13+1)]	@ 13 GPRs, LR, [SPSR]
	and r0, #0x1F
	cmp   r0, #0x10	@ 0x10 = user
	cmpne r0, #0x13	@ 
	cmpne r0, #0x17	@ 
	bne 3f

	cmp r0, #0x10	@ 0x10 = user
	bne 1f
	cps #0x1F 	@ 0x1F = "System" (user regs, kernel privs)
	mov r1, sp
	mov r2, lr
	b 2f
1:
	cmp r0, #0x17	@ 0x17 = abort
	bne 1f
	sub r1, sp, #4*(13+2)
	ldr r2, [sp, #13*4]
	b 2f
1:
	cps #0x13 	@ 0x13 = supervisor (kernel)
	mov r1, sp
	mov r2, lr
	@b 2f
2:
	cps #23 	@ Switch back to abort mode
	bx lr
3:
	b .

GLOBAL(svc_handler)
	srsfd sp!, #19  @ Save state to stack
	push {r0-r5}
	
	mov r0, r12	@ R12 is the call ID
	mov r1, sp	@ Args have been pushed to kernel stack
	mov r2, #6	@ 6 of them
	bl syscalls_handler
	@ r0,r1 return value
	
	add sp, #6*4
	rfefd sp!

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
	.fnstart
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
	.fnend

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

ENTRY(__aeabi_memclr)
	MOV r2, #0
	b __aeabi_memset


ENTRY(__aeabi_uldivmod)
.extern __aeabi_uldivmod_
	.fnstart
	.save {lr}
	push {lr}
	.pad #20
	sub sp, #16
	push {sp}
	bl __aeabi_uldivmod_
	add sp, #4
	pop {r0, r1, r2, r3}
	pop {pc}
	.fnend

// A, B, num
ENTRY(memcmp)
	.fnstart
	.save {r4}
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
	.fnend

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

.section .rodata
data_abort_message:	.ascii "Data Abort: "
data_abort_message_end:
data_abort_message2:	.ascii "\n"
data_abort_message2_end:



.section .data
GLOBAL(dt_phys_base)	.long	0 	@ (Firmware) Device Tree base location
GLOBAL(kernel_phys_start).long	0	@ Start of kernel in RAM
GLOBAL(ram_first_free)	.long	0
GLOBAL(symbol_info_phys)	.long	0

.section .bss
init_stack_base:
	.space 0x20000, 0
init_stack:
	.space 0x1000, 0
abort_stack:


// Page Aligned data
.section .padata
.globl kernel_table0

kernel_table0:
	.long 0x00000402	@ Identity map the first 1 MiB
	.rept 0x800 - 1 - 8
		.long 0
	.endr
	.long user_last_map - KERNEL_BASE + 0x0000 + 1
	.long user_last_map - KERNEL_BASE + 0x0400 + 1
	.long user_last_map - KERNEL_BASE + 0x0800 + 1
	.long user_last_map - KERNEL_BASE + 0x0C00 + 1
	.long user_last_map - KERNEL_BASE + 0x1000 + 1
	.long user_last_map - KERNEL_BASE + 0x1400 + 1
	.long user_last_map - KERNEL_BASE + 0x1800 + 1
	.long user_last_map - KERNEL_BASE + 0x1C00 + 1
	@ 0x80000000 - User/Kernel split
	.long 0x00000000 + 0x402	@ Map first 8 MiB to 2GiB (KRW only)
	.long 0x00100000 + 0x402 	@ 
	.long 0x00200000 + 0x402	@ 
	.long 0x00300000 + 0x402	@ 
	.long 0x00400000 + 0x402	@ 
	.long 0x00500000 + 0x402 	@ 
	.long 0x00600000 + 0x402	@ 
	.long 0x00700000 + 0x402	@ 
	.rept 0xF00 - 0x800 - 8
		.long 0
	.endr
	@  - 0xF00_00000
	.rept 16
		.long 0
	.endr
	@  - 0xF10_00000
	.long hwmap_table_0 - KERNEL_BASE + 0x000 + 1
	.long hwmap_table_0 - KERNEL_BASE + 0x400 + 1
	.long hwmap_table_0 - KERNEL_BASE + 0x800 + 1
	.long hwmap_table_0 - KERNEL_BASE + 0xC00 + 1
	@  - 0xF14_00000
	.rept 0xFF8 - 0xF00 - 16 - 4
		.long 0
	.endr
	@ Page fractals and vectored exceptions
	.long 0, 0, 0, 0
	.long kernel_exception_map - KERNEL_BASE + 0x000 + 1
	.long kernel_exception_map - KERNEL_BASE + 0x400 + 1
	.long kernel_exception_map - KERNEL_BASE + 0x800 + 1
	.long kernel_exception_map - KERNEL_BASE + 0xC00 + 1

user_last_map:
	.rept 1024
		.long 0
	.endr
	.rept 1024-4
		.long 0
	.endr
	.long user_last_map - KERNEL_BASE + 0x0000 + 0x13
	.long user_last_map - KERNEL_BASE + 0x1000 + 0x13
	.long kernel_table0 - KERNEL_BASE + 0x0000 + 0x13
	.long kernel_table0 - KERNEL_BASE + 0x1000 + 0x13

.globl hwmap_table_0
hwmap_table_0:
	.long 0	@ Will be filled with UART base
	.rept 1023
		.long 0
	.endr
.globl kernel_exception_map
kernel_exception_map:
	@ First 1008 entries are empty (for use with kernel-side page tables)
	.rept 1024-16
		.long 0
	.endr
	.long 0x00000000 + 0x212	@ Exceptions at 0xFFFF0000, re-map first page
	.rept 16-1-1
		.long 0
	.endr
	.long 0	@ ALWAYS zero, to catch NULL-1 indexing
.globl kernel_maps_end
kernel_maps_end:

@ vim: ft=armasm
