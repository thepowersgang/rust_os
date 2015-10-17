# Tifflin userland makefile
#
#
-include ../common.mk

V ?= @

# List of root-level applications to build
APPS := loader init login
APPS += simple_console shell

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

RUSTFLAGS := $(RUST_COMMON_FLAGS)
RUSTFLAGS += -O
#RUSTFLAGS += -C llvm-args=-Os
RUSTFLAGS += -g
RUSTFLAGS += --cfg disable_float
RUSTFLAGS += -Z force-overflow-checks=yes
#RUSTFLAGS += -C lto
# - This feels bad, sure it's valid, but...
ifeq ($(ARCH),armv7)
  RUSTFLAGS += -C relocation-model=static
endif

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

# Delete all artifacts
clean:
	$(RM) -rf $(OUTDIR) $(OBJDIR) .docs

docs: $(patsubst lib%/,.docs/%/index.html,$(wildcard lib*/))

.docs/%/index.html: lib%/lib.rs $(LIBDIR)lib%.rlib
	@echo $@
	@$(RUSTDOC) -o .docs $< --crate-name $* $(RUST_COMMON_FLAGS)
.docs/%/index.html: lib%/src/lib.rs $(LIBDIR)lib%.rlib
	@echo $@
	@$(RUSTDOC) -o .docs $< --crate-name $* $(RUST_COMMON_FLAGS)

LOADER_DEPS := $(patsubst %,$(LIBDIR)lib%.rlib,$(call fn_getdeps,loader/src/main.rs) std std_io std_rt core)

# Most binary targets have simple templates, but loader and libcore are special
# loader: Requires an assembly stub
# > Linking is done in two steps so that --gc-sections works
$(BINDIR)loader: loader/link.ld $(OBJDIR)loader/start.ao $(OBJDIR)loader/src/main.o $(LOADER_DEPS)
	@mkdir -p $(dir $@)
	@echo [LD] $@
	@$(LD) -r --gc-sections --undefined image_header --entry start -o $(OBJDIR)loader.o $(OBJDIR)loader/start.ao $(OBJDIR)loader/src/main.o $(LOADER_DEPS)
	@$(LD) -o $(OBJDIR)loader.elf -T loader/link.ld $(OBJDIR)loader.o -Map $(OBJDIR)loader_map.txt
	@$(OBJCOPY) $(OBJDIR)loader.elf -O binary $@
	@$(OBJDUMP) -CS $(OBJDIR)loader.elf > $(OBJDIR)loader.dsm
$(OBJDIR)loader/src/main.o: $(LOADER_DEPS)

# Low-level loader dummy dynamic library
# - Used to tell ld that `loader`'s exported symbols actually exist
$(LIBDIR)libloader_dyn.so: loader/lib.S	$(makefile_deps)
	@mkdir -p $(dir $@)
	@echo [CC] $@
	$V$(CC) -o $@.o -c $< -fPIC -D ARCH_$(ARCH)
	$V$(LD) -o $@ $@.o -shared -soname=libloader_dyn.so

# Libraries are compiled into OBJDIR to get dependencies, but are actually stored in LIBDIR
$(LIBDIR)lib%.rlib: $(OBJDIR)lib%.rlib
	@mkdir -p $(dir $@)
	@cp $< $@
