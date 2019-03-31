# "Tifflin" Kernel
# - By John Hodge (thePowersGang)
#
# Makefile
# - Kernel core makefile
#
# TODO:
# - Module support
# - Multiple architecture configurations
#
# NOTE:
# - This downloads its own copy of rustc (to ../.prefix)
# - It also downloads+extracts libcore from the nightly tarball
-include ../common.mk

MODS := syscalls
MODS += network
MODS += virtio
MODS += storage_ata
MODS += input_ps2
MODS += fs_fat fs_iso9660 fs_extN
MODS += storage_ahci
MODS += nic_rtl8139
ifeq ($(ARCH),amd64)
#MODS += video_vga
endif
MODS += usb_core usb_ohci

ifeq ($(ARCH),amd64)
USE_ACPICA ?= 1
else
USE_ACPICA = 0
endif

# - External crates
EXT_CRATES := stack_dst
ifeq ($(ARCH),amd64)
EXT_CRATES += va_list
endif

fn_getdeps = $(shell cat $1 | sed -nr 's/.*extern crate ([a-zA-Z_0-9]+);/\1/p' | tr '\n' ' ')
fn_moddeps = $(filter-out core kernel,$(call fn_getdeps,Modules/$1/lib.rs))
uniq = $(eval seen :=) $(foreach _,$1,$(if $(filter $_,${seen}),,$(eval seen += $_))) ${seen}

OBJS := start.ao log_cfg.ao main.o $(MODS:%=lib%.rlib.o) libkernel.rlib.o
EOBJS := $(foreach m,$(MODS), $(patsubst %,lib%.rlib.o,$(call fn_moddeps,$m)) )
OBJS += $(EOBJS)
OBJS := $(shell echo $(call uniq, $(OBJS)))
ifeq ($(OBJS),)
	$(error BUGCHECK - Objects list empty)
endif

-include BuildNum.$(ARCH).mk
BUILDNUM ?= 1

V ?= @

nop :=
space := $(nop) $(nop)
comma := ,

# Function: Reference a locally-compile crate
LocalLib = --extern $1=$(OBJDIR)lib$1.rlib
# Function: Fix the specified dependency file (from http://scottmcpeak.com/autodepend/autodepend.html)
FixDep = cp -f $1 $1.tmp && sed -e 's/.*://' -e 's/\\$$//' < $1.tmp | head -n 1 | fmt -1 | sed -e 's/^ *//' -e 's/$$/:/' >> $1 && rm -f $1.tmp

EXCLUDE_FEATURES :=
ifeq ($(ARCH),amd64)
EXCLUDE_FEATURES := 3dnow avx avx2 sse sse2 sse3 sse4.1 sse4.2 mmx
endif
TARGET_FILE := Core/arch/$(ARCH)/target.json

OBJDIR := .obj/$(ARCH)/

RUSTFLAGS_C := --target=$(TARGET_FILE) --cfg 'arch="$(ARCH)"'
RUSTFLAGS_C += -L $(OBJDIR)
RUSTFLAGS_C += -L bin/plugins/
RUSTFLAGS_C += --cfg nightly --cfg no_std --cfg 'feature="no_std"'
RUSTFLAGS_C += --cfg disable_float
ifeq ($(USE_ACPICA),1)
	RUSTFLAGS_C += --cfg use_acpica
endif

RUSTFLAGS := $(RUSTFLAGS_C)
RUSTFLAGS += -O
#RUSTFLAGS += -C opt-level=1
RUSTFLAGS += -g
ifeq ($(ARCH),amd64)
RUSTFLAGS += -C code-model=kernel
endif
RUSTFLAGS += -C relocation-model=static
RUSTFLAGS += -C soft-float	# Disables requiring SSE etc
RUSTFLAGS += -C target-feature=$(subst $(space),$(comma),$(EXCLUDE_FEATURES:%=-%))
#RUSTFLAGS += -C no-vectorize-loops
#RUSTFLAGS += -C no-vectorize-slp
RUSTFLAGS += -Z no-landing-pads
#RUSTFLAGS += -C save-temps
#RUSTFLAGS += -Z time-passes
# - DISABLED: Causes undefined symbol errors
#RUSTFLAGS += -Zincremental=$(OBJDIR)incremental/

