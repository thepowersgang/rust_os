
-include ../../../common.mk

PLATFORM ?= virt

LIBCORE := ../../.obj/armv7/libcore.rlib
OBJDIR := .obj/$(PLATFORM)/
KERNEL := ../../bin/kernel-armv7.bin

loader-$(PLATFORM).bin: $(OBJDIR)start.ao $(OBJDIR)main.o $(KERNEL)
	arm-elf-eabi-ld -e start -o $@ $(OBJDIR)start.ao $(OBJDIR)main.o $(LIBCORE) --format binary $(KERNEL) -Map $(OBJDIR)map.txt --gc-sections

$(OBJDIR)start.ao: start.S platform-$(PLATFORM).h
	mkdir -p $(dir $@)
	arm-elf-eabi-cpp -o $@.S $< -include platform-$(PLATFORM).h
	arm-elf-eabi-as $@.S -o $@

$(OBJDIR)main.o: main.rs ../../Core/arch/armv7/target.json $(LIBCORE)
	mkdir -p $(dir $@)
	mkdir -p Core/arch/armv7; cp ../../Core/arch/armv7/target.json Core/arch/armv7/target.json
	$(RUSTC) --target Core/arch/armv7/target.json $< -o $@ --extern core=$(LIBCORE) -Z no-landing-pads --emit obj --crate-type lib

