#
# TODO: Switch to pure xargo (with `cargo rustc` to add the link args)
#
V ?= @

-include ../../common.mk

PLATFORM ?= virt

ifeq ($(PLATFORM),bcm2708)
  ARCH=armv6
else
  ARCH=armv7
endif

OBJDIR := .obj/$(PLATFORM)/
LIBCORE := $(OBJDIR)libcore.rlib
LIBCOMPILER_BUILTINS := $(OBJDIR)libcompiler_builtins.rlib
KERNEL := ../../Kernel/bin/kernel-$(ARCH).bin
TARGET := ../../Kernel/Core/arch/$(ARCH)/target.json

RUSTFLAGS := -O
RUSTFLAGS += -C panic=abort
RUSTFLAGS += --target $(TARGET)
RUSTFLAGS += -Z no-landing-pads

all: loader-$(PLATFORM).bin

clean:
	rm loader-$(PLATFORM).bin $(OBJDIR)start-$(PLATFORM).ao $(OBJDIR)main.o

loader-$(PLATFORM).bin: $(OBJDIR)start-$(PLATFORM).ao $(OBJDIR)main.o $(KERNEL)
	@echo "[LD] -o $@"
	$Varm-elf-eabi-ld -e start -o $@ $(OBJDIR)start-$(PLATFORM).ao $(OBJDIR)main.o $(LIBCORE) --format binary $(KERNEL) -Map $(OBJDIR)map.txt --gc-sections

# Assembly source for bootloader
$(OBJDIR)start-$(PLATFORM).ao: start-$(PLATFORM).S platform-$(PLATFORM).h
	@echo "[AS] -o $@"
	@mkdir -p $(dir $@)
	$Varm-elf-eabi-cpp -o $@.S $< -include platform-$(PLATFORM).h
	$Varm-elf-eabi-as $@.S -o $@
# Rust source for bootloader
$(OBJDIR)main.o: main.rs $(TARGET) $(LIBCORE) $(LIBCOMPILER_BUILTINS)
	@echo "[RUSTC] -o $@"
	@mkdir -p $(dir $@)
	$V$(RUSTC) $(RUSTFLAGS) $< -o $@ --extern core=$(LIBCORE) --extern compiler_builtins=$(LIBCOMPILER_BUILTINS) --emit obj --crate-type lib
# libcompiler_builtins (custom compile due to target path changes)
$(LIBCOMPILER_BUILTINS): $(RUSTUP_SRC_DIR)libcompiler_builtins/src/lib.rs $(TARGET) $(LIBCORE)
	@echo "[RUSTC] -o $@"
	@mkdir -p $(dir $@)
	$V$(RUSTC) $(RUSTFLAGS) $< -o $@ --extern core=$(LIBCORE) --crate-name=compiler_builtins --crate-type=lib --cfg feature=\"compiler-builtins\"
# libcore (custom compile due to target path changes)
$(LIBCORE): ../../libcore/lib.rs $(TARGET)
	@echo "[RUSTC] -o $@"
	@mkdir -p $(dir $@)
	$V$(RUSTC) $(RUSTFLAGS) $< -o $@ --crate-name=core --crate-type=lib

%.dtb: %.dts
	@echo "[DTC] -o $@"
	$Vdtc $< -O dtb -o $@
