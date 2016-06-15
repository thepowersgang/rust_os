/*
 */
#define HEADER_MAGIC	0x71FF1013
#define HDR_ARCH_x86	1
#define HDR_ARCH_amd64	2
#define HDR_ARCH_arm32	3
#define HDR_ARCH_arm64	4

#define ENTRY(v)	.globl v; .type v, "function"; v:
#define GLOBAL(v)	.globl v; v:

.globl BASE
.globl LIMIT

#include "../rustrt0/common.S"

#ifdef ARCH_amd64
# define DEFPTR 	.quad
# define PTR_SIZE	8
# define HDR_ARCH	HDR_ARCH_amd64
/*
=======================================================================
== AMD64: Aka - x86_64, x64, ...
=======================================================================
*/
.set BASE,  0x00007fffFFFE0000
.set LIMIT, 0x0000800000000000
#define RED_ZONE	128	/* the ABI defines a "red zone" above the stack for functions to dick with */

.section .text
.extern loader_main
/* RAX = Argument length */
ENTRY(start)
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
ENTRY(memset)
	mov %rsi, %rax
	mov %rdx, %rcx
	mov %rdi, %rdx // Save RDI
	rep stosb
	mov %rdx, %rax // because it's the return value
	ret
// RDI = Destination
// RSI = Source
// RDX = Count
ENTRY(memcpy)
	mov %rdx, %rcx
	mov %rdi, %rax // Prepare to return RDI
	rep movsb
	ret
;; RDI = A
;; RSI = B
;; RDX = Count
ENTRY(memcmp)
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
ENTRY(_Unwind_Resume)
	jmp .

#elif defined(ARCH_armv7)

# define DEFPTR	.long
# define PTR_SIZE	4
# define HDR_ARCH HDR_ARCH_arm32
.set BASE,  0x7fbc0000
.set LIMIT, 0x80000000

.section .text
.globl start
.extern loader_main
/* R0 = Argument length */
start:
	@ 0. ASSERT that the loader was loaded to the correct location
	ldr r1, =BASE
	sub r1, r1, pc
	cmp r1, #0x1000
	bge bad_location
	@ 1. Initialise the stack
	ldr sp, =init_stack_end
	@ 2. Set up the TLS area for stack checks
	@ - Wait, they're disabled. Good
	@ 3. Call into rust
	mov r1, r0	@ R0 is still the argument length
	ldr r0, =init_path
	b loader_main
bad_location:
	// Incorrect location, bail hard
	mov r0, #-2
	mov r12, #2	@ R12 = Call ID
	svc #2	@ Call ID too (TODO: Actually use this in kernel-land)
	b .

#include "../rustrt0/armv7-helpers.S"

#else
/*
=======================================================================
== FALLBACK
=======================================================================
*/
# error "Unknown architecture, set ARCH_<archname>"
#endif

.section HEADER
.extern codesize
.extern imagesize
GLOBAL(image_header)
	.long HEADER_MAGIC
	.byte (5*4+PTR_SIZE) , HDR_ARCH , 0 , 0	/* Header size, bit-ness, pad, pad */
	.long codesize
	.long imagesize
	.long init_path - BASE
	.long init_path_end - init_path
	DEFPTR start

.section .bss
	.space 0x1000	/* Guard page */
GLOBAL(init_stack_base)
	.space 0x8000
GLOBAL(init_stack_end)

GLOBAL(arg_count)	.space 4
GLOBAL(init_path)	.space 1024
GLOBAL(init_path_end)

/* vim: ft=gas
 */
