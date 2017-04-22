ENTRY(start)

STARTUP( .objs/armv7/rustrt0.o )

PHDRS {
	SegRX PT_LOAD;
	SegR PT_LOAD FLAGS(4);	/* 4 = PT_R */
	SegRW PT_LOAD;
	SegDYN PT_DYNAMIC;
}

SECTIONS {
	. = 0x200000;
	/*. += SIZEOF_HEADERS;*/

	.text : {
		*(.text .text.*)
	} :SegRX
	.plt : {
		*(.plt)
	} :SegRX

	.rodata ALIGN(0x2000): {
		*(.rodata .rodata.*)
	} :SegR
	
	/* ARM unwind tables */
	/DISCARD/ : { *(.ARM.extab.init .ARM.exidx.init) }
	.ARM.extab : {
		*(.ARM.extab* .gnu.linkonce.armextab.*)
	} :SegR
	PROVIDE_HIDDEN ( __exidx_start = . );
	.ARM.exidx : {
		*(.ARM.exidx* .gnu.linkonce.armexidx.*)
	} :SegR
	PROVIDE_HIDDEN ( __exidx_end = . );
	
	/* Dynamic linking info */
	.dynamic : { *(.dynamic) } :SegDYN :SegR
	.dynsym : { *(.dynsym) } :SegDYN :SegR
	.dynstr : { *(.dynstr) } :SegDYN :SegR
	.rela.got : { *(.rela.got) } :SegDYN :SegR
	.rela.dyn : { *(.rela.dyn) } :SegDYN :SegR
	.rel.dyn : { *(.rel.dyn) } :SegDYN :SegR
	.rel.plt : { *(.rel.plt) } :SegDYN :SegR
	
	. = ALIGN(0x2000);
	.data : {
		*(.data .data.*)
	} :SegRW
	/* . = ALIGN(0x2000);	/* File must be page-aligned */
	.bss : {
		*(.bss .bss.*)
	} :SegRW
	. = ALIGN(0x2000);
}

