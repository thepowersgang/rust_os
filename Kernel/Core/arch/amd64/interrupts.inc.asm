[section .inittext]
idt_init:
	; Save to make following instructions smaller
	mov rdi, IDT
	
	; Set an IDT entry to a callback
	%macro SETIDT 2
	mov rsi, %1
	mov rax, %2
	call set_idt
	%endmacro
	
	; Install error handlers
	%assign i 0
	mov rsi, 0
	%rep 32
	mov rax, Isr%[i]
	call set_idt
	inc rsi
	%assign i i+1
	%endrep
	; Install stub IRQs
	%assign i	32
	%rep 128-32
	mov rax, Irq%[i]
	call set_idt
	inc rsi
	%assign i i+1
	%endrep
	
	mov rdi, IDTPtr
	lidt [rdi]
	ret
; - Custom CC:
; RDI = IDT
; RSI = Index
; RAX = Address
set_idt:
	shl rsi, 4
	mov WORD [rdi + rsi], ax
	shr rax, 16
	mov WORD [rdi + rsi + 6], ax
	shr rax, 16
	mov DWORD [rdi + rsi + 8], eax
	; Enable
	mov ax, WORD [rdi + rsi + 4]
	or  ax, 0x8000
	mov WORD [rdi + rsi + 4], ax
	shr rsi, 4
	ret

[section .text]
%macro ISR_NOERRNO	1
Isr%1:
	xchg bx, bx
	push	QWORD 0
	push	QWORD %1
	jmp	ErrorCommon
%endmacro
%macro ISR_ERRNO	1
Isr%1:
	xchg bx, bx
	push	QWORD %1
	jmp	ErrorCommon
%endmacro

struc ErrorRegs
	;.gs: resq 1
	.gprs: resq 15	; SP not saved
	.num: resq 1
	.code: resq 1
	.rip: resq 1
	.cs: resq 1
endstruc

%if 0
; Doesn't work, nasm makes its own empty `debug_frame` section
[section .debug_frame]
dwunwind_ErrorCommon_cie:
	dd (.end - $) - 4; length (not including the length field)
	;dq 0xffffffffffffffff ; CIE_id
	dd 0xffffffff ; CIE_id
	db 4	; version
	db 0	; Augmentation, NUL terminated string
	db 8	; address_size
	db 0	; segment_size
	db 1	; code_alignment_factor
	db 8	; data_alignment_factor
	db 16	; return_address_register
	; Initial instructions (assuming CFA == base_rsp)
	db (2<<6)|0, 0	; DW_CFA_offset 0, 0	# RAX
	db (2<<6)|2, 1	; DW_CFA_offset 2, 1	# RCX
	db (2<<6)|1, 2	; DW_CFA_offset 1, 2	# RDX
	;	; `pusha` RSP is useless
	db (2<<6)|3, 3	; DW_CFA_offset 3, 3	# RBX
	db (2<<6)|6, 5	; DW_CFA_offset 6, 5	# RBP
	db (2<<6)|4, 6	; DW_CFA_offset 4, 6	# RSI
	db (2<<6)|5, 7	; DW_CFA_offset 5, 7	# RDI
	db (2<<6)|8, 8	; DW_CFA_offset 8, 8	# R8
	db (2<<6)|9, 9	; DW_CFA_offset 9, 9	# R9
	db (2<<6)|10, 10	; DW_CFA_offset 10, 10	# R10
	db (2<<6)|11, 11	; DW_CFA_offset 11, 11	# R11
	db (2<<6)|12, 12	; DW_CFA_offset 12, 12	# R12
	db (2<<6)|13, 13	; DW_CFA_offset 13, 13	# R13
	db (2<<6)|14, 14	; DW_CFA_offset 14, 14	# R14
	db (2<<6)|15, 15	; DW_CFA_offset 15, 15	# R15
	db (2<<6)|16, 17	; DW_CFA_offset 16, 17	# RIP
	db 0x0e, 20     	; DW_CFA_def_cfa_sf 7, 20	# Set CFA to be RSP, read from RSP[20]
	times (8 - ($ - dwunwind_ErrorCommon_cie) % 8)	db	0
