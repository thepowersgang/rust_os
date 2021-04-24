// "Tifflin" Kernel
// - By John Hodge (Mutabah)
//
// Core/arch/riscv64/start.asm
// - RISC-V 64-bit boot shim
.option norvc	# 
#define KERNEL_BASE	0xFFFFFFFF00000000
#define SBI_BOOT	1

#
# Initial boot
#
.section .text.init
.global start
start:
#if SBI_BOOT
	# SBI boot from qemu:
	# a0 = HART ID
	# a1 = FDT base
	mv	t5, a0
	mv	t6, a1
	la	a0, fdt_phys_addr_PHYS
	sd	t6, (a0)

#else
	csrr	t0, mhartid	// Read the [Hard]dware [T]hread ID
	bnez	t0, ap_wait	// if non-zero, go into the AP wait
#endif

	# -- Clear the BSS
.extern __bss_start
.extern __bss_end
	la 	a0, __bss_start_LOW
	la	a1, __bss_end_LOW
	bgeu	a0, a1, 2f
1:
	sd	zero, (a0)
	addi	a0, a0, 8
	bltu	a0, a1, 1b
2:
	# -- Prepare the stack
	li	sp, (0xFFFFFFE000000000 + 4*0x1000)
	# -- Set up paging (39 bit, ASID=0)
.extern boot_pt_lvl3_0_PHYS_PG
	la	a0, boot_pt_lvl3_0_PHYS_PG
	li	a1, 8
	sll	a1, a1, 60
	or	a0, a0, a1	# 8 = 39bit virtual address space (0=48bit)
	csrw	satp, a0

	# -- Prepare an exception frame from which to `mret`
#if SBI_BOOT
	# SPP = Supervisor Previous Permission (Set to 1 to stay in supervisor mode)
	# SPIE = Supervisor Previous Interurpt Enable (enable interrupts on mret/sret)
	li	t0, (1 << 18) | (1 << 8) | (1 << 5)	# SUM=0, SPP[8]=1, Interrupts on (SPIE[5]=1)
	csrw	sstatus, t0
#else
	li	t0, (1 << 18) + (0b01 << 11) + (1 << 7) + (1 << 3)	# "Supervisor mode" ([11:12]=1), Interrupts on (MPIE, MIE:3=1)
	csrw	mstatus, t0
#endif
	li	t0, 3
	csrw	sie, t0
	la	t0, kmain	# Common entrypoint, store in `mret`'s target
#if SBI_BOOT
	csrw	sepc, t0
#else
	csrw	mepc, t0
#endif
	la	t0, trap_vector	# Supervisor-level error handler
	csrw	stvec, t0
	la	ra, 1f	# Set a return address for `kmain`... just in case
#if SBI_BOOT
	sret	// Supervisor interrupt return
#else
	mret	// Machine interrupt return
#endif
1:
	wfi
	j 1b

ap_wait:
	wfi
	# Load an address, and jump to that (pre-initialised to just jump back to `ap_wait`)
	ld a1, ap_wait_target_ptr
	ld a0, (a1)
	jr a0

.section .initdata
# > Reference the from `.data` and `.padata`
ap_wait_target_ptr: .quad ap_wait_target - KERNEL_BASE
boot_pt_lvl3_0_PHYS_ptr:	.quad boot_pt_lvl3_0 - KERNEL_BASE


//
//
//
.section .text

.globl _Unwind_Resume
_Unwind_Resume:
	j .

.globl trap_vector
.extern trap_vector_rs
trap_vector:
	#wfi
	#j trap_vector
	j trap_vector_rs

#include "../../../../Usermode/rustrt0/riscv64-helpers.S"

#
# See arch::threads::start_thread
#
.global thread_trampoline
thread_trampoline:
	ld t0, 0(sp)	# pop the body shim
	ld a0, 8(sp)	# Pop the data pointer
	add sp, sp, 16
	jalr t0
	j .
# ARGS:
# - a0: old sp destination
# - a1: new sp
# - a2: new satp
# - a3: new thread pointer
.global task_switch
task_switch:
	add sp, sp, -15*8
	sd ra, 14*8(sp)
	sd gp, 13*8(sp)
	sd tp, 12*8(sp)
	sd s0, 0*8(sp)
	sd s1, 1*8(sp)
	sd s2, 2*8(sp)
	sd s3, 3*8(sp)
	sd s4, 4*8(sp)
	sd s5, 5*8(sp)
	sd s6, 6*8(sp)
	sd s7, 7*8(sp)
	sd s8, 8*8(sp)
	sd s9, 9*8(sp)
	sd s10, 10*8(sp)
	sd s11, 11*8(sp)
	# Disable interrupts (clear SIE)
	csrci sstatus, 0x2
	# Save SP, get new SP, switch SATP
	sd sp, (a0)
	mv sp, a1
	csrw satp, a2
	csrw sscratch, a3
	# Re-enable interrupts (set SIE)
	csrsi sstatus, 0x2
	# Pop state
	ld ra, 14*8(sp)
	ld gp, 13*8(sp)
	ld tp, 12*8(sp)
	ld s0, 0*8(sp)
	ld s1, 1*8(sp)
	ld s2, 2*8(sp)
	ld s3, 3*8(sp)
	ld s4, 4*8(sp)
	ld s5, 5*8(sp)
	ld s6, 6*8(sp)
	ld s7, 7*8(sp)
	ld s8, 8*8(sp)
	ld s9, 9*8(sp)
	ld s10, 10*8(sp)
	ld s11, 11*8(sp)
	add sp, sp, 15*8
	# Return
	ret

