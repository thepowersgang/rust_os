//#define ENTRY(v)	.section .text.v ; .globl v ; .type v, "function"; v:
#define ENTRY(v)	.globl v ; .type v, "function"; v:
#define GLOBAL(v)	.globl v; v:


#include "armv8-helpers.S"
#include "common.S"

.extern main
.extern register_arguments

.section .text.start
ENTRY(start)
	bl register_arguments
	bl main
	
	mov x12, #CORE_EXIT
	svc #CORE_EXIT

