
%define MAX_CPUS	1
%define KSTACK_BASE	0xFFFFA00000000000
%define INITIAL_KSTACK_SIZE	16
%define KERNEL_BASE	0xFFFFFFFF80000000

; Save a list of registers to the stack
%macro SAVE 0-*
	sub rsp, (%0)*8
%assign POS 0
%rep %0
	mov [rsp+POS], %1
%assign POS POS+8
%rotate 1
%endrep
%endmacro
; Restore a list of registers
%macro RESTORE 0-*
%assign POS 0
%rep %0
	mov %1, [rsp+POS]
%assign POS POS+8
%rotate 1
%endrep
	add rsp, (%0)*8
%endmacro

%macro PUSH_GPR	0
	SAVE rax, rcx, rdx, rbx,  rbp, rsi, rdi, r8, r9, r10, r11, r12, r13, r14, r15
%endmacro
%macro POP_GPR	0
	RESTORE rax, rcx, rdx, rbx,  rbp, rsi, rdi, r8, r9, r10, r11, r12, r13, r14, r15
%endmacro

; RBP, RBX, R12-R15 are callee save (RSP ignored)
%define API_SAVE_SIZE	(8*(3+2+4))	; Data-BX, Addr-BP/SP, Half of top
%macro API_SAVE 0
	SAVE    rax, rcx, rdx, rsi, rdi, r8, r9, r10, r11
%endmacro
%macro API_RESTORE 0
	RESTORE rax, rcx, rdx, rsi, rdi, r8, r9, r10, r11
%endmacro


%macro EXPORT 1
[global %1]
%1:
%endmacro

; vim: ft=nasm
