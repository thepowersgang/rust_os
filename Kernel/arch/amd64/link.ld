
_kernel_base = 0xFFFFFFFF80000000;

OUTPUT_FORMAT(elf64-x86-64)
ENTRY(start)

SECTIONS {
	. = 0x100000;
	gKernelBase = .;
	. += SIZEOF_HEADERS;
	__load_addr = .;
	.multiboot : AT(ADDR(.multiboot)) {
		*(.multiboot)
	}
	.init : AT(ADDR(.init)) {
		*(.inittext)
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
		*(.MODULE_LIST)
		modules_end = .;
	}
	
	.ktls ALIGN (0x1000) : AT(ADDR(.ktls) - _kernel_base) {
		*(.tbss)
		*(.tbss.*)
		tbss_top = .;
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
}