.end:
dwunwind_ErrorCommon_fde:
	dd (.end - $) - 4; length (not including the length field)
	dq dwunwind_ErrorCommon_cie
	dq ErrorCommon
	dq (ErrorCommon.popstate - ErrorCommon)
.end:
[section .text]
%endif

ErrorCommon:
	PUSH_GPR
	
	mov rax, [rsp+ErrorRegs.code]	; Grab error code
	cmp rax, 0xffffffff80000000
	ja .spurrious

	mov rax, [rsp+ErrorRegs.cs]
	cmp rax, 0x08
	jz .inkernel
	cmp rax, 0x2B
	jnz .bugcheck
	; Reset the GS/FS base
	swapgs
.inkernel:

.callhandler:
	mov rdi, rsp
	[extern error_handler]
	call error_handler


	mov rax, [rsp+ErrorRegs.cs]
	cmp rax, 0x08
	jz .inkernel2
	cmp rax, 0x2B
	jnz .bugcheck
	; Reset the GS/FS base
	swapgs
.inkernel2:

.popstate:
	POP_GPR
	add rsp, 2*8
	iretq
.bugcheck:
	int 3
	jmp $
.spurrious:
	mov rdi, rsp
	[extern spurrious_handler]
	call spurrious_handler
	int3
	pop gs
	POP_GPR
	add rsp, 1*8
	iretq

ISR_NOERRNO	0;  0: Divide By Zero Exception
ISR_NOERRNO	1;  1: Debug Exception
ISR_NOERRNO	2;  2: Non Maskable Interrupt Exception
ISR_NOERRNO	3;  3: Int 3 Exception
ISR_NOERRNO	4;  4: INTO Exception
ISR_NOERRNO	5;  5: Out of Bounds Exception
ISR_NOERRNO	6;  6: Invalid Opcode Exception
ISR_NOERRNO	7;  7: Coprocessor Not Available Exception
ISR_ERRNO	8;  8: Double Fault Exception (With Error Code!)
ISR_NOERRNO	9;  9: Coprocessor Segment Overrun Exception
ISR_ERRNO	10; 10: Bad TSS Exception (With Error Code!)
ISR_ERRNO	11; 11: Segment Not Present Exception (With Error Code!)
ISR_ERRNO	12; 12: Stack Fault Exception (With Error Code!)
ISR_ERRNO	13; 13: General Protection Fault Exception (With Error Code!)
ISR_ERRNO	14; 14: Page Fault Exception (With Error Code!)
ISR_NOERRNO	15; 15: Reserved Exception
ISR_NOERRNO	16; 16: Floating Point Exception
ISR_NOERRNO	17; 17: Alignment Check Exception
ISR_NOERRNO	18; 18: Machine Check Exception
ISR_NOERRNO	19; 19: Reserved
ISR_NOERRNO	20; 20: Reserved
ISR_NOERRNO	21; 21: Reserved
ISR_NOERRNO	22; 22: Reserved
ISR_NOERRNO	23; 23: Reserved
ISR_NOERRNO	24; 24: Reserved
ISR_NOERRNO	25; 25: Reserved
ISR_NOERRNO	26; 26: Reserved
ISR_NOERRNO	27; 27: Reserved
ISR_NOERRNO	28; 28: Reserved
ISR_NOERRNO	29; 29: Reserved
ISR_NOERRNO	30; 30: Reserved
ISR_NOERRNO	31; 31: Reserved

%macro BLANKINT 1
Irq%1:
	push rbx
	mov rbx, %1
	jmp IRQCommon
%endmacro

%assign i	32
%rep 128-32
BLANKINT i
%assign i i+1
%endrep
[extern irq_handler]
IRQCommon:
	API_SAVE
	mov rdi, rbx
	call irq_handler
	API_RESTORE
	pop rbx
	iretq

; vim: ft=nasm