.section .data
.global ap_wait_target
ap_wait_target:	.quad	ap_wait
.global fdt_phys_addr
fdt_phys_addr:	.quad 0

.section .padata
# Three level PTs
# - Top-level covers 512GB, 1GB per slot
# Useful bits:
# - 0: Valid (must be set)
# - 1-3: RWX
# - 4: User (Note: Supervisor can't read these without a flag set)
# - 5: Global
# - 6: Accessed
# - 7: Written
.global boot_pt_lvl3_0
boot_pt_lvl3_0:
.extern boot_pt_lvl2_stacks_PHYS_DIV4
.extern boot_pt_lvl2_hwmaps_PHYS_DIV4
.extern boot_pt_lvl2_final_PHYS_DIV4
	.quad 0
	.quad 0
	.quad	_phys_base_DIV4 + (3 << 6) + (7 << 1) + (1 << 0)	# Kernel at 2GB, 1GB map RWX (identity)
	.rept 256-3
	 .quad 0
	.endr
	# 0x00000040_00000000
	# 0xFFFFFFC0_00000000
	.rept 128
	 .quad 0
	.endr
	# 0xFFFFFFE0_00000000: Stacks base
	.quad boot_pt_lvl2_stacks_PHYS_DIV4 + (1 << 0)
	.rept 128-(1+3)
	 .quad	0
	.endr
	# 0xFFFFFFFF_40000000
	.quad	boot_pt_lvl2_hwmaps_PHYS_DIV4 + (1 << 0)
	# NOTE: Fractal mapping doesn't work (needs permissions bits on the bottom layer)
	.quad	_phys_base_DIV4 + (3 << 6) + (7 << 1) + (1 << 0)	# Kernel at -2GB, 1GB map RWX
	.quad	boot_lt_lvl2_final_PHYS_DIV4 + (1 << 0)
# --- Kernel Stacks (prepared with a 3 page boot stack) ---
# 1GB coverage, 2MiB per slot
.global boot_pt_lvl2_stacks
boot_pt_lvl2_stacks:
.extern boot_pt_lvl1_stacks_PHYS_DIV4
	.quad	boot_pt_lvl1_stacks_PHYS_DIV4 + (1 << 0)
	.rept 512-1
	 .quad	0
	.endr
.global boot_pt_lvl1_stacks
boot_pt_lvl1_stacks:
.extern init_stack_PHYS_DIV4
	.quad	0
	.quad init_stack_PHYS_DIV4 + (0*0x1000/4) + (3 << 1) + (1 << 0)
	.quad init_stack_PHYS_DIV4 + (1*0x1000/4) + (3 << 1) + (1 << 0)
	.quad init_stack_PHYS_DIV4 + (2*0x1000/4) + (3 << 1) + (1 << 0)
	.rept 512-4
	.quad	0
	.endr
# --- Hardware mappings (prepared with UART) ---
# 1GB coverage, 2MiB per slot
.global boot_pt_lvl2_hwmaps
.extern boot_pt_lvl1_hwmaps_PHYS_DIV4
boot_pt_lvl2_hwmaps:
	.quad	boot_pt_lvl1_hwmaps_PHYS_DIV4 + (1 << 0)
	.rept 512-1
	.quad	0
	.endr
# 2MiB coverage, 4KiB per slot
.global boot_pt_lvl1_hwmaps
boot_pt_lvl1_hwmaps:
	.quad	(0x10000000)/4 + (3 << 6)+(3<<1)+(1<<0)	# qemu UART
	.rept 512-1
	.quad	0
	.endr
# --- Final table (includes temporary mappings, must always be present) ---
.global boot_lt_lvl2_final
boot_lt_lvl2_final:
	.rept 512-2
	.quad	0
	.endr
	.quad	boot_pt_lvl1_temp_PHYS_DIV4 + (1 << 0)
	.quad	0	# Last 2MiB is unmapped
# @0xFFFFFFFF_FFC00000 : Temporary mappings
.global boot_pt_lvl1_temp
boot_pt_lvl1_temp:
	.rept 512
	.quad	0
	.endr

.global init_stack
init_stack:
	.space 4096*3, 0

