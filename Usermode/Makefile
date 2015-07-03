# Tifflin userland makefile
#
#
-include ../common.mk

V ?= @

# List of root-level applications to build
APPS := loader init

# Build directories
# - Distribution output root
OUTDIR := .output/$(ARCH)/
# - Application/Binary output
BINDIR := $(OUTDIR)bin/
# - Library output
LIBDIR := $(OUTDIR)libs/
# - Intermediate objects
OBJDIR := .objs/$(ARCH)/

# Rust compilation flags
TARGET_SPEC := target-$(ARCH).json
RUSTFLAGS := --target=$(TARGET_SPEC) --cfg arch__$(ARCH) --cfg nightly
RUSTFLAGS += -O
RUSTFLAGS += -g
RUSTFLAGS += --cfg disable_float
#RUSTFLAGS += -C soft-float	# STOPGAP until SSE is supported
#RUSTFLAGS += -C no-vectorize-loops
#RUSTFLAGS += -C no-vectorize-slp
RUSTFLAGS += -L $(LIBDIR)

# - List non-file targets
.PHONY: all clean
# - Don't delete intermediate targets
.SECONDARY:
# - (debugging) Remove most default targets
.SUFFIXES:


# DEFAULT: Compile all applications
all: $(APPS:%=$(BINDIR)%)

# Delete all artifacts
clean:
	$(RM) -rf $(OUTDIR) $(OBJDIR)

LOADER_DEPS := $(patsubst %,$(LIBDIR)lib%.rlib,$(call fn_getdeps,loader/src/main.rs) std core)

# Most binary targets have simple templates, but loader and libcore are special
# loader: Requires an assembly stub
# > Linking is done in two steps so that --gc-sections works
$(BINDIR)loader: loader/link.ld $(OBJDIR)loader/start.ao $(OBJDIR)loader/src/main.o
	@mkdir -p $(dir $@)
	@echo [LD] $@
	@$(LD) -r --gc-sections --undefined image_header --entry start -o $(OBJDIR)loader.o $(OBJDIR)loader/start.ao $(OBJDIR)loader/src/main.o $(LOADER_DEPS)
	@$(LD) -o $(OBJDIR)loader.elf -T loader/link.ld $(OBJDIR)loader.o -Map $(OBJDIR)loader_map.txt
	@objcopy $(OBJDIR)loader.elf -O binary $@
	@objdump -CS $(OBJDIR)loader.elf > $(OBJDIR)loader.dsm

$(OBJDIR)loader/src/main.o: $(LOADER_DEPS)

# Libraries are compiled into OBJDIR to get dependencies, but are actually stored in LIBDIR
$(LIBDIR)lib%.rlib: $(OBJDIR)lib%.rlib
	@mkdir -p $(dir $@)
	@mv $(OBJDIR)lib$*.rlib $@
# libcore: Same code as used by the kernel, but different build
$(OBJDIR)libcore.rlib: ../libcore/lib.rs $(TARGET_SPEC) Makefile
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	$V$(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(OBJDIR) $<
	@mv $(OBJDIR)core.d $(OBJDIR)libcore.d

$(BINDIR)%: %/src/main.rs $(TARGET_SPEC) Makefile link-$(ARCH).ld $(OBJDIR)rustrt0.o
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	$V$(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(OBJDIR) $< --crate-type=bin --crate-name=$* --verbose
	@mv $(OBJDIR)$* $@

# - local library crates
$(OBJDIR)lib%.rlib: lib%/src/lib.rs $(TARGET_SPEC) Makefile
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	$V$(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(OBJDIR) $< --crate-type=rlib --crate-name=$*
	@mv $(OBJDIR)$*.d $(OBJDIR)lib$*.d

# - crates.io crates
$(OBJDIR)lib%.rlib: ../externals/crates.io/%/src/lib.rs $(TARGET_SPEC) Makefile $(LIBDIR)libstd.rlib
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	$V$(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(OBJDIR) $< --crate-type=rlib --crate-name=$*
	@mv $(OBJDIR)$*.d $(OBJDIR)lib$*.d

../externals/crates.io/%/src/lib.rs: ../externals/crates.io/%.repo
	@mkdir -p ../externals/crates.io/$*
	test -e $@ || git clone `cat ../externals/crates.io/$*.repo` ../externals/crates.io/$*


$(OBJDIR)%.o: %.rs $(TARGET_SPEC) Makefile
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	@$(RUSTC) $(RUSTFLAGS) -o $@ $< --emit=obj,dep-info
-include $(OBJDIR)loader/src/main.d

$(OBJDIR)%.ao: %.S Makefile
	@mkdir -p $(dir $@)
	@echo [AS] $@
	$V$(CC) -o $@ -c $< -D ARCH_$(ARCH)
$(OBJDIR)rustrt0.o: rustrt0/$(ARCH).S Makefile
	@mkdir -p $(dir $@)
	@echo [AS] $@
	$V$(CC) -o $@ -c $< -D ARCH_$(ARCH)

# Generate dependencies for EVERY crate
fn_getstd = $(shell grep -F "![no_std]" $1 >/dev/null || echo "std")
fn_mmdep = $2: $(patsubst %,$(LIBDIR)lib%.rlib,$(call fn_getdeps,$1) $(call fn_getstd,$1))
#$(foreach m, $(wildcard */src/lib.rs */src/main.rs), $(warning $(call fn_mmdep,$m)))
#$(foreach m, $(wildcard */src/lib.rs */src/main.rs), $(eval $(call fn_mmdep,$m)))
# - Assumes that if there's a main.rs file, a binary is generated, and for lib.rs a rlib is generated
$(foreach m, $(wildcard */src/lib.rs), $(eval $(call fn_mmdep,$m,$(m:%/src/lib.rs=$(OBJDIR)%.rlib))))
$(foreach m, $(wildcard */src/lib.rs), $(warning $(call fn_mmdep,$m,$(m:%/src/lib.rs=$(OBJDIR)%.rlib))))
$(foreach m, $(wildcard */src/main.rs), $(eval $(call fn_mmdep,$m,$(m:%/src/main.rs=$(BINDIR)%))))
$(foreach m, $(wildcard */src/main.rs), $(warning $(call fn_mmdep,$m,$(m:%/src/main.rs=$(BINDIR)%))))

ALL_USED_LIBS := std $(sort $(foreach m, $(wildcard */src/lib.rs) $(wildcard */src/main.rs), $(call fn_getdeps,$m)))
ALL_PRESENT_BINS := $(patsubst %/src/main.rs,%, $(wildcard */src/main.rs))

DEPFILES := $(patsubst %,$(OBJDIR)lib%.d, $(ALL_USED_LIBS))
DEPFILES += $(patsubst %,$(OBJDIR)%.d, $(ALL_PRESENT_BINS))
$(warning $(DEPFILES))
-include $(DEPFILES)


