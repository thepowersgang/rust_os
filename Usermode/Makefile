# Tifflin userland makefile
#
#
-include ../common.mk

V ?= @

ifeq ($(OS), Windows_NT)
 ifeq ($(ARCH),native)
  EXESUF := .exe
 endif
endif
EXESUF ?= 

# List of root-level applications to build
APPS := loader init login
APPS += handle_server
APPS += simple_console shell
APPS += filebrowser fileviewer
APPS += vfs_test
APPS += hello_world

APPS := $(addsuffix $(EXESUF),$(APPS))

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
CARGO_FLAGS ?=
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
ifneq ($(ARCH),native)
 CARGO_OUTDIR := target/target-$(ARCH)/$(CARGO_PROFILE)/
else
 CARGO_OUTDIR := target/$(CARGO_PROFILE)/
endif
ifeq ($V,)
 CARGO_FLAGS += -v
endif

ifeq ($(ARCH),riscv64)
 ASFLAGS += -mabi=lp64
endif

# - List non-file targets
.PHONY: all clean docs xargo
# - Don't delete intermediate targets
.SECONDARY:
# - (debugging) Remove most default targets
.SUFFIXES:

#makefile_deps = Makefile
makefile_deps = 

# DEFAULT: Compile all applications
all: $(APPS:%=$(BINDIR)%)
$(BINDIR)handle_server$(EXESUF): xargo
	@mkdir -p $(dir $@)
	$V$(STRIP) $(CARGO_OUTDIR)handle_server-bin -o $@
$(BINDIR)%: xargo
	@mkdir -p $(dir $@)
	$V$(STRIP) $(CARGO_OUTDIR)$* -o $@

# Delete all artifacts
clean:
	$(RM) -rf $(OUTDIR) $(OBJDIR) .docs target

docs:
	$(CARGO) -Z build-std=core,alloc doc $(CARGO_FLAGS)

ifeq ($(OS) $(ARCH),Windows_NT native)
  LIBLOADER_BIN := $(LIBDIR)loader_dyn.dll
else
  LIBLOADER_BIN := $(LIBDIR)libloader_dyn.so
endif

# Most binary targets are handled directly by `xargo`, but the loader is special
# > Requires an assembly stub
# > Linking is done in two steps so that --gc-sections works fully
# > The main body of the code is in `libloader_binary.a`, build by xargo
ifneq ($(ARCH),native)
LOADER_OBJS := $(OBJDIR)rustrt0.o $(OBJDIR)loader/bin/start.ao $(abspath $(CARGO_OUTDIR)libloader_binary.a)
$(BINDIR)loader: loader/bin/link.ld $(LOADER_OBJS)
	@mkdir -p $(dir $@)
	@echo [LD] $@
	$V$(LD) -r --gc-sections --undefined image_header --entry loader_start -o $(OBJDIR)loader.o  $(LOADER_OBJS)
	$V$(LD) -o $(OBJDIR)loader.elf -T loader/bin/link.ld $(OBJDIR)loader.o -Map $(OBJDIR)loader_map.txt
	$V$(OBJCOPY) $(OBJDIR)loader.elf -O binary $@
	$V$(OBJDUMP) -CS $(OBJDIR)loader.elf > $(OBJDIR)loader.dsm
# xargo portion of loader
$(abspath $(CARGO_OUTDIR)libloader_binary.a): xargo
	#
else
$(BINDIR)loader$(EXESUF): xargo
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
xargo: $(OBJDIR)rustrt0.o $(LIBLOADER_BIN)	# Referenced by link-$(TARGET).ld
	RUSTFLAGS='$(RUSTFLAGS)' $(CARGO) build $(CARGO_FLAGS)
#	$(foreach app,$(APPS), cp $(CARGO_OUTDIR)$(app) $(BINDIR)$(app);)

# Ensure that all external crates are checked out
xargo: $(patsubst %,../externals/crates.io/%/src/lib.rs,cmdline_words_parser stack_dst utf16_literal va_list)

ifneq ($(ARCH),native)
$(LIBDIR)libloader_dyn.so: $(makefile_deps) Makefile
	@mkdir -p $(dir $@)
	$V$(CARGO) run --manifest-path ../Helpers/make_elf_stub/Cargo.toml -- -a $(ARCH) -o $@ new_process start_process
else
$(LIBLOADER_BIN): $(makefile_deps) loader/bin/placeholder_lib.rs Makefile
	@mkdir -p $(dir $@)
	$V$(RUSTC) loader/bin/placeholder_lib.rs -o $@ --crate-type cdylib --crate-name loader -C panic=abort --cfg arch=\"native\"
 ifeq ($(OS),Windows_NT)
	cp $@.lib $(@:%.dll=%.lib)
 endif
endif

# - Assembly objects (loader)
$(OBJDIR)%.ao: %.S $(makefile_deps)
	@mkdir -p $(dir $@)
	@echo [AS] $@
	$V$(CC) $(ASFLAGS) -o $@ -c $< -D ARCH_$(ARCH) -MMD -MF $@.d
# - Base runtime
$(OBJDIR)rustrt0.o: rustrt0/$(ARCH).S $(makefile_deps)
	@mkdir -p $(dir $@)
	@echo [AS] $@
	$V$(CC) $(ASFLAGS) -o $@ -c $< -D ARCH_$(ARCH) -MMD -MF $@.d

DEPFILES := 
DEPFILES += $(OBJDIR)loader/bin/start.ao.d $(OBJDIR)rustrt0.o.d
#DEPFILES += $(CARGO_OUTDIR)libloader_binary.d
-include $(DEPFILES)

# Cargo emits `build.rs` as a dependency for everything
#.PHONY: build.rs
