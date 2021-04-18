#
# ARMv7- (aarch32) bootloader makefile
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
OBJ_MAIN := target/target/release/libmain.a
KERNEL := ../../Kernel/bin/kernel-$(ARCH).bin
TARGET := ../../Kernel/Core/arch/$(ARCH)/target.json

FDT_FILE :=
ifneq ($(wildcard fdt-$(PLATFORM).dts),)
 FDT_FILE := fdt-$(PLATFORM).dtb
endif

.PHONY: all clean

all: loader-$(PLATFORM).bin

clean:
	rm -r loader-$(PLATFORM).bin $(OBJDIR)start-$(PLATFORM).ao $(dir $(OBJ_MAIN))

loader-$(PLATFORM).bin: $(OBJDIR)start.ao $(OBJ_MAIN) $(KERNEL) $(FDT_FILE)
	@echo "[LD] -o $@"
	$Varm-elf-eabi-ld -e start -o $@ $(OBJDIR)start.ao $(OBJ_MAIN) --format binary $(KERNEL) $(FDT_FILE) -Map $(OBJDIR)map.txt --gc-sections

# Assembly source for bootloader
$(OBJDIR)start.ao: start-$(PLATFORM).S platform-$(PLATFORM).h
	@echo "[AS] -o $@"
	@mkdir -p $(dir $@)
	$Varm-elf-eabi-cpp -o $@.S $< -include platform-$(PLATFORM).h
	$Varm-elf-eabi-as $@.S -o $@
# Rust source for bootloader
$(OBJ_MAIN): main.rs $(TARGET)
	@echo "[CARGO] -o $@"
	@mkdir -p $(dir $@)
	$V$(CARGO) build -Z build-std=core --target $(TARGET) --release 

%.dtb: %.dts
	@echo "[DTC] -o $@"
	$Vdtc $< -O dtb -o $@

-include $(OBJ_MAIN).d
