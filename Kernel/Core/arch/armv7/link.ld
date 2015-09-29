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
	. = 0x00001000;
	gKernelBase = .;
	/* . += SIZEOF_HEADERS; */
	__load_addr = .;
	
	. += _kernel_base;
	
	.text ALIGN(0x1000): AT(ADDR(.text) - _kernel_base) {
		KEEP( *(VECTORS) )
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

	/DISCARD/ : { *(.ARM.extab.init .ARM.exidx.init) }
	.ARM.extab : AT( ADDR(.ARM.extab) - _kernel_base) {
		*(.ARM.extab* .gnu.linkonce.armextab.*)
	}
	PROVIDE_HIDDEN ( __exidx_start = . );
	.ARM.exidx : AT( ADDR(.ARM.exidx) - _kernel_base) {
		*(.ARM.exidx* .gnu.linkonce.armexidx.*)
	}
	PROVIDE_HIDDEN ( __exidx_end = . );

	data_phys_base = . - _kernel_base;
	
	/* .data (and all other mutable data) is aligned to 1MB */
	.data ALIGN (0x100000) : AT(data_phys_base) {
		*(.padata)
		*(.data)
		*(.data.*)
	}
	kernel_table0_ofs = kernel_table0 - ADDR(.data);
	hwmap_table_0_ofs = hwmap_table_0 - ADDR(.data);
	kernel_exception_map_ofs = kernel_exception_map - ADDR(.data);
	kernel_maps_len = kernel_maps_end - kernel_table0;
	data_len = . - ADDR(.data);

	__bss_start = .;
	.bss : AT(ADDR(.bss) - ADDR(.data) + data_phys_base) {
		*(COMMON)
		*(.bss)
		*(.bss.*)
	}
	. = ALIGN(4);
	__bss_end = .;
	__bss_len = __bss_end - __bss_start;
	v_kernel_end = (. + 0xFFF)&0xFFFFF000;
	
	/DISCARD/ : { *(.tbss) }
}

