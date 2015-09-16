/*
 * "Tifflin" Kernel
 * - By John Hodge (thePowersGang)
 *
 * arch/armv7/link.ld
 * - ARMv7 linker script 
 */
_kernel_base = 0x80000000;

/* OUTPUT_FORMAT(elf32) */
ENTRY(start)

SECTIONS {
	. = 0x000000;
	gKernelBase = .;
	. += SIZEOF_HEADERS;
	__load_addr = .;
	
	/* Boot data */
	.init : AT(ADDR(.init)) {
		*(.inittext)
		*(.initdata)
	}
	
	. += _kernel_base;
	
	.text ALIGN(0x1000): AT(ADDR(.text) - _kernel_base) {
		*(.text)
		*(.text.*)
	}
	
	.usertext ALIGN(0x1000): AT(ADDR(.usertext) - _kernel_base) {
		_UsertextBase = .;
		*(.usertext)
		_UsertextEnd = .;
	}
	
	.rodata ALIGN(0x1000): AT(ADDR(.rodata) - _kernel_base) {
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
	
	.data ALIGN (0x1000) : AT(ADDR(.data) - _kernel_base) {
		*(.padata)
		*(.data)
		*(.data.*)
	}

	__bss_start = .;
	.bss : AT(ADDR(.bss) - _kernel_base) {
		*(COMMON)
		*(.bss)
		*(.bss.*)
	}
	v_kernel_end = (. + 0xFFF)&0xFFFFF000;
	
	/DISCARD/ : { *(.tbss) }
}

