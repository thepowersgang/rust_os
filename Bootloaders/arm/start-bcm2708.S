
#define FDT_BASE	0x100	@ Placed here by the GPU


.extern bootloader_link_addr
.extern bootloader_size

.section .text
.globl start
start:
	mov sp, #0x8000	@ Stick the stack just before the image
	
	@ Relocate the bootloader so the kernel can be loaded to the bottom of RAM
	mov r4, #0x8000	@ Source: Load address for bootloader
	ldr r5, =bootloader_link_addr
	mov r6, =bootloader_size

1:
	ldmia r4, {r0-r3}
	stmia r5, {r0-r3}
	sub r6, #4*4
	jge 1b

	@ Bootloader relocated, determine kernel size and move FDT to above the kernel.
	ldr r8, =_binary_______Kernel_bin_kernel_armv7_bin_start

	@ Get the size of the kernel image
	mov r0, r8
	bl elf_get_size
	mov r12, r0
	add r12, #0xFFF
	bic r12, #0xFFF
	@ > R12: Page aligned kernel size

	@ - Get the size of the FDT
	mov r1, #0	@ Default FDT size to zero
	ldr r0, =FDT_BASE
	ldr r1, [r0]
	ldr r2, =0xedfe0dd0
	teq r1, r2
	bne 1f
	ldr r1, [r0, #4]
	rev r1, r1
	@ R1 = Size of FDT
1:
	
	mov r10, r12	@ Save FDT address
	mov r4, #FDT_BASE
	mov r5, r10
	mov r6, r1
1:
	ldmia r4, {r0-r3}
	stmia r5, {r0-r3}
	sub r6, #4*4
	jge 1b
	mov r12, r5	@ Save the post-FDT position in R12

	@ Now that the FDT is out of the way, load the kernel and symbols
	mov r0, r8	@ < Kernel ELF file
	mov r1, #0	@ < Load address
	bl elf_load_segments
	mov r9, r0	@ Save entrypoint
	
	mov r11, r12	@ Save symbol address
	@ Load symbols
	mov r0, r8	@ < Kernel ELF file
	mov r1, r12	@ < Post-FDT position
	bl elf_load_symbols
	add r12, r12, r0
	mov r8, r9

	@ R8: Kernel entrypoint
	@ R9: UART Address
	@ R10: FDT base address
	@ R11: Symbol information base
	@ R12: End of used RAM
	@ R13: Magic
	mov r8, r9	@ < Entrypoint saved here earlier
	ldr r9, =UART_BASE
	ldr r13, =0x1badb002	@ Just ninja the mulitboot magic value
	mov pc, r8
	
	


; vim: ft=armasm
