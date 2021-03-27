/*
 * "Tifflin" Kernel
 * - By John Hodge (thePowersGang)
 *
 * arch/riscv64/link.ld
 * - RISC-V 64-bit (General, Compressed) linker script 
 */
_kernel_base = 0xFFFFFFFF80000000;
_phys_base = 0x80000000;

OUTPUT_FORMAT(elf64-littleriscv)
ENTRY(start)

/*MEMORY
{
  ram   (wxa!ri) : ORIGIN = 0x80000000, LENGTH = 128M
}*/

SECTIONS {
	. = _phys_base;
	gKernelBase = .;
	/* . += SIZEOF_HEADERS; */
	__load_addr = .;
	
	/* Boot data */
	.init : AT(ADDR(.init)) {
		*(.text.init)
		. = ALIGN(8);
		*(.initdata)
	}
	
	. += _kernel_base - _phys_base;
	
	.text ALIGN(0x1000): AT(ADDR(.text) - _kernel_base + _phys_base) {
		*(.text)
		*(.text.*)
	}
	
	.usertext ALIGN(0x1000): AT(ADDR(.usertext) - _kernel_base + _phys_base) {
		_UsertextBase = .;
		*(.usertext)
		_UsertextEnd = .;
	}
	
	.rodata ALIGN(0x1000): AT(ADDR(.rodata) - _kernel_base + _phys_base) {
		*(.initpd)
		*(.rodata)
		*(.rodata.*)
		*(.rdata)
		*(.rdata.*)
		
		. = ALIGN(0x10);
		modules_base = .;
		KEEP( *(.MODULE_LIST) )
		modules_end = .;
	}
	
	.data ALIGN (0x1000) : AT(ADDR(.data) - _kernel_base + _phys_base) {
		*(.padata)
		*(.data)
		*(.data.*)
	}

	__bss_start_LOW = . - _kernel_base + _phys_base;
	.bss : AT(ADDR(.bss) - _kernel_base + _phys_base) {
		*(COMMON)
		*(.bss)
		*(.bss.*)
	}
	__bss_end_LOW = . - _kernel_base + _phys_base;
	v_kernel_end = (. + 0xFFF)&0xFFFFFFFFFFFFF000;
	
	/DISCARD/ : { *(.tbss) }

	ap_wait_target_LOW = ap_wait_target - _kernel_base + _phys_base;
	boot_pt_lvl3_0_PHYS = boot_pt_lvl3_0 - _kernel_base + _phys_base;
	boot_pt_lvl3_0_PHYS_PG = boot_pt_lvl3_0_PHYS >> 12;

	/* For the boot page table */
	_phys_base_DIV4 = _phys_base >> 2;
	boot_pt_lvl2_hwmaps_PHYS_DIV4 = (boot_pt_lvl2_hwmaps - _kernel_base + _phys_base) >> 2;
	boot_pt_lvl1_hwmaps_PHYS_DIV4 = (boot_pt_lvl1_hwmaps - _kernel_base + _phys_base) >> 2;
}

