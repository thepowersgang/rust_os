ENTRY(start)
OUTPUT_FORMAT(binary)

SECTIONS {
	. = BASE;
	.text : {
		*(HEADER);
		*(.text .text.*)
	}
	.rodata ALIGN(0x1000): {
		rodata_start = .;
		*(.rodata .rodata.*)
	}
	codesize = . - BASE;
	.data ALIGN(0x1000): {
		data_start = .;
		*(.data .data.*)
	}
	.bss : {
		bss_start = .;
		*(.bss .bss.*)
		bss_end = .;
	}
	imagesize = . - BASE;
}
