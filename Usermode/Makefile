# Tifflin userland makefile
#
#
-include ../common.mk

V ?= @

# List of root-level applications to build
APPS := loader init login
APPS += handle_server
APPS += simple_console shell
APPS += filebrowser fileviewer
APPS += vfs_test
APPS += hello_world

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

RUST_COMMON_FLAGS := --target=$(TARGET_SPEC) --cfg 'arch="$(ARCH)"' --cfg nightly
RUST_COMMON_FLAGS += -L $(LIBDIR)

XARGO := RUST_TARGET_PATH=$(dir $(abspath $(TARGET_SPEC))) $(XARGO)
XARGO_TARGET := target-$(ARCH)
XARGO_PROFILE := release
XARGO_FLAGS := --target $(XARGO_TARGET)
ifeq ($(XARGO_PROFILE),debug)
else ifeq ($(XARGO_PROFILE),release)
  XARGO_FLAGS += --release
else
  $(error Unknown xargo profile $(XARGO_PROFILE))
endif
XARGO_OUTDIR := target/$(XARGO_TARGET)/$(XARGO_PROFILE)/

RUSTFLAGS := $(RUST_COMMON_FLAGS)
RUSTFLAGS += -O
#RUSTFLAGS += -C llvm-args=-Os
RUSTFLAGS += -g
RUSTFLAGS += --cfg disable_float
RUSTFLAGS += -Z force-overflow-checks=yes
RUSTFLAGS += -Z no-landing-pads
# - This feels bad, sure it's valid, but...
ifeq ($(ARCH),armv7)
  RUSTFLAGS += -C relocation-model=static
endif
BIN_RUSTFLAGS := 
#BIN_RUSTFLAGS += -C lto

RUSTFLAGS_compiler_builtins = --cfg feature=\"compiler-builtins\" --cfg stage0
RUSTFLAGS_alloc = --edition 2018

# - List non-file targets
.PHONY: all clean
# - Don't delete intermediate targets
.SECONDARY:
# - (debugging) Remove most default targets
.SUFFIXES:

#makefile_deps = Makefile
makefile_deps = 


# DEFAULT: Compile all applications
all: $(APPS:%=$(BINDIR)%)
$(BINDIR)%: xargo
	@mkdir -p $(dir $@)
	@if test -e $(XARGO_OUTDIR)$*; then cp $(XARGO_OUTDIR)$* $@; else cp $(XARGO_OUTDIR)$*-bin $@; fi

# Delete all artifacts
clean:
	$(RM) -rf $(OUTDIR) $(OBJDIR) .docs target

docs: $(patsubst lib%/,.docs/%/index.html,$(wildcard lib*/))

.docs/%/index.html: lib%/lib.rs $(LIBDIR)lib%.rlib
	@echo [RUSTDOC] -o $@
	$V$(RUSTDOC) -o .docs $< --crate-name $* $(RUST_COMMON_FLAGS)
.docs/%/index.html: lib%/src/lib.rs $(LIBDIR)lib%.rlib
	@echo "[RUSTDOC] -o $@"
	$V$(RUSTDOC) -o .docs $< --crate-name $* $(RUST_COMMON_FLAGS)

# Most binary targets are handled directly by `xargo`, but the loader is special
# > Requires an assembly stub
# > Linking is done in two steps so that --gc-sections works fully
# > The main body of the code is in `libloader_binary.a`, build by xargo
LOADER_OBJS := $(OBJDIR)rustrt0.o $(OBJDIR)loader/bin/start.ao $(XARGO_OUTDIR)libloader_binary.a
$(BINDIR)loader: loader/bin/link.ld $(LOADER_OBJS)
	@mkdir -p $(dir $@)
	@echo [LD] $@
	$V$(LD) -r --gc-sections --undefined image_header --entry start -o $(OBJDIR)loader.o  $(LOADER_OBJS)
	$V$(LD) -o $(OBJDIR)loader.elf -T loader/bin/link.ld $(OBJDIR)loader.o -Map $(OBJDIR)loader_map.txt
	$V$(OBJCOPY) $(OBJDIR)loader.elf -O binary $@
	$V$(OBJDUMP) -CS $(OBJDIR)loader.elf > $(OBJDIR)loader.dsm
# xargo portion of loader
$(XARGO_OUTDIR)libloader_binary.a: xargo

.PHONY: xargo
xargo: $(OBJDIR)rustrt0.o $(LIBDIR)libloader_dyn.so	# Referenced by link-$(TARGET).ld
	$(XARGO) build $(XARGO_FLAGS)
# Ensure that all external crates are checked out
xargo: $(patsubst %,../externals/crates.io/%/src/lib.rs,cmdline_words_parser stack_dst utf16_literal va_list)
# Low-level loader dummy dynamic library
# - Used to tell ld that `loader`'s exported symbols actually exist
$(LIBDIR)libloader_dyn.so: loader/bin/lib.S	$(makefile_deps)
	@mkdir -p $(dir $@)
	@echo [CC] $@
	$V$(CC) -o $@.o -c $< -fPIC -D ARCH_$(ARCH)
	$V$(LD) -o $@ $@.o -shared -soname=libloader_dyn.so

# - Assembly objects (loader)
$(OBJDIR)%.ao: %.S $(makefile_deps)
	@mkdir -p $(dir $@)
	@echo [AS] $@
	$V$(CC) -o $@ -c $< -D ARCH_$(ARCH) -MMD -MF $@.d
# - Base runtime
$(OBJDIR)rustrt0.o: rustrt0/$(ARCH).S $(makefile_deps)
	@mkdir -p $(dir $@)
	@echo [AS] $@
	$V$(CC) -o $@ -c $< -D ARCH_$(ARCH) -fPIC -MMD -MF $@.d

DEPFILES := 
DEPFILES += $(OBJDIR)loader/bin/start.ao.d $(OBJDIR)rustrt0.o.d
-include $(DEPFILES)
