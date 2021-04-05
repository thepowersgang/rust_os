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
	la	sp, init_stack
	# -- Set up paging (39 bit, ASID=0)
.extern boot_pt_lvl3_0_PHYS_PG
	la	a0, boot_pt_lvl3_0_PHYS_PG
	li	a1, 8
	sll	a1, a1, 60
	or	a0, a0, a1	# 8 = 39bit virtual address space (0=48bit)
	csrw	satp, a0

	# -- Prepare an exception frame from which to `mret`
#if SBI_BOOT
	li	t0, (1 << 8) | (1 << 5) | (1 << 1)	# SPP[8]=1 ([11:12]=1), Interrupts on (SPIE[5]=1, SIE[1]=1)
	csrw	sstatus, t0
#else
	li	t0, (0b01 << 11) | (1 << 7) | (1 << 3)	# "Supervisor mode" ([11:12]=1), Interrupts on (MPIE, MIE:3=1)
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

.globl trap_vector
trap_vector:
	wfi
	j trap_vector

#include "../../../../Usermode/rustrt0/riscv64-helpers.S"

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
.extern boot_pt_lvl2_hwmaps_PHYS_DIV4
boot_pt_lvl3_0:
	.rept 512-3
	.quad	0
	.endr
	.quad	boot_pt_lvl2_hwmaps_PHYS_DIV4 + (1 << 0)
	# NOTE: Fractal mapping doesn't work (needs permissions bits on the bottom layer)
	.quad	_phys_base_DIV4 + (3 << 6) + (7 << 1) + (1 << 0)	# Kernel at -2GB, 1GB map RWX
	.quad	boot_lt_lvl2_final_PHYS_DIV4 + (1 << 0)
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
	

.section .bss
# TODO: Replace this with KSTACK_BASE
	.space 4096, 0
init_stack:
