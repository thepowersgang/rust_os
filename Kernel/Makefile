#
#
#
ARCH = amd64
OBJS := start.ao kcore.o libcore.rlib

V ?= @

nop :=
space := $(nop) $(nop)
comma := ,

# Function: Reference a locally-compile crate
LocalLib = --extern $1=$(OBJDIR)lib$1.rlib

EXCLUDE_FEATURES := 3dnow avx avx2 sse sse2 sse3 sse4.1 sse4.2 mmx

RUSTFLAGS := --crate-type lib
#RUSTFLAGS += --debuginfo 1
#RUSTFLAGS += -C code-model=kernel
RUSTFLAGS += -C soft-float
RUSTFLAGS += -C no-redzone
RUSTFLAGS += -C target-feature=$(subst $(space),$(comma),$(EXCLUDE_FEATURES:%=-%))
RUSTFLAGS += --cfg arch__$(ARCH)
#RUSTFLAGS += -Z time-passes
#RUSTFLAGS += --target=x86_64-pc-elf 

ENV := RUST_VERSION="$(shell rustc --version)"
ENV += TK_GITSPEC="$(shell git log -n 1 | head -n 1 | awk '{print $$2}')"
ENV += TK_VERSION="0.1"
ENV += TK_BUILD="0"

LIBDIR := /home/tpg/apps/lib/rustlib/x86_64-unknown-linux-gnu/lib/
LIBCOMPILERRT := $(LIBDIR)libcompiler-rt.a
LIBCORESRC := /home/tpg/Source/rust/rust-nightly/src/libcore/
LIBGCC := $(shell x86_64-elf-gcc -print-path-libgcc)

OBJDIR := .obj/$(ARCH)/
OBJS := $(OBJS:%=$(OBJDIR)%)

all: kernel-$(ARCH).bin

clean:
	rm -r $(OBJDIR)

run: kernel-$(ARCH).bin
	./RunQemuPXE $<.elf32 ""

kernel-$(ARCH).bin: $(OBJS) arch/$(ARCH)/link.ld Makefile
	@echo [LD] -o $@
	@x86_64-elf-ld -o $@ $(OBJS) $(LIBCORE) $(LIBCOMPILERRT) $(LIBGCC) -T arch/$(ARCH)/link.ld -Map $(OBJDIR)map.txt -z max-page-size=0x1000 -g
	@objcopy $@ -F elf32-i386 $@.elf32
	@mcopy -i fdd.img $@.elf32 ::/ -D o
	@objdump -S $@ > $@.dsm

$(OBJDIR)%.ao: arch/$(ARCH)/%.asm Makefile
	@echo [AS] -o $@
	@mkdir -p $(dir $@)
	@nasm -o $@ $< -f elf64 -MD $@.dep -MP

$(OBJDIR)kcore.o: Core/main.rs $(OBJDIR)libcore.rlib Makefile
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) rustc -O $(RUSTFLAGS) --emit obj -o $@ $< --dep-info $@.dep $(call LocalLib,core) $(call LocalLib,common)

$(OBJDIR)libcommon.rlib: common/lib.rs $(OBJDIR)libcore.rlib Makefile
	@echo [RUSTC] -o $@
	mkdir -p $(dir $@)
	@$(ENV) rustc -O $(RUSTFLAGS) --crate-name common -o $@ $< --dep-info $@.dep $(call LocalLib,core)

# - Locally compiled libcore, needs to not use SSE
$(OBJDIR)libcore.rlib: $(LIBCORESRC)lib.rs
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	@$(ENV) rustc -O $(RUSTFLAGS) --crate-name core -o $@ $< --dep-info $@.dep
	

-include $(OBJS:%=%.dep)