LINKFLAGS := -T Core/arch/$(ARCH)/link.ld -Map $(OBJDIR)map.txt -g
LINKFLAGS += -z max-page-size=0x1000	# Reduces binary size
LINKFLAGS += --gc-sections	# Prevents complaining about floating point methods
#LINKFLAGS += --print-gc-sections

ENV := RUST_VERSION="$(shell $(RUSTC) --version)"
ENV += TK_GITSPEC="$(shell git log -n 1 | head -n 1 | awk '{print $$2}')"
ENV += TK_VERSION="0.1"
ENV += TK_BUILD="$(shell hostname --fqdn):$(BUILDNUM)"

LIBCORESRC := ../libcore/
IMGDIR := ../Graphics/.output/shared/

# - Exclude non-module crates from the object list
OBJS := $(filter-out $(EXT_CRATES:%=%.o), $(OBJS))
OBJS := $(OBJS:%=$(OBJDIR)%)
LIBS := $(OBJDIR)libcore.rlib $(OBJDIR)libcompiler_builtins.rlib $(EXT_CRATES:%=$(OBJDIR)lib%.rlib)
ifeq ($(USE_ACPICA),1)
	LIBS += $(OBJDIR)libacpica.a
endif

.SUFFIXES:
.PHONY: all clean docs dsm doctest test

all: bin/kernel-$(ARCH).bin

test: run-libkernel-test run-libnetwork-test
.PHONY: run-%-test
.PRECIOUS: $(OBJDIR)libnetwork-test
run-%-test: $(OBJDIR)%-test
	$<

dsm: bin/kernel-$(ARCH).bin.dsm

clean:
	rm -r $(OBJDIR)

docs: $(patsubst %,doc/$(ARCH)/%/index.html,core kernel)

doc/$(ARCH)/%/index.html:
	@mkdir -p doc/$(ARCH)
	@echo [RUSTDOC] $<
	$V$(ENV) $(RUSTDOC) $(RUSTFLAGS_C) -o doc/$(ARCH) $<
doc/$(ARCH)/kernel/index.html: Core/main.rs $(OBJDIR)libkernel.rlib
doc/$(ARCH)/core/index.html: ../libcore/lib.rs $(OBJDIR)libcore.rlib


# ===
# Toolchain maintainence
# - Downloads rustc and libcore
# ===
.PHONY: UPDATE
UPDATE:
	@mkdir -p ../.prefix
	curl https://static.rust-lang.org/rustup/rustup-init.sh -sSf | RUSTUP_HOME=$(abspath ../.prefix) CARGO_HOME=$(abspath ../.prefix) sh -s -- --default-toolchain none --no-modify-path -y
	$(call fn_rustcmd,rustup) update $(RUSTUP_VER)
	$(call fn_rustcmd,rustup) default $(RUSTUP_VER)
	#$(call fn_rustcmd,rustup) component add rust-src
	curl $(LIBCORE_URL)
	test -f ../.prefix/bin/xargo || $(CARGO) install xargo

../libcore/lib.rs: ../libcore_nofp.patch $(RUSTUP_SRC_DIR)libcore/lib.rs
	rm -rf ../libcore
	cp -r $(RUSTUP_SRC_DIR)libcore ../libcore
	cp -r $(RUSTUP_SRC_DIR)stdsimd ../stdsimd
	#cd .. && patch -p0 < libcore_nofp.patch
.PHONY: NEWPATCH
NEWPATCH:
	rm -rf ../libcore_orig
	cp -r $(RUSTUP_SRC_DIR)libcore ../libcore_orig
	cd .. && (diff -rub libcore_orig/ libcore/; true) > libcore_nofp.patch
	

