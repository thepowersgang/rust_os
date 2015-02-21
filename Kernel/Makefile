# "Tifflin" Kernel
# - By John Hodge (thePowersGang)
#
# Makefile
# - Kernel core makefile
#
# TODO:
# - Module support
# - Multiple architecture configurations
ARCH = amd64
TRIPLE = x86_64-none-elf
MODS := video_vga storage_ata

OBJS := start.ao $(MODS:%=mod_%.o) libkernel.rlib libcore.rlib

-include BuildNum.$(ARCH).mk
BUILDNUM ?= 1

V ?= @

nop :=
space := $(nop) $(nop)
comma := ,

# Function: Reference a locally-compile crate
LocalLib = --extern $1=$(OBJDIR)lib$1.rlib

EXCLUDE_FEATURES := 3dnow avx avx2 sse sse2 sse3 sse4.1 sse4.2 mmx
TARGET_FILE := Core/arch/$(ARCH)/target.json

OBJDIR := .obj/$(ARCH)/

RUSTFLAGS := -L $(OBJDIR)
RUSTFLAGS += -O
RUSTFLAGS += -g
#RUSTFLAGS += --debuginfo 1
#RUSTFLAGS += -C code-model=kernel
RUSTFLAGS += -C soft-float	# Disables requiring SSE etc
RUSTFLAGS += -C target-feature=$(subst $(space),$(comma),$(EXCLUDE_FEATURES:%=-%))
RUSTFLAGS += --cfg arch__$(ARCH)
RUSTFLAGS += --target=$(TARGET_FILE)
#RUSTFLAGS += -Z time-passes

LINKFLAGS := -T Core/arch/$(ARCH)/link.ld -Map $(OBJDIR)map.txt -g
LINKFLAGS += -z max-page-size=0x1000	# Reduces binary size
LINKFLAGS += --gc-sections	# Prevents complaining about floating point methods

ENV := RUST_VERSION="$(shell rustc --version)"
ENV += TK_GITSPEC="$(shell git log -n 1 | head -n 1 | awk '{print $$2}')"
ENV += TK_VERSION="0.1"
ENV += TK_BUILD="$(shell hostname --fqdn):$(BUILDNUM)"

ifneq ($(USE_RUST_MASTER),)
RUSTSRC := /home/tpg/Source/rust/rust-master/
else
RUSTSRC := /home/tpg/Source/rust/rustc-nightly/
endif
LIBCORESRC := $(RUSTSRC)src/libcore/
LIBGCC := /home/tpg/apps/lib/gcc/$(TRIPLE)/4.7.1/libgcc.a

OBJS := $(OBJS:%=$(OBJDIR)%)

all: kernel-$(ARCH).bin

clean:
	rm -r $(OBJDIR)

pretty:
	$V$(ENV) rustc -Z unstable-options --pretty expanded $(RUSTFLAGS) -o kernel-$(ARCH).exp_rs Core/main.rs $(call LocalLib,core)

docs:
	@mkdir -p doc/Kernel
	$V$(ENV) rustdoc -o doc/Kernel Core/main.rs --cfg arch__amd64
run: kernel-$(ARCH).bin
	./RunQemuPXE $<.elf32 ""

kernel-$(ARCH).bin: $(OBJS) Core/arch/$(ARCH)/link.ld Makefile
	@echo [LD] -o $@
	@$(TRIPLE)-ld -o $@ $(OBJS) $(LIBGCC) $(LINKFLAGS)
	@objcopy $@ -F elf32-i386 $@.elf32
	#@mcopy -i fdd.img $@.elf32 ::/ -D o
	@objdump -C -S $@ > $@.dsm
	@echo "BUILDNUM := $$(( $(BUILDNUM) + 1 ))" > BuildNum.$(ARCH).mk

$(OBJDIR)%.ao: Core/arch/$(ARCH)/%.asm Makefile
	@echo [AS] -o $@
	@mkdir -p $(dir $@)
	@nasm -o $@ $< -f elf64 -MD $@.d -MP

$(OBJDIR)kernel.ir: Core/main.rs $(OBJDIR)libcore.rlib $(TARGET_FILE)
	$V$(ENV) rustc $(RUSTFLAGS) --emit=llvm-ir -o $@ $< $(call LocalLib,core)

$(OBJDIR)libkernel.rlib: Core/main.rs $(OBJDIR)libcore.rlib Makefile $(TARGET_FILE)
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) rustc $(RUSTFLAGS) --emit=link,dep-info -o $@ $< $(call LocalLib,core)

# - Locally compiled libcore, needs to not use SSE
$(OBJDIR)libcore.rlib: $(LIBCORESRC)lib.rs $(TARGET_FILE)
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) rustc $(RUSTFLAGS) --emit=link,dep-info --crate-name core -o $@ $<

# - Modules
$(OBJDIR)mod_%.o: Modules/%/lib.rs $(OBJDIR)libcore.rlib $(OBJDIR)libkernel.rlib
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) rustc $(RUSTFLAGS) --emit=obj,dep-info --crate-name=$* --crate-type=lib -o $@ $< $(call LocalLib,core) $(call LocalLib,kernel)

-include $(OBJDIR)libkernel.d
-include $(OBJDIR)libcore.d
-include $(patsubst %,%.d,$(OBJS))



