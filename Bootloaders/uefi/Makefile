
-include ../../common.mk

V ?= @

BIN := ../bootloader-efi.pe
TARGET_NAME := target-uefi
TARGET_FILE := ./$(TARGET_NAME).json
RUSTFLAGS := -Z no-landing-pads -L .obj/ -C opt-level=s
TARGET_DIR := $(abspath target/$(TARGET_NAME)/release)/

OBJS := $(TARGET_DIR)libbootloader_uefi.a .obj/libstubs.a

.PHONY: all clean empty

all: $(BIN)

clean:
	rm $(OBJS)
	$(XARGO) clean

.obj/bootloader-efi.o: $(OBJS)
	@echo [LD] -o $@
	$Vx86_64-efi-pe-ld -g -r -o $@ -e efi_main $(OBJS) --strip-discarded --gc-sections --allow-multiple-definition

$(TARGET_DIR)libbootloader_uefi.a: .obj/libstubs.a $(TARGET_FILE) empty
	@echo [XARGO] build
	$VRUSTFLAGS="$(RUSTFLAGS)" RUST_TARGET_PATH=$(abspath .) $(XARGO) build --target=$(TARGET_NAME) --release 

$(BIN): .obj/bootloader-efi.o
	@echo [LD] -o $@
	$Vx86_64-efi-pe-ld -dll -shared --subsystem 10 -e efi_main -o $@ .obj/bootloader-efi.o -Map map.txt --gc-sections 2>&1 | (grep -v 'has no section'; true)
	$Vx86_64-efi-pe-objdump -d $@ > $@.dsm
	$Vx86_64-efi-pe-strip $@

.obj/stubs.o: stubs.S
	@mkdir -p .obj/
	@echo [AS] -o $@
	$Vx86_64-efi-pe-as $< -o $@
.obj/libstubs.a: .obj/stubs.o
	$Vx86_64-efi-pe-ar ru $@ $<

-include $(TARGET_DIR)libbootloader_uefi.d

