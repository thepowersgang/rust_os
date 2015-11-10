//#define ENTRY(v)	.section .text.v ; .globl v ; .type v, "function"; v:
#define ENTRY(v)	.globl v ; .type v, "function"; v:
#define GLOBAL(v)	.globl v; v:


#include "armv7-helpers.S"

.extern main
.extern register_arguments

.section .text.start
ENTRY(start)
	bl register_arguments
	bl main
	
	mov r12, #2
	svc #2

