
.section .data

.globl RUSTOS_NATIVE_SOCKET
RUSTOS_NATIVE_SOCKET: .long 3

.section .text
.globl _Unwind_Resume
_Unwind_Resume:
    jmp .
