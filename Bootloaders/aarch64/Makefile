
-include ../../common.mk

PLATFORM ?= virt

LIBCORE := ../../Kernel/.obj/armv8/libcore.rlib
OBJDIR := .obj/$(PLATFORM)/

all: loader-$(PLATFORM).bin

loader-$(PLATFORM).bin: $(OBJDIR)start.ao $(OBJDIR)main.o $(KERNEL)
	@echo "[LD] -o $@"
	@cp $(KERNEL) kernel.bin && $(TRIPLE)-objcopy -I binary -O elf64-littleaarch64 -B aarch64 kernel.bin $(OBJDIR)kernel_blob.o && rm kernel.bin
	@$(TRIPLE)-ld -g -e start -o $@ $(OBJDIR)start.ao $(OBJDIR)main.o $(OBJDIR)kernel_blob.o $(LIBCORE) -Map $(OBJDIR)map.txt --gc-sections -z defs
	@$(TRIPLE)-objdump -S $@ > $@.dsm

$(OBJDIR)start.ao: start.S platform-$(PLATFORM).h ../../Usermode/rustrt0/armv8-helpers.S
	@echo "[AS] -o $@"
	@mkdir -p $(dir $@)
	@$(TRIPLE)-cpp -o $@.S $< -include platform-$(PLATFORM).h
	@$(TRIPLE)-as $@.S -o $@

$(OBJDIR)main.o: main.rs ../../Kernel/Core/arch/armv8/target.json $(LIBCORE)
	@echo "[RUSTC] -o $@"
	@mkdir -p $(dir $@)
	@mkdir -p Core/arch/armv8; cp ../../Kernel/Core/arch/armv8/target.json Core/arch/armv8/target.json
	@$(RUSTC) --target Core/arch/armv8/target.json $< -o $@ --extern core=$(LIBCORE) -Z no-landing-pads --emit obj --crate-type lib -O -g

%.dtb: %.dts
	@echo "[DTC] -o $@"
	@dtc $< -O dtb -o $@
