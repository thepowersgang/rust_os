
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
	.set log_cfg_count_inner, log_cfg_count_inner + 1 ; \
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

.set log_cfg_count_inner, 0
.section .rodata.logcfg
.align 8
.globl log_cfg_data
log_cfg_data:
	LOG_CFG_ENT("kernel::memory::heap::heapdef", LEVEL_DEBUG)	// Heap only prints higher than debug
	LOG_CFG_ENT("kernel::memory::phys"         , LEVEL_TRACE)	// PMM only prints >Trace
	//LOG_CFG_ENT("kernel::metadevs::storage", LEVEL_TRACE)
	LOG_CFG_ENT("kernel::arch::imp::acpi::internal::shim_out", LEVEL_TRACE)	// Shut up the x86 ACPI shim
	LOG_CFG_ENT("kernel::async", LEVEL_DEBUG)
	LOG_CFG_ENT("kernel::irqs", LEVEL_DEBUG)
	LOG_CFG_ENT("kernel::sync::event_channel", LEVEL_DEBUG)
	//LOG_CFG_ENT("fs_fat", LEVEL_DEBUG)
	//LOG_CFG_ENT("storage_ata::io", LEVEL_DEBUG)
	LOG_CFG_ENT("input_ps2::i8042", LEVEL_TRACE)
	LOG_CFG_ENT("syscalls::gui_calls", LEVEL_DEBUG)
.globl log_cfg_count
#log_cfg_count:	DEF_PTR( (log_cfg_count - log_cfg_data)/(8+4+4) )
log_cfg_count:	DEF_PTR( log_cfg_count_inner )

