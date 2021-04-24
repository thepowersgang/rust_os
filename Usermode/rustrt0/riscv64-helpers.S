
.section .text

#
#
#
# memset(ptr, val, count)
# - basic byte memset
.globl memset
memset:
	add a2, a0, a2
1:
	bgeu a0, a2, 2f
	sb a1, (a0)
	addi a0, a0, 1
	j 1b
2:
	ret
# memcpy
.globl memcpy
memcpy:
	add a2, a0, a2
1:
	bgeu a0, a2, 2f
	lb a3, (a1)
	sb a3, (a0)
	addi a0, a0, 1
	addi a1, a1, 1
	j 1b
2:
	ret
# memmove - Overlap-safe memcpy
.globl memmove
memmove:
	# If the source is after the destination, ...
	bgeu a1, a0, memcpy
	# ... or ends before the dest - then memcpy
	add a3, a1, a2
	bgeu a3, a0, memcpy
	# Otherwise, do a REVERSE copy
	add a0, a0, a2
	add a1, a1, a2
1:
	beq a2, zero, 2f
	addi a0, a0, -1
	addi a1, a1, -1
	addi a2, a2, -1
	lb a3, (a1)
	sb a3, (a0)
	j 1b
2:
	ret
# memcmp(p1, p2, n)
.globl memcmp
memcmp:
	add a2, a0, a2	# p1_end = p1 + n
1:
	bgeu a0, a2, 2f	# if p1 >= p1_end: 2f
	lb a3, (a0)
	lb a4, (a1)
	addi a0, a0, 1
	addi a1, a1, 1
	bgtu a3,a4, 3f
	bltu a3,a4, 4f
	j 1b
2:
	li a0, 0
	ret
3:
	li a0, 1
	ret
4:
	li a0, -1
	ret
