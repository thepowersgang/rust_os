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

# - Environment variable read by cargo
RUSTFLAGS :=
RUSTFLAGS += --cfg arch="$(ARCH)"
RUSTFLAGS += -L $(LIBDIR)

CARGO_PROFILE := release
CARGO_FLAGS :=
ifneq ($(ARCH),native)
  CARGO_FLAGS += --target $(TARGET_SPEC)
  CARGO_FLAGS += -Z build-std=core,alloc
else
  CARGO_PROFILE := debug
endif
ifeq ($(CARGO_PROFILE),debug)
  # No --release
else ifeq ($(CARGO_PROFILE),release)
  CARGO_FLAGS += --release
else
  $(error Unknown xargo profile $(CARGO_PROFILE))
endif
CARGO_OUTDIR := target/target-$(ARCH)/$(CARGO_PROFILE)/
ifeq ($(ARCH),native)
CARGO_OUTDIR := target/$(CARGO_PROFILE)/
endif

# - List non-file targets
.PHONY: all clean docs
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
	@if test -e $(CARGO_OUTDIR)$*; then cp $(CARGO_OUTDIR)$* $@; else cp $(CARGO_OUTDIR)$*-bin $@; fi

# Delete all artifacts
clean:
	$(RM) -rf $(OUTDIR) $(OBJDIR) .docs target

docs:
	$(CARGO) -Z build-std=core,alloc doc $(CARGO_FLAGS)

# Most binary targets are handled directly by `xargo`, but the loader is special
# > Requires an assembly stub
# > Linking is done in two steps so that --gc-sections works fully
# > The main body of the code is in `libloader_binary.a`, build by xargo
ifneq ($(ARCH),native)
LOADER_OBJS := $(OBJDIR)rustrt0.o $(OBJDIR)loader/bin/start.ao $(CARGO_OUTDIR)libloader_binary.a
$(BINDIR)loader: loader/bin/link.ld $(LOADER_OBJS)
	@mkdir -p $(dir $@)
	@echo [LD] $@
	$V$(LD) -r --gc-sections --undefined image_header --entry start -o $(OBJDIR)loader.o  $(LOADER_OBJS)
	$V$(LD) -o $(OBJDIR)loader.elf -T loader/bin/link.ld $(OBJDIR)loader.o -Map $(OBJDIR)loader_map.txt
	$V$(OBJCOPY) $(OBJDIR)loader.elf -O binary $@
	$V$(OBJDUMP) -CS $(OBJDIR)loader.elf > $(OBJDIR)loader.dsm
# xargo portion of loader
$(CARGO_OUTDIR)libloader_binary.a: xargo
else
$(BINDIR)loader: xargo
	@mkdir -p $(dir $@)
	RUSTFLAGS='$(RUSTFLAGS)' $(CARGO) build $(CARGO_FLAGS) --manifest-path loader/native/Cargo.toml
ifeq ($(OS),Windows_NT)
	cp loader/native/$(CARGO_OUTDIR)loader_dyn.dll $(dir $@)
else
	cp loader/native/$(CARGO_OUTDIR)libloader_dyn.so $(dir $@)
endif
	touch $@
endif

.PHONY: xargo
xargo: $(OBJDIR)rustrt0.o $(LIBDIR)libloader_dyn.so	# Referenced by link-$(TARGET).ld
	RUSTFLAGS='$(RUSTFLAGS)' $(CARGO) build $(CARGO_FLAGS)

# Ensure that all external crates are checked out
xargo: $(patsubst %,../externals/crates.io/%/src/lib.rs,cmdline_words_parser stack_dst utf16_literal va_list)

# Low-level loader dummy dynamic library
# - Used to tell ld that `loader`'s exported symbols actually exist
$(LIBDIR)libloader_dyn.so: loader/bin/placeholder_lib.rs	$(makefile_deps) Makefile
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	$V$(RUSTC) --crate-type=cdylib --crate-name loader_dyn loader/bin/placeholder_lib.rs --out-dir $(LIBDIR) --cfg 'arch="$(ARCH)"'
	if [ -e $(LIBDIR)loader_dyn.dll.lib ]; then cp $(LIBDIR)loader_dyn.dll.lib $(LIBDIR)loader_dyn.lib; fi

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
