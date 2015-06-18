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
TARGET_SPEC := target-$(ARCH).json
RUSTFLAGS := --target=$(TARGET_SPEC) --cfg arch__$(ARCH)
RUSTFLAGS += -O
RUSTFLAGS += -g
RUSTFLAGS += -C soft-float	# STOPGAP until SSE is supported
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
$(BINDIR)loader: loader/link.ld $(OBJDIR)loader/start.ao $(OBJDIR)loader/src/main.o
	@mkdir -p $(dir $@)
	@echo [LD] $@
	@$(LD) -r --gc-sections --undefined image_header --entry start -o $(OBJDIR)loader.o $(OBJDIR)loader/start.ao $(OBJDIR)loader/src/main.o $(LIBDIR)libstd.rlib $(LIBDIR)libcore.rlib $(LIBDIR)libtifflin_syscalls.rlib
	@$(LD) -o $@ -T loader/link.ld $(OBJDIR)loader.o -Map $(OBJDIR)loader_map.txt
	@$(LD) -o $(OBJDIR)loader.elf -T loader/link.ld --oformat=elf64-x86-64 $(OBJDIR)loader.o -Map $(OBJDIR)loader_map.txt

$(OBJDIR)loader/src/main.o: $(LIBDIR)libstd.rlib

# libcore: Same code as used by the kernel, but different build
$(LIBDIR)libcore.rlib: ../libcore/lib.rs $(TARGET_SPEC) Makefile
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	@$(RUSTC) $(RUSTFLAGS) -o $@ $<

$(BINDIR)%: %/src/main.rs $(TARGET_SPEC) Makefile
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	@$(RUSTC) $(RUSTFLAGS) -o $@ $< --crate-type=bin --crate-name=$*

$(LIBDIR)lib%.rlib: lib%/src/lib.rs $(TARGET_SPEC) Makefile
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	@$(RUSTC) $(RUSTFLAGS) -o $@ $< --crate-type=rlib --crate-name=$*

$(OBJDIR)%.o: %.rs $(TARGET_SPEC) Makefile
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	@$(RUSTC) $(RUSTFLAGS) -o $@ $< --emit=obj

$(OBJDIR)%.ao: %.S Makefile
	@mkdir -p $(dir $@)
	@echo [AS] $@
	@$(CC) -o $@ -c $< -D ARCH_$(ARCH)

# Generate dependencies for EVERY crate
fn_getstd = $(shell grep -F "![no_std]" $1 >/dev/null || echo "std")
fn_mmdep = $2: $(patsubst %,$(LIBDIR)lib%.rlib,$(call fn_getdeps,$1) $(call fn_getstd,$1))
#$(foreach m, $(wildcard */src/lib.rs */src/main.rs), $(warning $(call fn_mmdep,$m)))
#$(foreach m, $(wildcard */src/lib.rs */src/main.rs), $(eval $(call fn_mmdep,$m)))
# - Assumes that if there's a main.rs file, a binary is generated, and for lib.rs a rlib is generated
$(foreach m, $(wildcard */src/lib.rs), $(eval $(call fn_mmdep,$m,$(m:%/src/lib.rs=$(LIBDIR)%.rlib))))
$(foreach m, $(wildcard */src/main.rs), $(eval $(call fn_mmdep,$m,$(m:%/src/main.rs=$(BINDIR)%))))

