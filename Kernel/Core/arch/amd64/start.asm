; "Tifflin" Kernel
; - By John Hodge (thePowersGang)
;
; arch/amd64/start.asm
; - AMD64/IA-32e boot shim
%include "Core/arch/amd64/common.inc.asm"	; WTF Nasm

[extern low_InitialPML4]

[section .multiboot]
[global mboot]
mboot:
	%define MULTIBOOT_PAGE_ALIGN	1<<0
	%define MULTIBOOT_MEMORY_INFO	1<<1
	%define MULTIBOOT_REQVIDMODE	1<<2
	%define MULTIBOOT_HEADER_MAGIC	0x1BADB002
	%define MULTIBOOT_HEADER_FLAGS	(MULTIBOOT_PAGE_ALIGN | MULTIBOOT_MEMORY_INFO | MULTIBOOT_REQVIDMODE)
	%define MULTIBOOT_CHECKSUM	-(MULTIBOOT_HEADER_MAGIC + MULTIBOOT_HEADER_FLAGS)
	
	; This is the GRUB Multiboot header. A boot signature
	dd MULTIBOOT_HEADER_MAGIC
	dd MULTIBOOT_HEADER_FLAGS
	dd MULTIBOOT_CHECKSUM
	dd mboot
	; a.out kludge (not used)
	dd 0	; load_addr
	dd 0	; load_end_addr
	dd 0	; bss_end_addr
	dd 0	; entry_addr
	; Video mode
	dd 0	; Mode type (0: LFB)
	;dd 0,0	; Width, Height (no preference)
	;dd 1600,900	; Width, Height ('HD+')
	dd 1366,768	; Width, Height ('HD+')
	;dd 1024,768	; Width, Height ('HD+')
	dd 32	; Depth (32-bit preferred)

