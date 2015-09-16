
.globl __aeabi_memcpy4
.globl __aeabi_memcpy8
__aeabi_memcpy8:
__aeabi_memcpy4:
1:
	LDR r3, [r1], #4
	STR r3, [r0], #4
	SUBS r2, r2, #4
	BGE 1b
	BX lr

.globl __aeabi_memset4
.globl __aeabi_memset8
__aeabi_memset4:
__aeabi_memset8:
	LSL r3, r1, #8
	ORR r1, r1, r3
	LSL r3, r1, #16
	ORR r1, r1, r3
1:
	STR r1, [r0], #0
	SUBS r2, r2, #4
	BGE 1b
	BX lr
.globl __aeabi_memclr4
.globl __aeabi_memclr8
__aeabi_memclr4:
__aeabi_memclr8:
	MOV r1, #0
	b 1b

.globl __aeabi_memset
__aeabi_memset:
	b .
.globl __aeabi_memclr
__aeabi_memclr:
	b .

.globl __aeabi_memcpy
__aeabi_memcpy:
	b .


.globl memcmp
memcmp:
	b .

.globl __aeabi_unwind_cpp_pr0
.globl __aeabi_unwind_cpp_pr1
__aeabi_unwind_cpp_pr0:
__aeabi_unwind_cpp_pr1:
	b .
