ENTRY(start)

/*SEARCH_DIR( .objs/amd64/libs )*/
SEARCH_DIR( .output/amd64/libs )
STARTUP( .objs/amd64/rustrt0.o )

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
	.plt : {
		*(.plt)
	} :SegRX

	.rodata ALIGN(0x1000): {
		*(.rodata .rodata.*)
	} :SegR
	
	/* Dynamic linking info */
	.dynamic : { *(.dynamic) } :SegDYN :SegR
	.dynsym : { *(.dynsym) } :SegDYN :SegR
	.dynstr : { *(.dynstr) } :SegDYN :SegR
	.rela.got : { *(.rela.got) } :SegDYN :SegR
	.rela.dyn : { *(.rela.dyn) } :SegDYN :SegR
	.rel.dyn : { *(.rel.dyn) } :SegDYN :SegR

	. = ALIGN(0x1000);
	.data ALIGN(0x1000): {
		*(.data .data.*)
	} :SegRW
	.bss ALIGN(0x1000) : {
		*(.bss .bss.*)
	} :SegRW
}

