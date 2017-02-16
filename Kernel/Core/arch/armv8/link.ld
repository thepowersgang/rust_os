/*
 * "Tifflin" Kernel
 * - By John Hodge (thePowersGang)
 *
 * arch/armv8/link.ld
 * - ARMv8 linker script 
 */
/* - -64GB */
_kernel_base = 0xFFFF800000000000;
_PAGE_SIZE = 0x10000;	/* 64KB */

ENTRY(start)

SECTIONS {
	. = 0x00000000;
	gKernelBase = .;
	/* . += SIZEOF_HEADERS; */
	__load_addr = .;
	
	. += _kernel_base;
	
	.text ALIGN(_PAGE_SIZE): AT(ADDR(.text) - _kernel_base) {
		KEEP( *(VECTORS) )
		*(.text)
		*(.text.*)
	}
	
	.usertext ALIGN(_PAGE_SIZE): AT(ADDR(.usertext) - _kernel_base) {
		_UsertextBase = .;
		*(.usertext)
		_UsertextEnd = .;
	}
	
	.rodata ALIGN(_PAGE_SIZE): AT(ADDR(.rodata) - _kernel_base) {
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

	/* .data (and all other mutable data) is aligned to 1MB */
	.data ALIGN (0x100000) : AT(ADDR(.data) - _kernel_base) {
		*(.padata)
		*(.data)
		*(.data.*)
	}
	kernel_maps_len = kernel_maps_end - kernel_maps_start;

	__bss_start = .;
	.bss : AT(ADDR(.bss) - _kernel_base) {
		*(COMMON)
		*(.bss)
		*(.bss.*)
		. = ALIGN(_PAGE_SIZE);
		*(.pabss)
	}
	. = ALIGN(4);
	. = ALIGN(_PAGE_SIZE);
	v_kernel_end = .;
	
	/DISCARD/ : { *(.tbss) }
}

