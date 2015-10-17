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
MODS += virtio
MODS += storage_ata
MODS += input_ps2
MODS += fs_fat fs_iso9660
#MODS += video_vga

ifeq ($(ARCH),amd64)
USE_ACPICA ?= 1
else
USE_ACPICA = 0
endif

fn_getdeps = $(shell cat $1 | sed -nr 's/.*extern crate ([a-zA-Z_0-9]+);/\1/p' | tr '\n' ' ')
fn_moddeps = $(filter-out core kernel,$(call fn_getdeps,Modules/$1/lib.rs))
uniq = $(eval seen :=) $(foreach _,$1,$(if $(filter $_,${seen}),,$(eval seen += $_))) ${seen}

OBJS := start.ao $(MODS:%=%.o) libkernel.rlib libcore.rlib
EOBJS := $(foreach m,$(MODS), $(addsuffix .o,$(call fn_moddeps,$m)) )
OBJS += $(EOBJS)
$(warning $(OBJS))
OBJS := $(shell echo $(call uniq, $(OBJS)))
ifeq ($(OBJS),)
	$(error BUGCHECK - Objects list empty)
endif

PREFIX ?= ../.prefix/
rustcmd = LD_LIBRARY_PATH=$(PREFIX)lib/ $(PREFIX)bin/$1
RUSTC := $(call rustcmd,rustc)
RUSTDOC := $(call rustcmd,rustdoc)
RUSTDOC := $(call rustcmd,cargo)

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

RUSTFLAGS := -L $(OBJDIR)
RUSTFLAGS += -O
#RUSTFLAGS += -C opt-level=1
RUSTFLAGS += -g
RUSTFLAGS += -C code-model=kernel
RUSTFLAGS += -C relocation-model=static
RUSTFLAGS += -C soft-float	# Disables requiring SSE etc
RUSTFLAGS += -C target-feature=$(subst $(space),$(comma),$(EXCLUDE_FEATURES:%=-%))
#RUSTFLAGS += -C no-vectorize-loops
#RUSTFLAGS += -C no-vectorize-slp
RUSTFLAGS += -Z no-landing-pads
RUSTFLAGS += --cfg 'arch="$(ARCH)"'
RUSTFLAGS += -L bin/plugins/
RUSTFLAGS += --cfg nightly --cfg no_std
RUSTFLAGS += --cfg disable_float
#RUSTFLAGS += -C save-temps
ifeq ($(USE_ACPICA),1)
	RUSTFLAGS += --cfg use_acpica
endif
RUSTFLAGS += --target=$(TARGET_FILE)
#RUSTFLAGS += -Z time-passes

LINKFLAGS := -T Core/arch/$(ARCH)/link.ld -Map $(OBJDIR)map.txt -g
LINKFLAGS += -z max-page-size=0x1000	# Reduces binary size
LINKFLAGS += --gc-sections	# Prevents complaining about floating point methods
#LINKFLAGS += --print-gc-sections

ENV := RUST_VERSION="$(shell $(RUSTC) --version)"
ENV += TK_GITSPEC="$(shell git log -n 1 | head -n 1 | awk '{print $$2}')"
ENV += TK_VERSION="0.1"
ENV += TK_BUILD="$(shell hostname --fqdn):$(BUILDNUM)"

LIBCORESRC := ../libcore/

# - External crates
EXT_CRATES := va_list stack_dst

# - Exclude non-module crates from the object list
OBJS := $(filter-out $(EXT_CRATES:%=%.o), $(OBJS))
OBJS := $(OBJS:%=$(OBJDIR)%)
LIBS := $(EXT_CRATES:%=$(OBJDIR)lib%.rlib)
ifeq ($(USE_ACPICA),1)
	LIBS += $(OBJDIR)acpica.a
endif

.SUFFIXES:
.PHONY: all clean docs run dsm

all: bin/kernel-$(ARCH).bin

dsm: bin/kernel-$(ARCH).bin.dsm

clean:
	rm -r $(OBJDIR)

docs:
	@mkdir -p doc/Kernel
	$V$(ENV) $(RUSTDOC) -o doc/Kernel Core/main.rs --cfg 'arch="amd64"' -L bin/plugins
	@chmod +rX doc/ -R 

run: bin/kernel-$(ARCH).bin
ifeq ($(ARCH),amd64)
	cd rundir && ./RunQemuPXE ../$<.elf32 "SYSDISK=ATA-0p0 SYSROOT=Tifflin"
else ifeq ($(ARCH),armv7)
	cd rundir && qemu-system-arm -machine realview-pb-a8 -serial stdio -s -no-reboot -kernel ../$< -append "SYSDISK=ATA-0p0 SYSROOT=Tifflin"