# libcore: Same code as used by the kernel, but different build
$(OBJDIR)libcore.rlib: ../rustc_src/libcore/lib.rs $(TARGET_SPEC) $(makefile_deps)
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	$V$(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(OBJDIR) $<
	@mv $(OBJDIR)core.d $(OBJDIR)libcore.d

# ===============
# Executables
# ===============
$(BINDIR)%: $(OBJDIR)%
	@mkdir -p $(dir $@)
	@cp $< $@
$(APPS:%=$(OBJDIR)%): $(OBJDIR)%: %/src/main.rs $(TARGET_SPEC) $(makefile_deps) link-$(ARCH).ld $(OBJDIR)rustrt0.o $(LIBDIR)libloader_dyn.so
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	$V$(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(OBJDIR) $< --crate-type=bin --crate-name=$* --verbose
	$V$(OBJDUMP) -S $@ > $@.dsm

# ===============
# Libraries
# ===============
# - Local libraries (with src dir)
$(OBJDIR)lib%.rlib: lib%/src/lib.rs $(TARGET_SPEC) $(makefile_deps)
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	$V$(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(OBJDIR) $< --crate-type=rlib --crate-name=$*
	@mv $(OBJDIR)$*.d $(OBJDIR)lib$*.d
# - Local libraries (with no src dir)
$(OBJDIR)lib%.rlib: lib%/lib.rs $(TARGET_SPEC) $(makefile_deps)
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	$V$(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(OBJDIR) $< --crate-type=rlib --crate-name=$*
	@mv $(OBJDIR)$*.d $(OBJDIR)lib$*.d

# - Official libraries
$(OBJDIR)lib%.rlib: ../rustc_src/lib%/lib.rs $(OBJDIR)libcore.rlib $(TARGET_SPEC) $(makefile_deps)
	@mkdir -p $(dir $@)
	@echo [RUSTC rust-src] $@
	$VCFG_COMPILER_HOST_TRIPLE=x86_64-unknown-linux-gnu $(RUSTC) $(RUSTFLAGS) -A unused_unsafe -A unused_features --emit=link,dep-info --out-dir $(OBJDIR) $< --crate-type=rlib --crate-name=$*
	@mv $(OBJDIR)$*.d $(OBJDIR)lib$*.d

# - Loader's interface library, lives in the loader directory
$(OBJDIR)libloader.rlib: loader/lib.rs $(TARGET_SPEC) $(makefile_deps) $(patsubst %,$(LIBDIR)lib%.rlib,$(call fn_getdeps,loader/lib.rs))
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	$V$(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(OBJDIR) $< --crate-type=rlib
	@mv $(OBJDIR)loader.d $(OBJDIR)liblib.d

# - crates.io crates
$(OBJDIR)lib%.rlib: ../externals/crates.io/%/src/lib.rs $(TARGET_SPEC) $(makefile_deps) $(LIBDIR)libstd.rlib
	@mkdir -p $(dir $@)
	@echo [RUSTC crates.io] $@
	$V$(RUSTC) $(RUSTFLAGS) --emit=link,dep-info --out-dir $(OBJDIR) $< --crate-type=rlib --crate-name=$*
	@mv $(OBJDIR)$*.d $(OBJDIR)lib$*.d
#  > Handle clone/update of the external repo when missing or rustc is updated
../externals/crates.io/%/src/lib.rs: ../externals/crates.io/%.repo
	$(eval DIR=../externals/crates.io/$*)
	@mkdir -p $(DIR)
	test -e $@ || git clone `cat ../externals/crates.io/$*.repo` $(DIR) && (cd $(DIR) && git pull)

# - General object files (mainly loader)
$(OBJDIR)%.o: %.rs $(TARGET_SPEC) $(makefile_deps)
	@mkdir -p $(dir $@)
	@echo [RUSTC] $@
	@$(RUSTC) $(RUSTFLAGS) $< --out-dir=$(dir $@) --emit=obj,dep-info
-include $(OBJDIR)loader/src/main.d

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

# Generate dependencies for EVERY crate
fn_getstd = $(shell grep -F "![no_std]" $1 >/dev/null && echo "core" || echo "std")
fn_mmdep = $2: $(filter-out $2,$(patsubst %,$(LIBDIR)lib%.rlib,$(call fn_getdeps,$1) $(call fn_getstd,$1)))
# - Assumes that if there's a main.rs file, a binary is generated, and for lib.rs a rlib is generated
$(foreach m, $(wildcard */lib.rs), $(eval $(call fn_mmdep,$m,$(m:%/lib.rs=$(OBJDIR)%.rlib))))
$(foreach m, $(wildcard */src/lib.rs), $(eval $(call fn_mmdep,$m,$(m:%/src/lib.rs=$(OBJDIR)%.rlib))))
$(foreach m, $(wildcard */src/main.rs), $(eval $(call fn_mmdep,$m,$(m:%/src/main.rs=$(OBJDIR)%))))

#$(foreach m, $(wildcard ../rustc_src/lib*/lib.rs), $(eval $(call fn_mmdep,$m,$(m:../rustc_src/%/lib.rs=$(OBJDIR)%.rlib))))
#$(foreach m, $(wildcard ../rustc_src/lib*/lib.rs), $(warning $(call fn_mmdep,$m,$(m:../rustc_src/%/lib.rs=$(OBJDIR)%.rlib))))

ALL_USED_LIBS := std $(sort $(foreach m, $(wildcard */src/lib.rs) $(wildcard */src/main.rs), $(call fn_getdeps,$m)))
ALL_PRESENT_BINS := $(patsubst %/src/main.rs,%, $(wildcard */src/main.rs))

DEPFILES := $(patsubst %,$(OBJDIR)lib%.d, $(ALL_USED_LIBS))
DEPFILES += $(patsubst %,$(OBJDIR)%.d, $(ALL_PRESENT_BINS))
DEPFILES += $(OBJDIR)loader/start.ao.d $(OBJDIR)rustrt0.o.d
-include $(DEPFILES)

# Official library dependency tree
fn_rustdeps = $(patsubst %,$(OBJDIR)lib%.rlib,$1): $(patsubst %,$(LIBDIR)lib%.rlib,$2)
$(eval $(call fn_rustdeps, collections, rustc_unicode alloc))



#%.rs:
#	
