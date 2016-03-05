
-include ../../common.mk

BIN := ../bootloader-efi.pe
TARGET_FILE := target.json
RUSTFLAGS := --target=$(TARGET_FILE) --out-dir=.obj/ -Z no-landing-pads -L .obj/ -C lto

all: $(BIN)

.obj/bootloader-efi.o: .obj/main.o .obj/core.o .obj/uefi.o Makefile
	x86_64-efi-pe-ld -r -o $@ -e efi_main .obj/main.o .obj/core.o .obj/uefi.o --strip-discarded --gc-sections --allow-multiple-definition
$(BIN): .obj/bootloader-efi.o
	x86_64-efi-pe-ld -dll -shared --subsystem 10 -e efi_main -o $@ .obj/bootloader-efi.o -Map map.txt --gc-sections 2>&1 | (grep -v 'has no section'; true)
	x86_64-efi-pe-objdump -d $@ > $@.dsm
	x86_64-efi-pe-strip $@

.obj/uefi.o .obj/libuefi.rlib: ../libuefi/src/lib.rs $(TARGET_FILE) .obj/libcore.rlib
	@mkdir -p .obj/
	$(RUSTC) $< --emit=link,obj,dep-info $(RUSTFLAGS)

.obj/core.o .obj/libcore.rlib: ../../libcore/lib.rs $(TARGET_FILE)
	@mkdir -p .obj/
	$(RUSTC) $< --emit=link,obj,dep-info $(RUSTFLAGS) --cfg disable_float

.obj/main.o: main.rs .obj/libcore.rlib .obj/libuefi.rlib $(TARGET_FILE) Makefile
	mkdir -p .obj/
	$(RUSTC) --emit=obj,dep-info,asm -O $< $(RUSTFLAGS)

-include .obj/main.d
-include .obj/core.d
-include .obj/uefi.d

