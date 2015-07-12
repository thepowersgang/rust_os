ENTRY(start)

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
		*(.got .got.*)
		. = ALIGN(0x1000);
	} = 0x0000
	.bss ALIGN(0x1000) : {
		bss_start = .;
		*(.bss .bss.*)
		bss_end = .;
	}
	imagesize = . - BASE;

	ASSERT(. < LIMIT, "Image is too large to fit in reservation")
}
