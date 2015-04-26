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
ARCH = amd64
TRIPLE = x86_64-none-elf
MODS := storage_ata
#MODS += video_vga

USE_ACPICA ?= 1

OBJS := start.ao $(MODS:%=mod_%.o) libkernel.rlib libcore.rlib
ifeq ($(USE_ACPICA),1)
	OBJS += acpica.a
endif

PREFIX := ../.prefix/
RUSTC := LD_LIBRARY_PATH=$(PREFIX)lib/ $(PREFIX)bin/rustc
RUSTDOC := LD_LIBRARY_PATH=$(PREFIX)lib/ $(PREFIX)bin/rustdoc

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

EXCLUDE_FEATURES := 3dnow avx avx2 sse sse2 sse3 sse4.1 sse4.2 mmx
TARGET_FILE := Core/arch/$(ARCH)/target.json

OBJDIR := .obj/$(ARCH)/

RUSTFLAGS := -L $(OBJDIR)
RUSTFLAGS += -O
RUSTFLAGS += -g
RUSTFLAGS += -C code-model=kernel
RUSTFLAGS += -C soft-float	# Disables requiring SSE etc
RUSTFLAGS += -C target-feature=$(subst $(space),$(comma),$(EXCLUDE_FEATURES:%=-%))
RUSTFLAGS += --cfg arch__$(ARCH)
ifeq ($(USE_ACPICA),1)
	RUSTFLAGS += --cfg use_acpica
endif
RUSTFLAGS += --target=$(TARGET_FILE)
#RUSTFLAGS += -Z time-passes

LINKFLAGS := -T Core/arch/$(ARCH)/link.ld -Map $(OBJDIR)map.txt -g
LINKFLAGS += -z max-page-size=0x1000	# Reduces binary size
LINKFLAGS += --gc-sections	# Prevents complaining about floating point methods

ENV := RUST_VERSION="$(shell $(RUSTC) --version)"
ENV += TK_GITSPEC="$(shell git log -n 1 | head -n 1 | awk '{print $$2}')"
ENV += TK_VERSION="0.1"
ENV += TK_BUILD="$(shell hostname --fqdn):$(BUILDNUM)"

LIBCORESRC := ../libcore/

OBJS := $(OBJS:%=$(OBJDIR)%)

.PHONY: all clean docs run

all: bin/kernel-$(ARCH).bin bin/kernel-$(ARCH).bin.dsm

clean:
	rm -r $(OBJDIR)

docs:
	@mkdir -p doc/Kernel
	$V$(ENV) $(RUSTDOC) -o doc/Kernel Core/main.rs --cfg arch__amd64 
	@chmod +rX doc/ -R 

run: bin/kernel-$(ARCH).bin
	cd rundir && ./RunQemuPXE ../$<.elf32 ""

# ===
# Toolchain maintainence
# - Downloads rustc and libcore
# ===
.PHONY: UPDATE UPDATE_rustc UPDATE_libcore_src
UPDATE: UPDATE_rustc UPDATE_libcore_src
UPDATE_rustc: 
	@mkdir -p ../.prefix
	curl -s https://static.rust-lang.org/rustup.sh | sh -s -- --channel=nightly --prefix=../.prefix
UPDATE_libcore_src:
	cd .. && wget https://static.rust-lang.org/dist/rustc-nightly-src.tar.gz -N -nv

../.prefix/bin/rustc: UPDATE_rustc
	@#
../rustc-nightly-src.tar.gz:
	@echo -- nightly source required, run "make UPDATE"
	@false
../libcore/lib.rs: ../rustc-nightly-src.tar.gz
	tar -xmf ../rustc-nightly-src.tar.gz -C ../ rustc-nightly/src/libcore --transform 's~^rustc-nightly/src/~~'

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
	@mkdir -p bin/
	@echo [LD] -o $@
	@$(TRIPLE)-ld -o $@ $(OBJS) $(LINKFLAGS)
	@objcopy $@ -F elf32-i386 $@.elf32
	@echo "BUILDNUM := $$(( $(BUILDNUM) + 1 ))" > BuildNum.$(ARCH).mk

%.dsm: %
	@echo "[OBJDUMP] >" $*.dsm
	@objdump -C -S $* > $*.dsm

# Assembly
$(OBJDIR)%.ao: Core/arch/$(ARCH)/%.asm Makefile
	@echo [AS] -o $@
	@mkdir -p $(dir $@)
	@nasm -o $@ $< -f elf64 -MD $@.d -MP

# - Kernel Core
$(OBJDIR)libkernel.rlib: Core/main.rs $(OBJDIR)libcore.rlib Makefile $(TARGET_FILE) ../Graphics/logo.rs
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) $(RUSTFLAGS) --emit=link,dep-info -o $@ $< $(call LocalLib,core)
	@$(call FixDep, $(@:%.rlib=%.d))


# - Locally compiled libcore, needs to not use SSE
$(OBJDIR)libcore.rlib: $(LIBCORESRC)lib.rs $(TARGET_FILE)
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) $(RUSTFLAGS) --emit=link,dep-info -o $@ --crate-name core $<
	@$(call FixDep, $(@:%.rlib=%.d))

$(OBJDIR)acpica.a: ../acpica/Makefile
	TRIPLE=$(TRIPLE) make -C ../acpica
	cp ../acpica/acpica-$(TRIPLE).a $@

# - Modules
$(OBJDIR)mod_%.o: Modules/%/lib.rs $(OBJDIR)libcore.rlib $(OBJDIR)libkernel.rlib
	@echo [RUSTC] -o $@
	@mkdir -p $(dir $@)
	$V$(ENV) $(RUSTC) $(RUSTFLAGS) --emit=obj,dep-info -o $@ --crate-name=$* --crate-type=lib $< $(call LocalLib,core) $(call LocalLib,kernel)
	@$(call FixDep, $(@:%.o=%.d))

../Graphics/logo.rs: ../Graphics/TifflinLogoV1-128.png ../Graphics/ConvertTo32bppRS
	cd ../Graphics && ./ConvertTo32bppRS TifflinLogoV1-128.png logo.rs S_LOGO

-include $(OBJDIR)libkernel.d
-include $(OBJDIR)libcore.d
-include $(filter %.d, $(patsubst %.o,%.d,$(OBJS)))



