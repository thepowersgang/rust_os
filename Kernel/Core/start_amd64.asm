;
;
;
%define MAX_CPUS	1
%define KSTACK_BASE	0xFFFFA00000000000
%define INITIAL_KSTACK_SIZE	16
%define KERNEL_BASE	0xFFFFFFFF80000000

[extern low_InitialPML4]
[extern low_GDTPtr]
[extern low_GDT]

[section .multiboot]
[global mboot]
mboot:
	MULTIBOOT_PAGE_ALIGN	equ 1<<0
	MULTIBOOT_MEMORY_INFO	equ 1<<1
	MULTIBOOT_HEADER_MAGIC	equ 0x1BADB002
	MULTIBOOT_HEADER_FLAGS	equ MULTIBOOT_PAGE_ALIGN | MULTIBOOT_MEMORY_INFO
	MULTIBOOT_CHECKSUM	equ -(MULTIBOOT_HEADER_MAGIC + MULTIBOOT_HEADER_FLAGS)
	
	; This is the GRUB Multiboot header. A boot signature
	dd MULTIBOOT_HEADER_MAGIC
	dd MULTIBOOT_HEADER_FLAGS
	dd MULTIBOOT_CHECKSUM

[section .inittext]
[BITS 32]
[global start]
start:
	mov dx, 0x3F8	; Prepare for serial debug
	
	; 1. Ensure that CPU is compatible
	mov eax, 0x80000000
	cpuid
	cmp eax, 0x80000001	; Compare the A-register with 0x80000001.
	jb not64bitCapable
	mov eax, 0x80000001
	cpuid
	test edx, 1<<29
	jz not64bitCapable
	
	mov al, 'O'
	out dx, al
	mov al, 'K'
	out dx, al
	out 0xe9, al
	
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
	
	mov al, 'K'
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
	
	mov al, 'e'
	out dx, al

	; 3. Enable paging and enter long mode
	mov eax, cr0
	or eax, 0x80010000	; PG & WP
	mov cr0, eax
	lgdt [low_GDTPtr]
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
	mov al, '6'
	out dx, al
	
	mov rax, start64_higher
	mov rsp, KSTACK_BASE+INITIAL_KSTACK_SIZE*0x1000
	jmp rax
[section .initdata]
strNot64BitCapable:
	db "ERROR: CPU doesn't support 64-bit operation",0

[section .text]
[extern kmain]
start64_higher:
	mov al, 'H'
	out dx, al
	mov eax, 0xC0000101	; GS Base
	mov rcx, TID0TLS
	wrmsr
	mov eax, 0xC0000100	; GS Base
	mov rcx, TID0TLS
	wrmsr
	; 4. Call rust kmain
	call kmain
.dead_loop:
	cli
	hlt
	jmp .dead_loop
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


[section .padata]
[global InitialPML4]
InitialPML4:	; Covers 256 TiB (Full 48-bit Virtual Address Space)
	dd	InitialPDP - KERNEL_BASE + 3, 0	; Identity Map Low 4Mb
	times 0xA0*2-1	dq	0
	dd	StackPDP - KERNEL_BASE + 3, 0
	times 512-4-($-InitialPML4)/8	dq	0
	dd	InitialPML4 - KERNEL_BASE + 3, 0	; Fractal Mapping
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
[global GDT]
[global GDTPtr]
GDT:
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
	dd	low_GDT
	dd	0
[global TID0TLS]
TID0TLS:
	times 0x70 db 0
	dq KSTACK_BASE+0x1000

; vim: ft=nasm
