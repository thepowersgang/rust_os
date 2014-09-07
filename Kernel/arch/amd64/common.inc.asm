
%define MAX_CPUS	1
%define KSTACK_BASE	0xFFFFA00000000000
%define INITIAL_KSTACK_SIZE	16
%define KERNEL_BASE	0xFFFFFFFF80000000
%macro SAVE_GPR 1
	mov [%1-0x08], r15
	mov [%1-0x10], r14
	mov [%1-0x18], r13
	mov [%1-0x20], r12
	mov [%1-0x28], r11
	mov [%1-0x30], r10
	mov [%1-0x38], r9
	mov [%1-0x40], r8
	mov [%1-0x48], rdi
	mov [%1-0x50], rsi
	mov [%1-0x58], rbp
	mov [%1-0x60], rsp
	mov [%1-0x68], rbx
	mov [%1-0x70], rdx
	mov [%1-0x78], rcx
	mov [%1-0x80], rax
%endmacro

%macro PUSH_GPR	0
	SAVE_GPR rsp
	sub rsp, 0x80
%endmacro

%macro RESTORE_GPR 1
	mov r15, [%1-0x08]
	mov r14, [%1-0x10]
	mov r13, [%1-0x18]
	mov r12, [%1-0x20]
	mov r11, [%1-0x28]
	mov r10, [%1-0x30]
	mov r9,  [%1-0x38]
	mov r8,  [%1-0x40]
	mov rdi, [%1-0x48]
	mov rsi, [%1-0x50]
	mov rbp, [%1-0x58]
	;mov rsp, [%1-0x60]
	mov rbx, [%1-0x68]
	mov rdx, [%1-0x70]
	mov rcx, [%1-0x78]
	mov rax, [%1-0x80]
%endmacro

%macro POP_GPR	0
	add rsp, 0x80
	RESTORE_GPR rsp
%endmacro

%macro EXPORT 1
[global %1]
%1:
%endmacro
