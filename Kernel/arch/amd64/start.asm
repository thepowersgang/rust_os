;
;
;
%include "arch/amd64/common.inc.asm"	; WTF Nasm

[extern low_InitialPML4]

[section .multiboot]
[global mboot]
mboot:
	MULTIBOOT_PAGE_ALIGN	equ 1<<0
	MULTIBOOT_MEMORY_INFO	equ 1<<1
	MULTIBOOT_REQVIDMODE	equ 1<<2
	MULTIBOOT_HEADER_MAGIC	equ 0x1BADB002
	MULTIBOOT_HEADER_FLAGS	equ MULTIBOOT_PAGE_ALIGN | MULTIBOOT_MEMORY_INFO | MULTIBOOT_REQVIDMODE
	MULTIBOOT_CHECKSUM	equ -(MULTIBOOT_HEADER_MAGIC + MULTIBOOT_HEADER_FLAGS)
	
	; This is the GRUB Multiboot header. A boot signature
	dd MULTIBOOT_HEADER_MAGIC
	dd MULTIBOOT_HEADER_FLAGS
	dd MULTIBOOT_CHECKSUM
	dd mboot
	; a.out kludge
	dd 0	; load_addr
	dd 0	; load_end_addr
	dd 0	; bss_end_addr
	dd 0	; entry_addr
	; Video mode
	dd 0	; Mode type (0: LFB)
	dd 0	; Width (no preference)
	dd 0	; Height (no preference)
	dd 32	; Depth (32-bit preferred)

[section .inittext]
[BITS 32]
[global start]
start:
	; 0. Save multboot state
	mov [s_multiboot_signature - KERNEL_BASE], eax
	or ebx, 0x80000000
	mov [s_multiboot_pointer - KERNEL_BASE], ebx
	
	; 1. Ensure that CPU is compatible
	mov eax, 0x80000000
	cpuid
	cmp eax, 0x80000001	; Compare the A-register with 0x80000001.
	jb not64bitCapable
	mov eax, 0x80000001
	cpuid
	test edx, 1<<29
	jz not64bitCapable
	
	mov dx, 0x3F8	; Prepare for serial debug
	
	mov al, 'O'
	out dx, al
	mov al, 'K'
	out dx, al
	
	;; 2. Switch into IA-32e mode
	; Enable:
	;   PGE (Page Global Enable)
	; + PAE (Physical Address Extension)
	; + PSE (Page Size Extensions)
	mov eax, cr4
	or eax, 0x80|0x20|0x10
	mov cr4, eax
	
	mov al, '4'
	out dx, al
	
	; Load PDP4
	mov eax, low_InitialPML4
	mov cr3, eax
	
	mov al, '3'
	out dx, al
	
	; Enable IA-32e mode
	; (Also enables SYSCALL and NX)
	mov ecx, 0xC0000080
	rdmsr
	or eax, (1 << 11)|(1 << 8)|(1 << 0)	; NXE, LME, SCE
	wrmsr
	
	mov dx, 0x3F8
	mov al, 'e'
	out dx, al

	; 3. Enable paging and enter long mode
	mov eax, cr0
	or eax, 0x80010000	; PG & WP
	mov cr0, eax
	lgdt [GDTPtr - KERNEL_BASE]
	jmp 0x08:start64
;;
;;
;;
not64bitCapable:
	mov ah, 0x0F
	mov dx, 0x3F8
	mov edi, 0xB8000
	mov esi, strNot64BitCapable
.loop:
	lodsb
	test al, al
	jz .hlt
	out dx, al
	stosw
	jmp .loop
.hlt:
	cli
	hlt
	jmp .hlt

[BITS 64]
start64:
	mov dx, 0x3F8
	mov al, '6'
	out dx, al
	
	mov rsp, KSTACK_BASE+INITIAL_KSTACK_SIZE*0x1000
	mov rax, start64_higher
	jmp rax
[section .initdata]
strNot64BitCapable:
	db "ERROR: CPU doesn't support 64-bit operation",0

[section .text]
[extern kmain]
start64_higher:
	mov al, 'H'
	out dx, al
	; 4. Set up FS/GS base for kernel
	mov rax, TID0TLS
	mov rdx, rax
	shr rdx, 32
	mov ecx, 0xC0000100	; FS Base
	wrmsr
	mov ecx, 0xC0000101	; GS Base
	wrmsr
	; 5. Set true GDT base
	lgdt [GDTPtr2 - KERNEL_BASE]
	; 6. Request setup of IRQ handlers
	call idt_init
	mov dx, 0x3F8
	mov al, 10
	out dx, al
	; 7. Call rust kmain
	call kmain
.dead_loop:
	cli
	hlt
	jmp .dead_loop
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
	%macro SETISR 1
	SETIDT %1, Isr%1
	%endmacro
	
	%assign i 0
	%rep 32
	SETISR i
	%assign i i+1
	%endrep
	
	mov rdi, IDTPtr
	lidt [rdi]
	ret

[global __morestack]
__morestack:
	ret
[global _Unwind_Resume]
_Unwind_Resume:
[global rust_eh_personality]
rust_eh_personality:
abort:
	cli
	hlt
	jmp abort

[global memset]
;; RDI = Address
;; RSI = Value
;; RDX = Count
memset:
	mov rax, rsi
	mov rcx, rdx
	rep stosb
	ret
