ENTRY(start)

PHDRS {
	SegRX PT_LOAD;
	SegRW PT_LOAD;
	SegDYN PT_DYNAMIC;
}

SECTIONS {
	. = 0x200000;
	
	.text : {
		*(.text .text.*)
	} :SegRX
	.rodata ALIGN(0x1000): {
		*(.rodata .rodata.*)
	} :SegRX
	
	/* ARM unwind tables */
	/DISCARD/ : { *(.ARM.extab.init .ARM.exidx.init) }
	.ARM.extab : {
		*(.ARM.extab* .gnu.linkonce.armextab.*)
	} :SegRX
	PROVIDE_HIDDEN ( __exidx_start = . );
	.ARM.exidx : {
		*(.ARM.exidx* .gnu.linkonce.armexidx.*)
	} :SegRX
	.rel.plt : {
		*(.rel.plt)
	} :SegRX
	PROVIDE_HIDDEN ( __exidx_end = . );
	
	/* Dynamic linking info */
	.dynamic : {
		*(.dynamic)
	} :SegDYN
	
	. = ALIGN(0x1000);
	.data ALIGN(0x1000): {
		*(.data .data.*)
	} :SegRW
	.bss ALIGN(0x1000) : {
		*(.bss .bss.*)
	} :SegRW
}

