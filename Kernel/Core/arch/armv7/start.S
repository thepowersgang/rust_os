@
@
@
#define KERNEL_BASE	0x80000000

#if 1 || defined(PLATFORM_qemuvirt)
# define UART_BASE	0x09000000
# define RAM_START	0x40000000
#elif defined(PLATFORM_realviewpb)
# define UART_BASE	0x10009000
# define RAM_START	0x00000000
#endif

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
	ldr pc, = start-KERNEL_BASE

//.section .inittext
.section .text

.extern hexdump
.extern kmain
.globl start
start:
	// 0. Print a '\n' to the serial port
	ldr r0, =UART_BASE
	mov r1, #'T' ; str r1, [r0]
	mov r1, #'i' ; str r1, [r0]
	mov r1, #'f' ; str r1, [r0]
	mov r1, #'f' ; str r1, [r0]
	mov r1, #'l' ; str r1, [r0]
	mov r1, #'i' ; str r1, [r0]
	mov r1, #'n' ; str r1, [r0]
	mov r1, #10 ; str r1, [r0]
	
	ldr r12, =RAM_START
	ldr r1, [r12]	// FDT magic value
	//ldr r2, =0xd00dfeed
	ldr r2, =0xedfe0dd0
	cmp r1, r2
	bne 1f
	
	mov r11, r12	// Store base in R11, which will be stored to RAM once VMM is up
	ldr r1, [r12, #4]	// Total blob size, i.e. start of usable RAM
	ldr r2, =0x000FFFFF
	add r1, r2
	mvn r2, r2
	and r1, r2	// Rounded up to 1MB
	add r12, r1
1:
	// R0: Start of usable RAM, we assume there's enough free to copy the .data section
	// - Build/copy paging structures in RAM
	mov r0, r12
	ldr r1, =data_phys_base
	ldr r2, =data_len
	bl __aeabi_memcpy4
	mov r1, #0
	ldr r2, =__bss_len
	bl __aeabi_memset4
	
prep_page_tables:
	// 1. Prepare VMSA State
	ldr r0, =kernel_table0_ofs
	add r0, r12

	// - Prepare page tables (offset with RAM base)
	mov r4, r0
	ldr r5, =kernel_maps_len
1:
	ldr r3, [r4]
	cmp r3, #0x1000
	blo 2f
	add r3, r12
2:
	str r3, [r4], #4
	subs r5, #4
	bne 1b
	
	// - Set RAM base in table
	orr r2, r12, #0x400
	orr r2, r2, #0x2
	ldr r3, =0x2004
	str r2, [r0, r3]
	
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

	@ Populate the first HWMapping address with the UART's base
	ldr r0, =UART_BASE + 0x13
	ldr r1, =hwmap_table_0+0
	str r0, [r1]

	ldr r0, =dt_base
	str r11, [r0]	@ R11 was set back at the start to be the DT base
	str r12, [r0, #4]
	
	ldr sp, =init_stack
	ldr pc, =kmain

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
	movs r2, r2
	beq 2f
1:
	strb r1, [r0], #1
	subs r2, #1
	bne 1b
2:
	bx lr
.globl __aeabi_memclr
__aeabi_memclr:
	MOV r1, #0
	b __aeabi_memset

.globl __aeabi_memcpy
__aeabi_memcpy:
	b .


.globl memcmp
// A, B, num
memcmp:
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


.section .data
.globl dt_base
dt_base:
	.long	0 	@ (Firmware) Device Tree base location
.globl kernel_data_start
kernel_data_start:
	.long	0	@ Start of kernel's .data segment

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
	.long 0 @ x00100000 + 0x402 	@ 
	.long 0 @ x00200000 + 0x402	@ 
	.long 0 @ x00300000 + 0x402	@ 
	.rept 0xF00 - 0x800 - 4
		.long 0
	.endr
	.rept 16
		.long 0
	.endr
	.long hwmap_table_0_ofs + 0x000 + 1
	.long hwmap_table_0_ofs + 0x400 + 1
	.long hwmap_table_0_ofs + 0x800 + 1
	.long hwmap_table_0_ofs + 0xC00 + 1
	.rept 0xFF8 - 0xF00 - 16 - 4
		.long 0
	.endr
	@ Page fractals and vectored exceptions
	.long 0, 0, 0, 0
	.long kernel_exception_map_ofs + 0x000 + 1
	.long kernel_exception_map_ofs + 0x400 + 1
	.long kernel_exception_map_ofs + 0x800 + 1
	.long kernel_exception_map_ofs + 0xC00 + 1

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
	.long kernel_table0_ofs + 0x0000 + 0x13 @ Base fractal (0xFFFF8000)
	.long kernel_table0_ofs + 0x1000 + 0x13
	.long kernel_table0_ofs + 0x2000 + 0x13
	.long kernel_table0_ofs + 0x3000 + 0x13
	.long kernel_exception_map_ofs + 0x13	@ Self fractal (0xFFFFC000)
	.long 0
	.long 0
	.long 0
.globl kernel_maps_end
kernel_maps_end:
