ENTRY(start)

PHDRS {
	SegRX PT_LOAD;
	SegRW PT_LOAD;
}

SECTIONS {
	. = 0x200000;
	
	.text : {
		*(.text .text.*)
	} :SegRX
	.rodata ALIGN(0x1000): {
		*(.rodata .rodata.*)
	} :SegRX
	. = ALIGN(0x1000);
	.data ALIGN(0x1000): {
		*(.data .data.*)
	} :SegRW
	.bss ALIGN(0x1000) : {
		*(.bss .bss.*)
	} :SegRW
}

