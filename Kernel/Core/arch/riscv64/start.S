#
#
#
.option norvc	# 
#define KERNEL_BASE	0xFFFFFFFF00000000

#
# Initial boot
#
.section .text.init
.global start
start:
	csrr	t0, mhartid	# Read the [Hard]dware [T]hread ID
	bnez	t0, ap_wait	# if non-zero, go into the AP wait

	csrw	satp, zero	# Set Supervisor Address Translation and Protection register to zero, ensuring that the MMU is disable

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
	la	sp, init_stack
	# -- Set up paging (39 bit, ASID=0)
.extern boot_pt_lvl3_0_PHYS_PG
	la	a0, boot_pt_lvl3_0_PHYS_PG
	li	a1, 8
	sll	a1, a1, 60
	or	a0, a0, a1	# 8 = 39bit virtual address space (0=48bit)
	csrw	satp, a0

	# -- Prepare an exception frame from which to `mret`
	li	t0, (0b01 << 11) | (1 << 7) | (1 << 3)	# "Supervisor mode" ([11:12]=1), Interrupts on (MPIE, MIE:3=1)
	csrw	mstatus, t0
	li	t0, 3
	csrw	mie, t0
	la	t0, kmain	# Common entrypoint, store in `mret`'s target
	csrw	mepc, t0
	la	t0, trap_vector	# Machine-level error handler
	csrw	mtvec, t0
	la	ra, 1f	# Set a return address for `kmain`... just in case
	mret	# Interrupt return
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

.section .text

.globl trap_vector
trap_vector:
	wfi
	j trap_vector

#include "../../../../Usermode/rustrt0/riscv64-helpers.S"

.section .data
.global ap_wait_target
ap_wait_target:	.quad	ap_wait

.section .padata
# Three level PTs
# - Top-level covers 512GB, 1GB per slot
.global boot_pt_lvl3_0
.extern boot_pt_lvl2_hwmaps_PHYS_DIV4
boot_pt_lvl3_0:
	.rept 512-3
	.quad	0
	.endr
	.quad	boot_pt_lvl2_hwmaps_PHYS_DIV4 + (1 << 0)
	# NOTE: Fractal mapping doesn't work (needs permissions bits on the bottom layer)
	.quad	_phys_base_DIV4 + (7 << 1) + (1 << 0)	# Kernel at -2GB, 1GB map RWX
	.quad	0	# Final entry unset
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
	.quad	(0x10000000)/4+(3<<1)+(1<<0)	# qemu UART
	.rept 512-1
	.quad	0
	.endr
	

.section .bss
# TODO: Replace this with KSTACK_BASE
	.space 4096, 0
init_stack:
