# Tifflin userland makefile
#
#
-include ../common.mk

# List of root-level applications to build
APPS := loader

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
RUSTFLAGS := --target=target-$(ARCH).json --cfg arch__$(ARCH)
RUSTFLAGS += -O
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


# Most binary targets have simple templates, but loader and libcore are special
# loader: Requires an assembly stub
# > Linking is done in two steps so that --gc-sections works
$(BINDIR)loader: .objs/$(ARCH)/loader/start.ao .objs/$(ARCH)/loader/src/main.o
	@mkdir -p $(dir $@)
	@echo [LD] $@
	@$(LD) -r --gc-sections --undefined image_header --entry start -o $(OBJDIR)loader.o $^ $(LIBDIR)libstd.rlib $(LIBDIR)libcore.rlib $(LIBDIR)libtifflin_syscalls.rlib
	@$(LD) -o $@ -T loader/link.ld $(OBJDIR)loader.o -Map map.txt

# libcore: Same code as used by the kernel, but different build
$(LIBDIR)libcore.rlib: ../libcore/lib.rs
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	@$(RUSTC) $(RUSTFLAGS) -o $@ $<

$(BINDIR)%: %/src/main.rs $(LIBDIR)libcore.rlib $(LIBDIR)libstd.rlib
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	@$(RUSTC) $(RUSTFLAGS) -o $@ $< --crate-type=bin --crate-name=$*
$(LIBDIR)lib%.rlib: lib%/src/lib.rs $(LIBDIR)libcore.rlib $(LIBDIR)libstd.rlib
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	@$(RUSTC) $(RUSTFLAGS) -o $@ $< --crate-type=rlib --crate-name=$*

# libstd: NOT the standard rust one! This is a custom job until the real libstd can be built
$(OBJDIR)%.o: %.rs $(LIBDIR)libcore.rlib $(LIBDIR)libstd.rlib
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	@$(RUSTC) $(RUSTFLAGS) -o $@ $< --emit=obj

$(OBJDIR)%.ao: %.S
	@mkdir -p $(dir $@)
	@echo [AS] $@
	@$(CC) -o $@ -c $< -D ARCH_$(ARCH)

# Generate dependencies for EVERY crate
fn_mmdep = $1: $(patsubst %,$(LIBDIR)lib%.rlib,$(call fn_getdeps,$1))
#$(foreach m, $(wildcard */src/lib.rs */src/main.rs), $(warning $(call fn_mmdep,$m)))
$(foreach m, $(wildcard */src/lib.rs */src/main.rs), $(eval $(call fn_mmdep,$m)))

