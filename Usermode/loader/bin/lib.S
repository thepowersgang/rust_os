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
#elif defined(ARCH_armv8)
	b .
_start:
	b .
#elif defined(ARCH_native)
_start:
	ud2
#else
# error "Unknown architecture in Usermode/loader/lib.S"
#endif

