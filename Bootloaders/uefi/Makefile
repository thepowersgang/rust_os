
-include ../../common.mk

BIN := ../bootloader-efi.pe
TARGET_FILE := Core/arch/amd64/target.json
LIBCORE := ../../Kernel/.obj/amd64/libcore.rlib

all: $(BIN)

$(BIN): .obj/main.o .obj/libcore.o Makefile
	x86_64-none-elf-ld -r -o .obj/bootloader-efi.o -e efi_main .obj/main.o .obj/libcore.o --gc-sections
	x86_64-w64-mingw32-ld -dll -shared --subsystem 10 -e efi_main -o $@ .obj/bootloader-efi.o -Map map.txt
	x86_64-w64-mingw32-objdump -d $@ > $@.dsm
	x86_64-w64-mingw32-strip $@

.obj/libcore.o: $(LIBCORE)
	x86_64-none-elf-ar x $< core.0.o
	mv core.0.o .obj/libcore.o

.obj/main.o: main.rs $(LIBCORE) ../../Kernel/$(TARGET_FILE) Makefile
	mkdir -p $(dir $(TARGET_FILE)) && cp ../../Kernel/$(TARGET_FILE) $(TARGET_FILE)
	mkdir -p .obj/
	$(RUSTC) --emit=obj,dep-info -O --out-dir .obj/ $< --target=$(TARGET_FILE) --extern core=$(LIBCORE) -Z no-landing-pads

-include .obj/main.d

