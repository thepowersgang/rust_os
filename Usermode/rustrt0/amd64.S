
.section .text

.extern main
.extern register_arguments

.globl start
start:
	call register_arguments
	mov $0, %rdi
	mov $0, %rsi
	call main
	
	// Save return value for EXITPROCESS call
	mov %rax, %rdi
	mov $1, %rax
	syscall
	ud2

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

// Unkown args, don't care
.globl _Unwind_Resume
_Unwind_Resume:
	jmp .


.section .rodata
start_msg:	.ascii "RustRT0 start"
start_msg_end:
