#
# ARMv8 (aarch64) bootloader makefile
#
V ?= @

-include ../../common.mk

PLATFORM ?= virt

OBJDIR := .obj/$(PLATFORM)/
OBJ_MAIN := target/target/release/libmain.a
KERNEL := ../../Kernel/bin/kernel-$(ARCH).bin
TARGET := ../../Kernel/Core/arch/$(ARCH)/target.json

.PHONY: all clean

all: loader-$(PLATFORM).bin

clean:
	rm -r loader-$(PLATFORM).bin $(OBJDIR)start.ao $(dir $(OBJ_MAIN))

loader-$(PLATFORM).bin: $(OBJDIR)start.ao $(OBJ_MAIN) $(OBJDIR)kernel_blob.o $(KERNEL)
	@echo "[LD] -o $@"
	@$(TRIPLE)-ld -g -e start -o $@ $(OBJDIR)start.ao $(OBJ_MAIN) $(OBJDIR)kernel_blob.o -Map $(OBJDIR)map.txt --gc-sections -z defs
	@$(TRIPLE)-objdump -S $@ > $@.dsm

$(OBJDIR)kernel_blob.o: $(KERNEL)
	@cp $< kernel.bin
#	@$(TRIPLE)-strip kernel.bin
	@$(TRIPLE)-objcopy -I binary -O elf64-littleaarch64 -B aarch64 kernel.bin $@
	@rm kernel.bin

$(OBJDIR)start.ao: start.S platform-$(PLATFORM).h ../../Usermode/rustrt0/armv8-helpers.S
	@echo "[AS] -o $@"
	@mkdir -p $(dir $@)
	@$(TRIPLE)-cpp -o $@.S $< -include platform-$(PLATFORM).h
	@$(TRIPLE)-as $@.S -o $@

$(OBJ_MAIN): main.rs $(TARGET)
	@echo "[CARGO] -o $@"
	@mkdir -p $(dir $@)
	$V$(CARGO) build -Z build-std=core --target $(TARGET) --release 

%.dtb: %.dts
	@echo "[DTC] -o $@"
	@dtc $< -O dtb -o $@

-include $(OBJ_MAIN).d
