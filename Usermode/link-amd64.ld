ENTRY(start)

/* */ PHDRS {
	SegRX PT_LOAD;
	SegR PT_LOAD FLAGS(4);	/* 4 = PT_R */
	SegRW PT_LOAD;
	SegDYN PT_DYNAMIC;
}
/* */

SECTIONS {
	. = 0x200000;
	
	.text : {
		*(.text .text.*)
	} :SegRX
	.rodata ALIGN(0x1000): {
		*(.rodata .rodata.*)
	} :SegR
	
	.dynamic : { *(.dynamic) } :SegDYN :SegR
	.dynsym : { *(.dynsym) } :SegR
	.dynstr : { *(.dynstr) } :SegR
	.rela.dyn : { *(.rela.dyn) } :SegR
	.rel.dyn : { *(.rel.dyn) } :SegR
	
	.plt : { *(.plt) } :SegR

	. = ALIGN(0x1000);
	.data ALIGN(0x1000): {
		*(.data .data.*)
	} :SegRW
	.bss ALIGN(0x1000) : {
		*(.bss .bss.*)
	} :SegRW
}

