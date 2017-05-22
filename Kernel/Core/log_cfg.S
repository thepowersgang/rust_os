
#define LEVEL_LOG	5
#define LEVEL_DEBUG	6
#define LEVEL_TRACE	7

#if __SIZEOF_POINTER__ == 8
# define DEF_PTR(...)	.quad __VA_ARGS__
# define LOG_CFG_ENT_PAD	.long 0
#elif __SIZEOF_POINTER__ == 4
# define DEF_PTR(...)	.long __VA_ARGS__
# define LOG_CFG_ENT_PAD
#endif

#define LOG_CFG_ENT(str, level) \
	.section .rodata.logcfg ;\
	DEF_PTR(1f) ; \
	.short 2f-1f ; \
	.short level ; \
	LOG_CFG_ENT_PAD ; \
	.section .rodata.strings ; \
1: ;\
	.ascii str ;\
2: ;\
	.section .rodata.logcfg

.section .rodata.logcfg
.globl log_cfg
log_cfg:
	LOG_CFG_ENT("kernel::memory::heap::heapdef", LEVEL_DEBUG)	// Heap only prints higher than debug
	LOG_CFG_ENT("kernel::memory::phys"         , LEVEL_TRACE)	// PMM only prints >Trace
	//LOG_CFG_ENT("kernel::metadevs::storage", LEVEL_TRACE)
	LOG_CFG_ENT("kernel::arch::imp::acpi::internal::shim_out", LEVEL_TRACE)	// Shut up the x86 ACPI shim
	//LOG_CFG_ENT("kernel::async", LEVEL_DEBUG)
	//LOG_CFG_ENT("fs_fat", LEVEL_DEBUG)
	//LOG_CFG_ENT("storage_ata::io", LEVEL_DEBUG)
	LOG_CFG_ENT("input_ps2::i8042", LEVEL_TRACE)
.globl log_cfg_end
log_cfg_end:

