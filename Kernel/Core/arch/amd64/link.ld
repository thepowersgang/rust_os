/*
 * "Tifflin" Kernel
 * - By John Hodge (thePowersGang)
 *
 * arch/amd64/link.ld
 * - AMD64/IA-32e linker script 
 */
_kernel_base = 0xFFFFFFFF80000000;

OUTPUT_FORMAT(elf64-x86-64)
ENTRY(start)

SECTIONS {
	. = 0x100000;
	gKernelBase = .;
	. += SIZEOF_HEADERS;
	__load_addr = .;
	
	/* Boot data */
	.init : AT(ADDR(.init)) {
		KEEP( *(.multiboot) )
		*(.inittext)
		. = ALIGN(8);
		*(.initdata)
	}
	
	low_InitialPML4 = InitialPML4 - _kernel_base;
	
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
	v_kernel_end = (. + 0xFFF)&0xFFFFFFFFFFFFF000;
	
	/DISCARD/ : { *(.tbss) }
}
