#ifdef ARCH_amd64
/*
=======================================================================
== AMD64: Aka - x86_64, x64, ...
=======================================================================
*/
.globl BASE
.set BASE, 0x200000
#define RED_ZONE	128	/* the ABI defines a "red zone" above the stack for functions to dick with */

.section HEADER
.extern codesize
.extern imagesize
.globl image_header
image_header:
	.long 0x71FF1A33	# "TIFF" "IA33" (IA-32e)
	.byte 64, 0, 0, 0
	.quad BASE
	.quad start
	.long codesize
	.long imagesize
	.quad init_path

.section .text
.globl start
.extern loader_main
start:
	/* 1. Initialise the stack */
	mov $init_stack-RED_ZONE, %rsp
	/* 2. Set up the TLS area for stack checks*/
	/* - Wait, they're disabled. Good*/
	/* 3. Call into rust */
	mov $init_path, %rsi
	mov %rax, %rdi
	jmp loader_main

// RDI = Address
// RSI = Value
// RDX = Count
.globl memset
memset:
	mov %rsi, %rax
	mov %rdx, %rcx
	rep stosb
	ret
// RDI = Destination
// RSI = Source
// RDX = Count
.globl memcpy
memcpy:
	mov %rdx, %rcx
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

// HUGE STUBBING!
// - This should be replaced with a libm
#define EXPORT(s)	.globl s ; s:
/*
EXPORT( log    )
EXPORT( log2   )
EXPORT( log10  )
EXPORT( pow    )
EXPORT( exp    )
EXPORT( exp2   )
EXPORT( ceil   )
EXPORT( floor  )
EXPORT( fmod   )
EXPORT( round  )
EXPORT( trunc  )
EXPORT( fdim   )
EXPORT( fma    )
EXPORT( sqrt   )
EXPORT( logf   )
EXPORT( log2f  )
EXPORT( log10f )
EXPORT( powf   )
EXPORT( expf   )
EXPORT( exp2f  )
EXPORT( ceilf  )
EXPORT( floorf )
EXPORT( fmodf  )
EXPORT( roundf )
EXPORT( truncf )
EXPORT( fdimf  )
EXPORT( fmaf   )
EXPORT( sqrtf  )
// Softmath conversions
EXPORT(__fixsfqi)	// Single Float -> ? Int
EXPORT(__fixsfhi)	// Single Float -> ? Int
EXPORT(__fixdfqi)	// Double Float -> ? Int
EXPORT(__fixdfhi)
EXPORT(__fixunssfqi)
EXPORT(__fixunssfhi)
EXPORT(__fixunsdfqi)
EXPORT(__fixunsdfhi)
	jmp .halt 

.halt:
	hlt
	jmp .halt
*/

.section .bss
	.space 8196
init_stack:

init_path:
	.space 64


#else
/*
=======================================================================
== FALLBACK
=======================================================================
*/
# error "Unknown architecture, set ARCH_<archname>"
#endif
