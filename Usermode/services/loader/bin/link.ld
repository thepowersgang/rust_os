ENTRY(loader_start)

SECTIONS {
	. = BASE;
	.text : {
		*(HEADER);
		*(.text .text.*)
	}
	.rodata ALIGN(PAGE_SIZE): {
		rodata_start = .;
		*(.rodata .rodata.*)
	}
	/* ARM unwind tables (not present) */
	/DISCARD/ : { *(.ARM.extab.init .ARM.exidx.init) }
	.eh_frame : {
		*(.eh_frame)
	}
	.eh_frame_hdr : {
		*(.eh_frame_hdr)
	}
	.ARM.extab : {
		*(.ARM.extab* .gnu.linkonce.armextab.*)
	}
	PROVIDE_HIDDEN ( __exidx_start = . );
	.ARM.exidx : {
		*(.ARM.exidx* .gnu.linkonce.armexidx.*)
	}
	PROVIDE_HIDDEN ( __exidx_end = . );
	/*
	PROVIDE_HIDDEN ( __exidx_start = . );
	PROVIDE_HIDDEN ( __exidx_end = . );
	/* */

	. = ALIGN(PAGE_SIZE);
	codesize = . - BASE;
	.data ALIGN(PAGE_SIZE): {
		data_start = .;
		*(.sdata .sdata.*)
		*(.data .data.*)
		*(.got .got.*)
		. = ALIGN(PAGE_SIZE);
	} = 0x0000
	. = ALIGN(PAGE_SIZE);
	.bss ALIGN(PAGE_SIZE) : {
		bss_start = .;
		*(.bss .bss.*)
		*(.sbss .sbss.*)
		bss_end = .;
	}
	imagesize = . - BASE;
	init_path_ofs = init_path - BASE;

	ASSERT(. < LIMIT, "Image is too large to fit in reservation")
}
