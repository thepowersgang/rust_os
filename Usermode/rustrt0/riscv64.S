#define ENTRY(v)	.globl v ; .type v, "function"; v:
#define GLOBAL(v)	.globl v; v:


#include "riscv64-helpers.S"
#include "common.S"

.extern main
.extern register_arguments

.section .text.start
ENTRY(start)
	jal register_arguments
	jal main
	
	li a0, CORE_EXIT
	ecall

