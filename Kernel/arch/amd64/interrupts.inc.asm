
[section .inittext]
idt_init:
	; Save to make following instructions smaller
	mov rdi, IDT
	
	; Set an IDT entry to a callback
	%macro SETIDT 2
	mov rax, %2
	mov WORD [rdi + %1*16], ax
	shr rax, 16
	mov WORD [rdi + %1*16 + 6], ax
	shr rax, 16
	mov DWORD [rdi + %1*16 + 8], eax
	; Enable
	mov ax, WORD [rdi + %1*16 + 4]
	or  ax, 0x8000
	mov WORD [rdi + %1*16 + 4], ax
	%endmacro
	
	; Install error handlers
	%assign i 0
	%rep 32
	SETIDT i, Isr%[i]
	%assign i i+1
	%endrep
	
	mov rdi, IDTPtr
	lidt [rdi]
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
ErrorCommon:
	PUSH_GPR
	push gs
	push fs
	
	mov rdi, rsp
	[extern error_handler]
	call error_handler
	
	pop fs
	pop gs
	POP_GPR
	add rsp, 2*8
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

[extern LAPICTimerTick]
IsrLAPICTimer:
	API_SAVE
	;call LAPICTimerTick
	API_RESTORE
	iretq