[section .inittext]
[BITS 32]
[global start]
start:
	; NOTE: If this passes, it's being run in 64-bit mode
	cmp ecx, 0x71FF0EF1
	jz start_uefi
	
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
	
	mov al, 0x10
	out dx, al
	mov al, 'O'
	out dx, al
	mov al, 'K'
	out dx, al
	
	;; 2. Switch into IA-32e mode
	; Enable:
	;   [4] PGE (Page Global Enable)
	; + [5] PAE (Physical Address Extension)
	; + [7] PSE (Page Size Extensions)
	; + [ 9] OSFXSR (Operating System Support for FXSAVE and FXRSTOR instructions)
	; + [10] OSXMMEXCPT (Operating System Support for Unmasked SIMD Floating-Point Exceptions)
	mov eax, cr4
	or eax, 0x80|0x20|0x10
        or ax, (1 << 9)|(1 << 10)
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

	
	; 3. Enable paging and enter long mode (enable SSE too)
	; Set [31] PG (Paging enabled)
	; Set [16] WP (Kernel write-protect)
	; Set [3]TS (Enables #NM on all FPU instructions)
	; Set [1]MP (with TS, Enables #NM when FWAIT is used)
	; Clear [2]EM (Disables emulation of the FPU)
	mov eax, cr0
	or eax, 0x80010000|(1 << 3)|(1 << 1)	; PG & WP
	and ax, ~(1 << 2)
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
start_uefi:
	mov [s_multiboot_signature - KERNEL_BASE], ecx
	or edx, 0x80000000
	mov [s_multiboot_pointer - KERNEL_BASE], edx
	mov [s_multiboot_pointer - KERNEL_BASE + 4], DWORD 0xFFFFFFFF
	
	;; 1. Enable a nice set of features
	; Enable:
	;   [4] PGE (Page Global Enable)
	; + [5] PAE (Physical Address Extension)
	; + [7] PSE (Page Size Extensions)
	; + [ 9] OSFXSR (Operating System Support for FXSAVE and FXRSTOR instructions)
	; + [10] OSXMMEXCPT (Operating System Support for Unmasked SIMD Floating-Point Exceptions)
	mov rax, cr4
	or eax, 0x80|0x20|0x10
        or ax, (1 << 9)|(1 << 10)
	mov cr4, rax
	
	; Enable IA-32e mode
	; (Also enables SYSCALL and NX)
	mov ecx, 0xC0000080
	rdmsr
	or eax, (1 << 11)|(1 << 8)|(1 << 0)	; NXE, LME, SCE
	wrmsr
	
	; Load PDP4
	mov eax, low_InitialPML4
	mov cr3, rax
	; 2. Enable paging and enter long mode (enable SSE too)
	; Set [31] PG (Paging enabled)
	; Set [16] WP (Kernel write-protect)
	; Set [3]TS (Enables #NM on all FPU instructions)
	; Set [1]MP (with TS, Enables #NM when FWAIT is used)
	; Clear [2]EM (Disables emulation of the FPU)
	mov rax, cr0
	or eax, 0x80010000|(1 << 3)|(1 << 1)	; PG & WP
	and ax, ~(1 << 2)
	mov cr0, rax
	
	lgdt [DWORD GDTPtr - KERNEL_BASE]
	; - Far jump (indirect)
	jmp far [.ljmp_addr]

[section .initdata]
.ljmp_addr:
	dq	start64
	dw	0x08
[section .inittext]

[extern prep_tls]
start64:
	mov dx, 0x3F8
	mov al, '6'
	out dx, al
	
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
	; 4. Set true GDT base
	lgdt [a32 DWORD GDTPtr2 - KERNEL_BASE]
	; Load segment regs
	mov ax, 0x10
	mov ds, ax
	mov ss, ax
	mov es, ax
	mov fs, ax
	mov gs, ax
	
	; 5. Initialise TLS for TID0
	; - Use a temp stack for the following function
	mov rsp, KSTACK_BASE+0x1000+1024
	mov rax, KSTACK_BASE+0x1000
	mov [rsp+14*8], rax
	; - Pass the stack top, bottom, and TID0 pointer (null)
	mov rdi, KSTACK_BASE+INITIAL_KSTACK_SIZE*0x1000
	mov rsi, KSTACK_BASE+0x1000
	mov rdx, 0
	; - Prepare the TLS region
	call prep_tls
	; Switch to the real stack
	mov [rel s_tid0_tls_base], rax
	mov rsp, rax
	
	; 5. Set up FS/GS base for kernel
	mov rax, rsp
	mov rdx, rax
	shr rdx, 32
	mov ecx, 0xC0000100	; FS Base
	wrmsr
	mov ecx, 0xC0000101	; GS Base
	wrmsr
	; 6. Request setup of IRQ handlers
	call idt_init
	mov dx, 0x3F8
	mov al, 10
	out dx, al
	
	; Bind the 'SYSCALL' handler (and set flags for it)
	; LSTAR = 0xC000_0082
	mov rax, syscall_handler
	mov rdx, rax
	shr rdx, 32
	mov ecx, 0xC0000082
	wrmsr
	; STAR = 0xC000_0081
	mov eax, 0x00000000
	mov edx, 0x001B0008	; [63:48] User CS, [47:32] Kernel CS
	mov ecx, 0xC0000081
	wrmsr
	; FMASK = 0xC000_0084
	mov eax, 0x200	; - Clear IF on SYSCALL
	mov edx, 0
	mov ecx, 0xC0000084
	wrmsr
	
	mov rax, InitialPML4
	mov QWORD [rax], 0
	; 7. Call rust kmain
	call kmain
.dead_loop:
	cli
	hlt
	jmp .dead_loop

%include "Core/arch/amd64/interrupts.inc.asm"

; RDI: Save location for RSP
; RSI: New RSP (pointer)
; RDX: New FSBASE
; RCX: New CR3
[section .text.asm.task_switch]
EXPORT task_switch
	push rbp
	mov rbp, rsp
	SAVE rbx, r12, r13, r14, r15
	
	; Perfom context save/restore
	mov [rdi], rsp	; Save RSP
	mov rsp, [rsi]	; New RSP
	mov cr3, rcx	; New CR3
	invlpg [rsp]
	
	; Update stack top (RSP0) and TLS base (GS)
	; TLS base and stack top are the same address.
	; TODO: This assumes uniprocessor
	mov [rel TSSes+tss.rsp0], rdx
	mov rax, rdx
	shr rdx, 32	; EDX = High
	mov ecx, 0xC0000101	; GS Base
	wrmsr
	
	; Restore saved registers and return
	RESTORE rbx, r12, r13, r14, r15
	; - For debugging, reset RBP and trigger a tracepoint
	;mov rbp, rsp
	;int3
	pop rbp
	ret

[section .text]
EXPORT thread_trampoline
	pop rax	; 1. Pop thread root method off stack
	mov rdi, rsp	; 2. Set RDI to the object to call
	jmp rax	; 3. Jump to the thread root method, which should never return

; RDI: IP
; RDI: SP
; RDX: Arg
EXPORT drop_to_user
	mov rcx, rdi	; Set IP for SYSRET
	pushf
	cli
	pop r11	; Set RFLAGS for SYSRET
	swapgs
	mov ax, 0x23
	mov ds, ax
	mov es, ax
	mov fs, ax
	mov gs, ax
	mov rsp, rsi	; User's stack
	mov rax, rdx	; Argument passed in RAX
	db 0x48
	sysret

; -------------------------------------------------
; System Calls
; -------------------------------------------------
[section .text.asm.syscall_handler]
; RAX, RDI, RSI, RDX, [RCX/R10], R8, R9
EXPORT syscall_handler
	; RCX = RIP, R11 = EFLAGS

	; NOTE: If an interrupt happens between here and the load of `RSP`,
	; there can be state corruption.
	; - RFLAGS has IF cleared (loaded state)
	; - An NMI ignores that, but _should_ be using its own stack
	; TODO TODO TODO Actually use the IST for NMI
	
	; >>> Switch to kernel stack
	; - The format of 'gs' is specified in arch/amd64/threads.rs (TLSData)
	swapgs
	mov [gs:0x10], rsp	; Save user's RSP
	mov rsp, [gs:0x8]	; and load kernels
	; >>> Save user state
	SAVE rcx, r11	; RCX = userland IP, R11 = userland EFLAGS
	; >>> Push args (ready to be passed as slice)
	SAVE rdi, rsi, rdx, r10, r8, r9
	sti
	
	mov rdi, rax
	mov rsi, rsp
	mov rdx, 6
	[extern syscalls_handler]
	call syscalls_handler
	
	; "pop" the arguments
	RESTORE rdi, rsi, rdx, r10, r8, r9
	
	; All done
	; >>> Restore RCX/R11 for sysret
	RESTORE rcx, r11
	; >>> Restore user's SP
	mov rsp, [gs:0x10]
	; >>> TODO: Restore user's FS
	; >>> Restore GS
	swapgs
	; sysretq (no opcode for it in nasm)
	; - Returns to 64-bit mode, let's ignore compat mode
	db 0x48
	sysret

; -------------------------------------------------
; Helpers
; -------------------------------------------------
[section .text]
EXPORT __morestack
	jmp abort
abort:
	ud2
	cli
	hlt
	jmp abort

;; RDI = Address
;; RSI = Value
;; RDX = Count
EXPORT memset
	mov rax, rsi
	mov rcx, rdx
	rep stosb
	ret
;; RDI = Destination
;; RSI = Source
;; RDX = Count
EXPORT memcpy
	mov rax, rdi ; Prepare to return RDI
	mov rcx, rdx
	; Check if a word-wise copy is possible
	test di, 7
	jnz .byte
	test si, 7
	jnz .byte
	test cx, 7
	jnz .byte
	shr rcx, 3
	rep movsq
	ret
.byte:
	rep movsb
	ret
;; RDI = Destination
;; RSI = Source
;; RDX = Count
EXPORT memmove
	cmp rdi, rsi
	jz .ret 	; if RDI == RSI, do nothinbg
	jb memcpy	; if RDI < RSI, it's safe to do a memcpy
	add rsi, rdx	; RDI > RSI
	cmp rdi, rsi
	jae memcpy	; if RDI >= RSI + RDX, then the two regions don't overlap, and memcpy is safe
	; Reverse copy (add count to both addresses, and set DF)
	add rdi, rdx
	dec rdi
	dec rsi
	std
	mov rcx, rdx
	rep movsb
	cld
.ret:
	ret
;; RDI = A
;; RSI = B
;; RDX = Count
EXPORT memcmp
	test rdx, rdx
	mov rcx, rdx
	rep cmpsb
	mov rax, 0
	ja .pos
	jb .neg
.eq:
	ret
.pos:
	dec rax
	ret
.neg:
	inc rax
	ret
;; RDI = str
EXPORT strlen
	mov rsi, rdi
	mov rcx, 0
.loop:
	lodsb
	test al, al
	loopnz .loop
	neg rcx
	mov rax, rcx
	ret

EXPORT _Unwind_Resume
	jmp $
%include "Core/arch/amd64/stubs.inc.asm"

[section .padata]
[global InitialPML4]
InitialPML4:	; Covers 256 TiB (Full 48-bit Virtual Address Space)
	dd	InitialPDP - KERNEL_BASE + 3, 0	; Identity Map Low 4Mb
	times 0xA0*2-1	dq	0
	; Stacks at 0xFFFFA...
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
	dd	0x200000 + 0x183,0	; Global, 2MiB
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

[section .rodata]

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
.first_tss:
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
EXPORT s_tid0_tls_base
	dq	0

[section .bss]
EXPORT TSSes
	times MAX_CPUS resb tss.SIZE

; vim: ft=nasm
