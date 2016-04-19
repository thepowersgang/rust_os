ENTRY(start)

PHDRS {
	SegRX PT_LOAD;
	SegR PT_LOAD FLAGS(4);	/* 4 = PT_R */
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
	.rela.dyn : { *(.rela.dyn) } :SegDYN :SegR
	.rela.got : { *(.rela.got) } :SegDYN :SegR
	.rel.dyn : { *(.rel.dyn) } :SegDYN :SegR
	.rel.plt : { *(.rel.plt) } :SegDYN :SegR
	
	.plt : { *(.plt) } :SegR
	
	. = ALIGN(0x1000);
	.data ALIGN(0x1000): {
		*(.data .data.*)
	} :SegRW
	.bss ALIGN(0x1000) : {
		*(.bss .bss.*)
	} :SegRW
}

