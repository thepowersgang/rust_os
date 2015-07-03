
.globl start
start:
	mov $0, %rax
	mov $start_msg, %rdi
	mov $(start_msg_end - start_msg), %rsi
	syscall
1:
	jmp 1b

.section .rodata
start_msg:	.ascii "RustRT0 start"
start_msg_end:
