
RUSTFLAGS := -C code-model=kernel -mtriple=x86_64-pc-elf

ARCH = amd64
OBJS := start_$(ARCH).o main.o libarch.rlib

OBJDIR := obj/$(ARCH)/
OBJS := $(OBJS:%=$(OBJDIR)%)

all: ../kernel-$(ARCH).bin

clean:
	rm -r $(OBJDIR)

../kernel-$(ARCH).bin: $(OBJS) link_$(ARCH).ld Makefile
	x86_64-elf-ld -o $@ $(OBJS) -T link_$(ARCH).ld -Map $(OBJDIR)map.txt -z max-page-size=0x1000
	objcopy $@ -F elf32-i386 $@.elf32
	mcopy -i ../fdd.img ../kernel-amd64.bin.elf32 ::/ -D o

$(OBJDIR)%.o: %.asm Makefile
	mkdir -p $(dir $@)
	nasm -o $@ $< -f elf64

$(OBJDIR)main.o: main.rs $(OBJDIR)libarch.rlib Makefile
	mkdir -p $(dir $@)
	rustc -O --crate-type lib --emit obj -o $@ $< --extern arch=$(OBJDIR)/libarch.rlib

$(OBJDIR)libarch.rlib: arch_$(ARCH)/crate.rs Makefile
	mkdir -p $(dir $@)
	rustc -O --crate-type lib --crate-name arch -o $@ $<