[global memcpy]
;; RDI = Destination
;; RSI = Source
;; RDX = Count
memcpy:
	mov rcx, rdx
	rep movsb
	ret
[global memcmp]
;; RDI = A
;; RSI = B
;; RDX = Count
memcmp:
	mov rcx, rdx
	rep cmpsb
	mov rax, 0
	ja .pos
	jb .neg
	ret
.pos:
	dec rax
	ret
.neg:
	inc rax
	ret

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

;
;
;
EXPORT log
[global log2]
log2:
[global log10]
log10:
[global pow]
pow:
[global exp]
exp:
[global exp2]
exp2:
[global ceil]
ceil:
[global floor]
floor:
[global fmod]
fmod:
[global round]
round:
[global trunc]
trunc:
[global fdim]
fdim:
[global fma]
fma:
[global sqrt]
sqrt:
EXPORT logf
EXPORT log2f
EXPORT log10f
EXPORT powf
EXPORT expf
EXPORT exp2f
EXPORT ceilf
EXPORT floorf
EXPORT fmodf
EXPORT roundf
EXPORT truncf
EXPORT fdimf
EXPORT fmaf
EXPORT sqrtf
; Softmath conversions
EXPORT __fixsfqi	; Single Float -> ? Int
EXPORT __fixsfhi	; Single Float -> ? Int
EXPORT __fixdfqi	; Double Float -> ? Int
EXPORT __fixdfhi
EXPORT __fixunssfqi
EXPORT __fixunssfhi
EXPORT __fixunsdfqi
EXPORT __fixunsdfhi
	jmp halt 

halt:
	cli
	hlt
	jmp halt

;
;
;
[extern error_handler]
[global ErrorCommon]
ErrorCommon:
	PUSH_GPR
	push gs
	push fs
	
	mov rdi, rsp
	call error_handler
	
	pop fs
	pop gs
	POP_GPR
	add rsp, 2*8
	iretq

[section .padata]
[global InitialPML4]
InitialPML4:	; Covers 256 TiB (Full 48-bit Virtual Address Space)
	dd	InitialPDP - KERNEL_BASE + 3, 0	; Identity Map Low 4Mb
	times 0xA0*2-1	dq	0
	dd	StackPDP - KERNEL_BASE + 3, 0
	times 512-4-($-InitialPML4)/8	dq	0	; < dq until hit 512-4
	dd	InitialPML4 - KERNEL_BASE + 3, 0
	dq	0
	dq	0
	dd	HighPDP - KERNEL_BASE + 3, 0	; Map Low 4Mb to kernel base

[global InitialPDP]
InitialPDP:	; Covers 512 GiB
	dd	InitialPD - KERNEL_BASE + 3, 0
	times 511	dq	0

StackPDP:
	dd	StackPD - KERNEL_BASE + 3, 0
	times 511	dq	0

HighPDP:	; Covers 512 GiB
	times 510	dq	0
	dd	InitialPD - KERNEL_BASE + 3, 0
	dq	0

[global InitialPD]
InitialPD:	; Covers 1 GiB
	dd	0x000000 + 0x183,0	; Global, 2MiB
	dd	0x200000 + 0x183,0
	times 510	dq	0

StackPD:
	dd	KStackPT - KERNEL_BASE + 3, 0
	times 511	dq	0

KStackPT:	; Covers 2 MiB
	; Initial stack - 64KiB
	dq	0
	%assign i 0
	%rep INITIAL_KSTACK_SIZE-1
	dd	InitialKernelStack - KERNEL_BASE + i*0x1000 + 0x103, 0
	%assign i i+1
	%endrep
	times 512-INITIAL_KSTACK_SIZE	dq 0

InitialKernelStack:
	times 0x1000*(INITIAL_KSTACK_SIZE-1)	db 0	; 8 Pages

[section .data]
EXPORT s_multiboot_pointer
	dd 0
	dd 0xFFFFFFFF
EXPORT s_multiboot_signature
	dd 0
EXPORT GDT
	dd 0, 0
	dd 0x00000000, 0x00209A00	; 0x08: 64-bit Code
	dd 0x00000000, 0x00009200	; 0x10: 64-bit Data
	dd 0x00000000, 0x0040FA00	; 0x18: 32-bit User Code
	dd 0x00000000, 0x0040F200	; 0x20: User Data
	dd 0x00000000, 0x0020FA00	; 0x28: 64-bit User Code
	dd 0x00000000, 0x0000F200	; 0x30: User Data (64 version)
	times MAX_CPUS	dd	0, 0x00008900, 0, 0	; 0x38+16*n: TSS 0
GDTPtr:
	dw	$-GDT-1
	dd	GDT - KERNEL_BASE
	dd	0
GDTPtr2:
	dw	GDTPtr-GDT-1
	dq	GDT
EXPORT IDT
	; 64-bit Interrupt Gate, CS = 0x8, IST0 (Disabled)
	times 256	dd	0x00080000, 0x00000E00, 0, 0
IDTPtr:
	dw	256*16-1
	dq	IDT

[extern tbss_top]
EXPORT TID0TLS
	dq tbss_top
	times 0x70-8 db 0
	dq KSTACK_BASE+0x1000

; vim: ft=nasm
