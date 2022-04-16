#
#
#
V ?= @
-include ../common.mk


# ---------------------------------------

# - ACPICA can only be used on x86_64/amd64
ifeq ($(ARCH),amd64)
 USE_ACPICA ?= 1
else
 USE_ACPICA = 0
endif

ifeq ($(ARCH),riscv64)
 ASFLAGS += -mabi=lp64 -march=rv64gc
endif

-include BuildNum.$(ARCH).mk
BUILDNUM ?= 1

OBJDIR := .obj/$(ARCH)/
OBJS := $(addprefix $(OBJDIR),start.ao log_cfg.ao libmain.a build_info.ao)
IMGDIR := ../Graphics/.output/shared/

CARGO_FEATURES :=
# Allow caller to add extra cargo flags (e.g. --offline)
CARGO_FLAGS ?=
# - V is empty (i.e. it's not '@', then tell cargo to be verbose)
ifeq ($(V),)
CARGO_FLAGS += -v
endif

LINKFLAGS := -T Core/arch/$(ARCH)/link.ld -Map $(OBJDIR)map.txt -g
LINKFLAGS += -z max-page-size=0x1000	# Reduces binary size
LINKFLAGS += --gc-sections	# Prevents complaining about floating point methods
#LINKFLAGS += --print-gc-sections
#LINKFLAGS += --verbose

EXCLUDE_FEATURES :=

ifeq ($(USE_ACPICA),1)
  OBJS += $(OBJDIR)libacpica.a
  CARGO_FEATURES += acpica
endif


.PHONY: all dsm PHONY

all: bin/kernel-$(ARCH).bin
dsm: bin/kernel-$(ARCH).bin.dsm

clean:
	rm -rf bin/kernel-$(ARCH).bin* .obj/$(ARCH)
	$(CARGO) clean --target=targets/kernel-$(ARCH).json --release


# 
# Final linking stage
# 
# TODO: Need to get the library list from cargo
bin/kernel-$(ARCH).bin: $(OBJS) Core/arch/$(ARCH)/link.ld
	@mkdir -p bin/
	@echo [LD] -o $@
	$V$(LD) -o $@ --whole-archive $(OBJS) --no-whole-archive $(LINKFLAGS)
ifeq ($(ARCH),amd64)
	$Vobjcopy $@ --only-keep-debug $@.debug
	$Vobjcopy $@ -F elf32-i386 $@.elf32
	$V$(TRIPLE)-strip -g $@.elf32
else
	#$V$(TRIPLE)-strip $@
endif
	@echo "BUILDNUM := $$(( $(BUILDNUM) + 1 ))" > BuildNum.$(ARCH).mk

#
# Disassembly
#
%.dsm: %
	@echo "[OBJDUMP] >" $*.dsm
	$V$(OBJDUMP) -s -C -S $* > $*.dsm

#
# Invoke xargo to build the main part of the kernel
# - This includes all modules enabled in ./Cargo.toml
#
$(OBJDIR)libmain.a: PHONY targets/kernel-$(ARCH).json $(IMGDIR)logo.rs $(IMGDIR)panic.rs
	@echo "[CARGO] . > $@"
	$V$(CARGO) build -Z build-std=core,alloc --target=targets/kernel-$(ARCH).json --release --features "$(CARGO_FEATURES)" -j 1 $(CARGO_FLAGS)
	@mkdir -p $(dir $@)
	@if test ! -e $@ || test target/kernel-$(ARCH)/release/libmain.a -nt $@; then cp target/kernel-$(ARCH)/release/libmain.a $@; fi
$(patsubst %,$(OBJDIR)lib%.a,$(MODS)): $(OBJDIR)libmain.a

targets/kernel-$(ARCH).json: Core/arch/$(ARCH)/target.json
	@mkdir -p $(dir $@)
	$Vcp $< $@

# Build information (should be rebuilt if anything changes)
$(OBJDIR)build_info.ao: Core/build_info.S $(filter-out $(OBJDIR)build_info.ao, $(OBJS)) Makefile
	@echo [AS] -o $@
	@mkdir -p $(dir $@)
	$V$(CC) $(ASFLAGS) -o $@ -c $< \
		-D 'TK_BUILD="$(shell hostname --fqdn):$(BUILDNUM)"' \
		-D 'RUST_VERSION="$(shell $(RUSTC) --version)"' \
		-D 'TK_GITSPEC="$(shell git log -n 1 | head -n 1 | awk '{print $$2}')"' \
		-D 'TK_VERSION="0.1"'

# ACPICA
$(OBJDIR)libacpica.a: ../acpica/Makefile $(wildcard ../acpica/patches/source/include/platform/*)
	make -C ../acpica TRIPLE=$(TRIPLE) ARCH=$(ARCH) srcdir
	make -C ../acpica TRIPLE=$(TRIPLE) ARCH=$(ARCH)
	cp ../acpica/acpica-$(TRIPLE).a $@

$(OBJDIR)%.ao: Core/arch/$(ARCH)/%.asm
	@echo [AS] -o $@
	@mkdir -p $(dir $@)
	$Vnasm -o $@ $< -f elf64 -MD $@.d -MP -F dwarf -g
$(OBJDIR)%.ao: Core/arch/$(ARCH)/%.S
	@echo [AS] -o $@
	@mkdir -p $(dir $@)
	$V$(CC) $(ASFLAGS) -o $@ -c $< -Wp,-MMD,$@.d -Wp,-MT,$@ -Wp,-MP
# Log configuration
$(OBJDIR)log_cfg.ao: Core/log_cfg.S
	@echo [AS] -o $@
	@mkdir -p $(dir $@)
	$V$(CC) $(ASFLAGS) -o $@ -c $<

$(IMGDIR)%.rs: ../Graphics/Makefile
	cd ../Graphics && make $(@:../Graphics/%=%)

-include $(patsubst %,%.d,$(OBJS))
