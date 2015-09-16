.globl  new_process
.globl _start

#if defined(ARCH_amd64)
new_process:
	jmp .
_start:
	jmp .
#elif defined(ARCH_armv7)
new_process:
	b .
_start:
	b .

#else
# error "Unknown architecture in Usermode/loader/lib.S"
#endif

