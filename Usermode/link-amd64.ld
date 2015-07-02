ENTRY(start)

SECTIONS {
	. = 0x200000;
	
	.text : {
		*(.text .text.*)
	}
	.rodata ALIGN(0x1000): {
		*(.rodata .rodata.*)
	}
	.data ALIGN(0x1000): {
		*(.data .data.*)
	}
	.bss ALIGN(0x1000) : {
		*(.bss .bss.*)
	}
}

