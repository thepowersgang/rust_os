#define HEADER_MAGIC	0x71FF1013
#define HDR_ARCH_x86	1
#define HDR_ARCH_amd64	2
#define HDR_ARCH_arm32	3
#define HDR_ARCH_arm64	4

#ifdef ARCH_amd64
/*
=======================================================================
== AMD64: Aka - x86_64, x64, ...
=======================================================================
*/
.globl BASE
.globl LIMIT
.set BASE,  0x00007fffFFFE0000
.set LIMIT, 0x0000800000000000
#define RED_ZONE	128	/* the ABI defines a "red zone" above the stack for functions to dick with */

.section HEADER
.extern codesize
.extern imagesize
.globl image_header
image_header:
	.long HEADER_MAGIC
	.byte (3*4+2*8) , HDR_ARCH_amd64 , 0 , 0	/* Header size, bit-ness, pad, pad */
	.long codesize
	.long imagesize
	.long init_path - BASE
	.long init_path_end - init_path
	.quad start

.section .text
.globl start
.extern loader_main
/* RAX = Argument length */
start:
	mov %rax, %rsi	/* Save arg length in ARG2 */
	/* 0. ASSERT that the loader was loaded to the correct location */
	mov $BASE, %rax
	lea 0(%rip), %rcx
	sub %rax, %rcx
	cmp $0x1000, %rcx
	jge .bad_location
	/* 1. Initialise the stack */
	lea init_stack_end(%rip), %rsp
	sub $RED_ZONE, %rsp
	/* 2. Set up the TLS area for stack checks*/
	/* - Wait, they're disabled. Good*/
	/* 3. Call into rust */
	lea init_path(%rip), %rdi
	/* RSI = Initial RAX */
	jmp loader_main
.bad_location:
	// Incorrect location, bail hard
	mov $-2, %rdi
	mov $2, %rax
	syscall
.loop:
	jmp .loop

// RDI = Address
// RSI = Value
// RDX = Count
.globl memset
memset:
	mov %rsi, %rax
	mov %rdx, %rcx
	mov %rdi, %rdx // Save RDI
	rep stosb
	mov %rdx, %rax // because it's the return value
	ret
// RDI = Destination
// RSI = Source
// RDX = Count
.globl memcpy
memcpy:
	mov %rdx, %rcx
	mov %rdi, %rax // Prepare to return RDI
	rep movsb
	ret
;; RDI = A
;; RSI = B
;; RDX = Count
.globl memcmp
memcmp:
	mov %rdx, %rcx
	rep cmpsb
	mov $0, %rax
	ja .pos
	jb .neg
	ret
.pos:
	dec %rax
	ret
.neg:
	inc %rax
	ret
.globl _Unwind_Resume
_Unwind_Resume:
	jmp .

.section .bss
init_stack_base:
	.space 0x2000
init_stack_end:

init_path:
	.space 64
init_path_end:


#else
/*
=======================================================================
== FALLBACK
=======================================================================
*/
# error "Unknown architecture, set ARCH_<archname>"
#endif
