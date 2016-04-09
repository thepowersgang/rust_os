
#include "common.S"

.section .text.start

.extern main
.extern register_arguments

.globl start
start:
	call *start_ra(%rip)
	mov $0, %rdi
	mov $0, %rsi
	call *start_mn(%rip)
	
	// Save return value for EXITPROCESS call
	mov %rax, %rdi
	mov $2, %rax
	syscall
	ud2
start_ra: .quad register_arguments
start_mn: .quad main

.section .text.memfcns
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
// RDI = Destination
// RSI = Source
// RDX = Count
.globl memmove
memmove:
	mov %rdi, %rax // Prepare to return RDI
	cmp %rdi, %rsi
	jb memcpy	// If RDI < RSI
	add %rsi, %rdx
	cmp %rdi, %rsi
	ja memcpy	// If RDI > RSI+RDX
	
	add %rdx, %rdi
	mov %rdx, %rcx
	std
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

// Unkown args, don't care
.globl _Unwind_Resume
_Unwind_Resume:
	jmp .


.section .rodata
start_msg:	.ascii "RustRT0 start"
start_msg_end:
