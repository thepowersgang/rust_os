
RUSTFLAGS :=
#RUSTFLAGS := -C code-model=kernel
RUSTFLAGS += -C no-redzone
RUSTFLAGS += -C target-feature=-avx
#RUSTFLAGS += --target=x86_64-pc-elf 

ARCH = amd64
OBJS := arch_$(ARCH)/start.o main.o libarch.rlib

LIBDIR := /home/tpg/apps/lib/rustlib/x86_64-unknown-linux-gnu/lib/
LIBCOMPILERRT := $(LIBDIR)libcompiler-rt.a
LIBCORE := $(LIBDIR)libcore-4e7c5e5c.rlib

OBJDIR := obj/$(ARCH)/
OBJS := $(OBJS:%=$(OBJDIR)%)

all: ../kernel-$(ARCH).bin

clean:
	rm -r $(OBJDIR)

../kernel-$(ARCH).bin: $(OBJS) link_$(ARCH).ld Makefile
	x86_64-elf-ld -o $@ $(OBJS) $(LIBCORE) $(LIBCOMPILERRT) -T link_$(ARCH).ld -Map $(OBJDIR)map.txt -z max-page-size=0x1000
	objcopy $@ -F elf32-i386 $@.elf32
	mcopy -i ../fdd.img ../kernel-amd64.bin.elf32 ::/ -D o
	objdump -S $@ > $@.dsm

$(OBJDIR)%.o: %.asm Makefile
	mkdir -p $(dir $@)
	nasm -o $@ $< -f elf64

$(OBJDIR)main.o: main.rs $(OBJDIR)libarch.rlib Makefile
	mkdir -p $(dir $@)
	rustc -O --crate-type lib $(RUSTFLAGS) --emit obj -o $@ $< --extern arch=$(OBJDIR)libarch.rlib --dep-info $@.dep

$(OBJDIR)libarch.rlib: arch_$(ARCH)/crate.rs Makefile
	mkdir -p $(dir $@)
	rustc -O --crate-type lib $(RUSTFLAGS) --crate-name arch -o $@ $<  --dep-info $@.dep

-include $(OBJS:%=%.dep)

