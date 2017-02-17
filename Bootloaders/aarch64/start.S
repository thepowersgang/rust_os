
// "platform-$(PLATFORM).h" is inserted by cpp

#define STACK_TOP	(RAM_START+RAM_LENGTH)

#define ENTRY(n)	.globl n ; n:

#include "../../Usermode/rustrt0/armv8-helpers.S"

.extern _binary_kernel_bin_start

.section .text
ENTRY(start)
	mov x0, #STACK_TOP
	mov sp, x0
	mov x0, #UART_BASE
	str x0, [sp, #-8]!
	sub sp, sp, #8	// Align the stack

	ldr x0, =string_init
	mov x1, #(string_init_END-string_init)
	bl puts

	// Get the size of the kernel image
	ldr x0, =_binary_kernel_bin_start
	bl elf_get_size
	mov x12, x0	// X12 = ELF Size
	
	mov x1, #RAM_START
	add x21, x0, x1	// X21 = End of loaded image
	
#if defined(FDT_BASE)
	mov x20, #0	// Pre-set FDT base register to 0 (invalid)
	ldr x0, =FDT_BASE
	// Compare `*(FDT_BASE as *const u32) == 0xedfe0dd0`
	ldr w1, [x0]
	ldr w2, =0xedfe0dd0
	cmp w1, w2
	bne 1f
	// X1 = Size of FDT
	mov x1, #0
	ldr w1, [x0, #4]
	rev w1, w1
	// R1 = Size of FDT
	
	mov x20, x21	// Save FDT base
	add x21, x21, x1	// Increment free RAM base by size of FDT
	
	mov x2, x1	// Count
	mov x1, x0	// Source
	mov x0, x20	// Destination
	bl __aeabi_memcpy4
1:
#else
.extern fdt_start
.extern fdt_len
	ldr x2, =fdt_end
	ldr x1, =fdt_start
	sub x2, x2, x1
	mov x20, x21	// Save FDT base
	add x21, x21, x2	// Increment free RAM base by size of FDT
	
	mov x0, x20	// Destination
	bl __aeabi_memcpy4
#endif
	
	// Load image
	ldr x0, =_binary_kernel_bin_start
	mov x1, #RAM_START
	bl elf_load_segments
	mov x19, x0	// Save entrypoint
	
	// Load symbols
	ldr x0, =_binary_kernel_bin_start
	mov x1, x21
	bl elf_load_symbols
	add x12, x21, x0
	
	mov x10, x20
	mov x11, x21
	// R8: Kernel entrypoint
	// R9: UART Address
	// R10: FDT base address
	// R11: Symbol information base
	// R12: End of used RAM
	// R13: Magic
	ldr x9, =UART_BASE
	ldr x13, =0x1badb002	// Just ninja the mulitboot magic value
	blr x19
	
	
	ldr x0, =string_boot_fail
	mov x1, #(string_boot_fail_END-string_boot_fail)
	bl puts
	b .


ENTRY(putb)
	mov x1, #STACK_TOP
	ldr x1, [x1, #-8]
	strb w0, [x1]
	ret
// puts(*const u8, usize)
ENTRY(puts)
	mov x2, #STACK_TOP
	ldr x2, [x2, #-8]
	tst x1, x1
	beq 2f
1:
	ldrb w3, [x0], #1
	strb w3, [x2]
	subs x1,x1, #1
	bne 1b
2:
	ret

.section .rodata
string_init:
	.string "=== BOOTING ===\n"
string_init_END:
string_boot_fail:
	.string "Boot failure.\n"
string_boot_fail_END:

// vim: ft=armasm
