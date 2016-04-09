.globl  new_process
.globl  start_process
.globl _start

new_process:
start_process:
#if defined(ARCH_amd64)
	jmp .
_start:
	jmp .
#elif defined(ARCH_armv7)
	b .
_start:
	b .

#else
# error "Unknown architecture in Usermode/loader/lib.S"
#endif

