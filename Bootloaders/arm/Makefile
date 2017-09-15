
-include ../../common.mk

PLATFORM ?= virt

LIBCORE := ../../Kernel/.obj/armv7/libcore.rlib
OBJDIR := .obj/$(PLATFORM)/
KERNEL := ../../Kernel/bin/kernel-armv7.bin
TARGET := ../../Kernel/Core/arch/armv7/target.json

all: loader-$(PLATFORM).bin

loader-$(PLATFORM).bin: $(OBJDIR)start.ao $(OBJDIR)main.o $(KERNEL)
	@echo "[LD] -o $@"
	$Varm-elf-eabi-ld -e start -o $@ $(OBJDIR)start.ao $(OBJDIR)main.o $(LIBCORE) --format binary $(KERNEL) -Map $(OBJDIR)map.txt --gc-sections

$(OBJDIR)start.ao: start.S platform-$(PLATFORM).h
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
