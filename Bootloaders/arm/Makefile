
-include ../../common.mk

PLATFORM ?= virt

ifeq ($(PLATFORM),bcm2708)
  ARCH=armv6
else
  ARCH=armv7
endif

LIBCORE := ../../Kernel/.obj/$(ARCH)/libcore.rlib
OBJDIR := .obj/$(PLATFORM)/
KERNEL := ../../Kernel/bin/kernel-$(ARCH).bin
TARGET := ../../Kernel/Core/arch/$(ARCH)/target.json

all: loader-$(PLATFORM).bin

loader-$(PLATFORM).bin: $(OBJDIR)start-$(PLATFORM).ao $(OBJDIR)main.o $(KERNEL)
	@echo "[LD] -o $@"
	$Varm-elf-eabi-ld -e start -o $@ $(OBJDIR)start-$(PLATFORM).ao $(OBJDIR)main.o $(LIBCORE) --format binary $(KERNEL) -Map $(OBJDIR)map.txt --gc-sections

$(OBJDIR)start-$(PLATFORM).ao: start-$(PLATFORM).S platform-$(PLATFORM).h
	@echo "[AS] -o $@"
	@mkdir -p $(dir $@)
	$Varm-elf-eabi-cpp -o $@.S $< -include platform-$(PLATFORM).h
	$Varm-elf-eabi-as $@.S -o $@

$(OBJDIR)main.o: main.rs $(TARGET) $(LIBCORE)
	@echo "[RUSTC] -o $@"
	@mkdir -p $(dir $@)
	@mkdir -p Core/arch/armv7; cp $(TARGET) Core/arch/armv7/target.json
	$V$(RUSTC) --target Core/arch/armv7/target.json $< -o $@ --extern core=$(LIBCORE) -Z no-landing-pads --emit obj --crate-type lib -O

%.dtb: %.dts
	@echo "[DTC] -o $@"
	$Vdtc $< -O dtb -o $@