# ---
# Debug
# ---
.PHONY: pretty
pretty:
	$V$(ENV) $(RUSTC) -Z unstable-options --pretty typed $(RUSTFLAGS) -o bin/kernel-$(ARCH).exp_rs Core/main.rs $(call LocalLib,core)
$(OBJDIR)kernel.ir: Core/main.rs $(OBJDIR)libcore.rlib $(TARGET_FILE)
	$V$(ENV) $(RUSTC) $(RUSTFLAGS) --emit=llvm-ir -o $@ $< $(call LocalLib,core)


# ===
# Complilation
# ===
bin/kernel-$(ARCH).bin: $(OBJS) Core/arch/$(ARCH)/link.ld Makefile
	@mkdir -p bin/ $(OBJDIR)
	@echo [LD] -o $@
	$V$(LD) -o $@ --whole-archive $(OBJS) --no-whole-archive $(LIBS) $(LINKFLAGS)
ifeq ($(ARCH),amd64)
	@objcopy $@ --only-keep-debug $@.debug
	@objcopy $@ -F elf32-i386 $@.elf32
	@$(TRIPLE)-strip -g $@.elf32
else
	#@$(TRIPLE)-strip $@
endif
	@echo "BUILDNUM := $$(( $(BUILDNUM) + 1 ))" > BuildNum.$(ARCH).mk

%.dsm: %
	@echo "[OBJDUMP] >" $*.dsm
	$V$(OBJDUMP) -s -C -S $* > $*.dsm

# Assembly
$(OBJDIR)%.ao: Core/arch/$(ARCH)/%.asm
	@echo [AS] -o $@
	@mkdir -p $(dir $@)
	$Vnasm -o $@ $< -f elf64 -MD $@.d -MP
$(OBJDIR)%.ao: Core/arch/$(ARCH)/%.S
	@echo [AS] -o $@
	@mkdir -p $(dir $@)
	$V$(CC) -o $@ -c $<
$(OBJDIR)log_cfg.ao: Core/log_cfg.S
	@echo [AS] -o $@
	@mkdir -p $(dir $@)
	$V$(CC) -o $@ -c $<

$(OBJDIR)main.o: main/main.rs $(OBJDIR)libkernel.rlib $(OBJDIR)libcore.rlib $(OBJDIR)libsyscalls.rlib
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) --emit=obj,dep-info --out-dir $(dir $@) $(RUSTFLAGS) $<

# - Kernel Core
$(OBJDIR)libkernel.rlib: Core/main.rs $(OBJDIR)libcore.rlib $(TARGET_FILE) ../Graphics/logo.rs $(IMGDIR)panic.rs bin/plugins/libtag_safe.so $(LIBS)
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@) $(OBJDIR)incremental/
	$V$(ENV) $(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(dir $@) $<
	@cp $(OBJDIR)kernel.d $(OBJDIR)libkernel.d
$(OBJDIR)libkernel-test: Core/main.rs $(OBJDIR)test/libstack_dst.rlib $(OBJDIR)libkernel.rlib
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) --test -o $@ $< -L bin/plugins/ -L $(OBJDIR)test/
$(OBJDIR)test/libkernel.rlib: Core/main.rs $(OBJDIR)libkernel.rlib bin/plugins/libtag_safe.so $(EXT_CRATES:%=$(OBJDIR)test/lib%.rlib)
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) --out-dir $(dir $@) --emit=link,dep-info $< --crate-type rlib --crate-name kernel --cfg test_shim -L bin/plugins -L $(OBJDIR)test


# - Locally compiled libcore, needs to not use SSE
$(OBJDIR)libcore.rlib: $(LIBCORESRC)lib.rs $(TARGET_FILE)
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(dir $@) --crate-name core --crate-type=lib $<
	@cp $(OBJDIR)core.d $(OBJDIR)libcore.d
$(OBJDIR)libcompiler_builtins.rlib: $(RUSTUP_SRC_DIR)libcompiler_builtins/src/lib.rs $(TARGET_FILE)
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(dir $@) --cfg feature=\"compiler-builtins\" $< --cfg stage0
	@cp $(OBJDIR)core.d $(OBJDIR)libcore.d