endif

# ===
# Toolchain maintainence
# - Downloads rustc and libcore
# ===
.PHONY: UPDATE UPDATE_rustc UPDATE_libcore_src
UPDATE: UPDATE_rustc UPDATE_libcore_src
UPDATE_rustc: 
	@mkdir -p ../.prefix
	curl -s https://static.rust-lang.org/rustup.sh | sh -s -- --channel=nightly --prefix=../.prefix --disable-sudo --disable-ldconfig --yes
UPDATE_libcore_src:
	cd .. && wget http://static.rust-lang.org/dist/rustc-nightly-src.tar.gz -N -nv

#../.prefix/bin/rustc: UPDATE_rustc
#	@#
../rustc-nightly-src.tar.gz:
	@echo -- nightly source required, run "make UPDATE"
	@false
../libcore/lib.rs: ../rustc-nightly-src.tar.gz ../libcore_nofp.patch
	tar -xmf ../rustc-nightly-src.tar.gz -C ../ rustc-nightly/src/libcore --transform 's~^rustc-nightly/src/~~'
	cd .. && patch -p0 < libcore_nofp.patch

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
	$V$(LD) -o $@ $(OBJS) $(LIBS) $(LINKFLAGS)
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

# - Kernel Core
$(OBJDIR)libkernel.rlib: Core/main.rs $(OBJDIR)libcore.rlib $(TARGET_FILE) ../Graphics/logo.rs bin/plugins/libtag_safe.so $(LIBS)
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(dir $@) $<
	@$(call FixDep, $(@:lib%.rlib=%.d))
	@cp $(OBJDIR)kernel.d $(OBJDIR)libkernel.d


# - Locally compiled libcore, needs to not use SSE
$(OBJDIR)libcore.rlib: $(LIBCORESRC)lib.rs $(TARGET_FILE)
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(dir $@) --crate-name core $<
	@$(call FixDep, $(@:lib%.rlib=%.d))
	@cp $(OBJDIR)core.d $(OBJDIR)libcore.d

$(OBJDIR)acpica.a: ../acpica/Makefile $(wildcard ../acpica/patches/source/include/platform/*)
	make -C ../acpica TRIPLE=$(TRIPLE) ARCH=$(ARCH) srcdir
	make -C ../acpica TRIPLE=$(TRIPLE) ARCH=$(ARCH)
	cp ../acpica/acpica-$(TRIPLE).a $@

# - Modules
$(OBJDIR)lib%.rlib: Modules/%/lib.rs $(OBJDIR)libcore.rlib $(OBJDIR)libkernel.rlib
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) $(RUSTFLAGS) --emit=obj,link,dep-info --out-dir $(dir $@) --crate-name=$* --crate-type=lib $< $(call LocalLib,core) $(call LocalLib,kernel)
	@$(call FixDep, $(@:%.o=%.d))

$(OBJDIR)%.o: Modules/%/lib.rs $(OBJDIR)lib%.rlib
	@#

fn_modline = $(OBJDIR)lib$1.rlib: $(patsubst %,$(OBJDIR)lib%.rlib,$(call fn_moddeps,$1))
$(foreach m,$(MODS), $(eval $(call fn_modline,$m) ) )

# - External libs
$(OBJDIR)lib%.rlib: ../externals/crates.io/%/src/lib.rs $(OBJDIR)libcore.rlib
	@echo [RUSTC] -o $@ $<
	$V$(ENV) $(RUSTC) $(RUSTFLAGS) -o $@ $<

.PRECIOUS: %.rs
../externals/crates.io/%/src/lib.rs: ../externals/crates.io/%.repo
	$(eval D=../externals/crates.io/$*)
	git clone `cat $<` $(D)

# - Plugins
bin/plugins/lib%.so: SynExts/%/src/lib.rs $(PREFIX)bin/rustc
	mkdir -p $(dir $@)
	$(RUSTC) -o $@ $<
SynExts/tag_safe/src/lib.rs:
	@mkdir -p SynExts
	cd SynExts && git clone https://github.com/thepowersgang/tag_safe

../Graphics/logo.rs: ../Graphics/TifflinLogoV1-128.png ../Graphics/ConvertTo32bppRS
	cd ../Graphics && ./ConvertTo32bppRS TifflinLogoV1-128.png logo.rs S_LOGO

-include $(OBJDIR)libkernel.d
-include $(OBJDIR)libcore.d
-include $(filter %.d, $(patsubst %.o,%.d,$(OBJS)))



