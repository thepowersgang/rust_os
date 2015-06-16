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
	. = ALIGN(0x1000);
	codesize = . - BASE;
	.data ALIGN(0x1000): {
		data_start = .;
		*(.data .data.*)
		. = ALIGN(0x1000);
	}
	.bss : {
		bss_start = .;
		*(.bss .bss.*)
		bss_end = .;
	}
	imagesize = . - BASE;
}