$(OBJDIR)libacpica.a: ../acpica/Makefile $(wildcard ../acpica/patches/source/include/platform/*)
	make -C ../acpica TRIPLE=$(TRIPLE) ARCH=$(ARCH) srcdir
	make -C ../acpica TRIPLE=$(TRIPLE) ARCH=$(ARCH)
	cp ../acpica/acpica-$(TRIPLE).a $@

# - Modules
$(OBJDIR)lib%.rlib: Modules/%/lib.rs $(OBJDIR)libcore.rlib $(OBJDIR)libkernel.rlib
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(dir $@) --crate-name=$* --crate-type=lib $< $(call LocalLib,core) $(call LocalLib,kernel)
$(OBJDIR)test/lib%.rlib: Modules/%/lib.rs $(OBJDIR)lib%.rlib $(OBJDIR)test/libkernel.rlib
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) --out-dir $(dir $@) --crate-name=$* --crate-type=lib $< -L $(OBJDIR)test/
$(OBJDIR)lib%.rlib.o: $(OBJDIR)lib%.rlib
	$Vcp $< $@.tmp
	$Var d $@.tmp $$(ar t $< | grep -v '\.o$$')
	$Vmv $@.tmp $@
$(OBJDIR)lib%-test: Modules/%/lib.rs $(OBJDIR)test/lib%.rlib
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) --test -o $@ $< -L bin/plugins/ -L $(OBJDIR)test/

$(OBJDIR)%.o: Modules/%/lib.rs $(OBJDIR)lib%.rlib
	@#

fn_modline = $(OBJDIR)lib$1.rlib: $(patsubst %,$(OBJDIR)lib%.rlib,$(call fn_moddeps,$1))
fn_modline2 = $(OBJDIR)test/lib$1.rlib: $(patsubst %,$(OBJDIR)test/lib%.rlib,$(call fn_moddeps,$1))
$(foreach m,$(MODS), $(eval $(call fn_modline,$m) ) $(eval $(call fn_modline2,$m) ) )

# - External libs
$(OBJDIR)lib%.rlib: ../externals/crates.io/%/src/lib.rs $(OBJDIR)libcore.rlib
	@echo [RUSTC] -o $@ $<
	$V$(ENV) $(RUSTC) $(RUSTFLAGS) -o $@ $<
$(OBJDIR)test/lib%.rlib: ../externals/crates.io/%/src/lib.rs $(OBJDIR)libcore.rlib
	@echo [RUSTC] -o $@ $<
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) -o $@ $<

.PRECIOUS: %.rs
../externals/crates.io/%/src/lib.rs: ../externals/crates.io/%.repo
	$(eval D=../externals/crates.io/$*)
	git clone `cat $<` $(D)

# - Plugins
bin/plugins/lib%.so: SynExts/%/src/lib.rs $(PREFIX)bin/rustc
	mkdir -p $(dir $@)
	cd SynExts/$* ; $(CARGO) build --release
	cp SynExts/$*/target/release/lib*.so $@
SynExts/tag_safe/src/lib.rs:
	@mkdir -p SynExts
	cd SynExts && git clone https://github.com/thepowersgang/tag_safe

../Graphics/logo.rs: ../Graphics/TifflinLogoV1-128.png ../Graphics/ConvertTo32bppRS
	cd ../Graphics && ./ConvertTo32bppRS TifflinLogoV1-128.png logo.rs S_LOGO
$(IMGDIR)%.rs: ../Graphics/ConvertTo32bppRS
	cd ../Graphics && make $(@:../Graphics/%=%)

-include $(OBJDIR)libkernel.d
-include $(OBJDIR)test/libkernel.d
-include $(OBJDIR)libcore.d
-include $(filter %.d, $(patsubst %.o,%.d,$(OBJS)))
-include $(filter %.d, $(patsubst %.ao,%.ao.d,$(OBJS)))
-include $(filter %.d, $(patsubst $(OBJDIR)lib%.rlib.o,$(OBJDIR)%.d,$(OBJS)))

