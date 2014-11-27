#
#
#
ARCH = amd64
TRIPLE = x86_64-none-elf
OBJS := start.ao kcore.o libcore.rlib

-include BuildNum.$(ARCH).mk
BUILDNUM ?= 1


V ?= @

nop :=
space := $(nop) $(nop)
comma := ,

# Function: Reference a locally-compile crate
LocalLib = --extern $1=$(OBJDIR)lib$1.rlib

EXCLUDE_FEATURES := 3dnow avx avx2 sse sse2 sse3 sse4.1 sse4.2 mmx
TARGET_FILE := arch/$(ARCH)/target.json

RUSTFLAGS := --crate-type lib
RUSTFLAGS += -O
#RUSTFLAGS += --debuginfo 1
#RUSTFLAGS += -C code-model=kernel
#RUSTFLAGS += -C soft-float
RUSTFLAGS += -C target-feature=$(subst $(space),$(comma),$(EXCLUDE_FEATURES:%=-%))
RUSTFLAGS += --cfg arch__$(ARCH)
#RUSTFLAGS += -Z time-passes
RUSTFLAGS += --target=$(TARGET_FILE)

LINKFLAGS := -z max-page-size=0x1000 -g
LINKFLAGS += -T arch/$(ARCH)/link.ld -Map $(OBJDIR)map.txt
LINKFLAGS += --gc-sections	# Prevents complaining about floating point methods

ENV := RUST_VERSION="$(shell rustc --version)"
ENV += TK_GITSPEC="$(shell git log -n 1 | head -n 1 | awk '{print $$2}')"
ENV += TK_VERSION="0.1"
ENV += TK_BUILD="$(shell hostname --fqdn):$(BUILDNUM)"

LIBDIR := /home/tpg/apps/lib/rustlib/x86_64-unknown-linux-gnu/lib/
PREFIX := $(shell pwd)/.prefix/$(OBJDIR)/
RUSTSRC := /home/tpg/Source/rust/rust-nightly/
LIBCORESRC := $(RUSTSRC)src/libcore/
LIBGCC := /home/tpg/apps/lib/gcc/$(TRIPLE)/4.7.1/libgcc.a

RUSTC := $(PREFIX)bin/rustc

OBJDIR := .obj/$(ARCH)/
OBJS := $(OBJS:%=$(OBJDIR)%)

all: kernel-$(ARCH).bin

clean:
	rm -r $(OBJDIR)

pretty:
	$V$(ENV) rustc --pretty expanded $(RUSTFLAGS) -o kernel-$(ARCH).exp_rs Core/main.rs $(call LocalLib,core)

run: kernel-$(ARCH).bin
	./RunQemuPXE $<.elf32 ""

kernel-$(ARCH).bin: $(OBJS) arch/$(ARCH)/link.ld Makefile
	@echo [LD] -o $@
	@$(TRIPLE)-ld -o $@ $(OBJS) $(LIBCORE) $(LIBGCC) $(LINKFLAGS)
	@objcopy $@ -F elf32-i386 $@.elf32
	#@mcopy -i fdd.img $@.elf32 ::/ -D o
	@objdump -S $@ > $@.dsm
	@echo "BUILDNUM := $$(( $(BUILDNUM) + 1 ))" > BuildNum.$(ARCH).mk

$(OBJDIR)%.ao: arch/$(ARCH)/%.asm Makefile
	@echo [AS] -o $@
	@mkdir -p $(dir $@)
	@nasm -o $@ $< -f elf64 -MD $@.dep -MP

$(OBJDIR)kcore.o: Core/main.rs $(OBJDIR)libcore.rlib Makefile $(TARGET_FILE)
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) rustc $(RUSTFLAGS) --emit obj -o $@ $< --dep-info $@.dep $(call LocalLib,core)

# - Locally compiled libcore, needs to not use SSE
$(OBJDIR)libcore.rlib: $(LIBCORESRC)lib.rs $(TARGET_FILE)
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) rustc $(RUSTFLAGS) --crate-name core -o $@ $< --dep-info $@.dep

-include $(OBJS:%=%.dep)

