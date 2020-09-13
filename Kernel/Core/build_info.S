

#define DEF_STR(name, str)\
	.globl name ;\
	name: .short (1f - name)-2;\
	.ascii str ; \
	1:

.section .rodata
DEF_STR(VERSION_STRING, "Tifflin Kernel v" TK_VERSION " build " TK_BUILD)
DEF_STR(BUILD_STRING, "Git state : " TK_GITSPEC ", Built with " RUST_VERSION)
