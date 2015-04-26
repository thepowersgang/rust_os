
TARBALL_URL := https://acpica.org/sites/acpica/files/acpica-unix2-20150410.tar.gz
TARBALL := $(notdir $(TARBALL_URL))
SRCDIR := $(TARBALL:%.tar.gz=%)/
COMPONENT_DIR: $(SRCDIR)source/components/
OBJDIR := .obj/$(TRIPLE)/

SRCS := $(wildcard $(COMPONENT_DIR)*/*.c) $(COMPONENT_DIR)hardware/hwacpi.c
OBJS := $(SRCS:$(SRCDIR)%.c=$(OBJDIR)%.o)

PATCHED: $(SRCDIR)include/source/include/platform/acenv.h $(SRCDIR)source/include/platform/acrust.h


all: acpica-$(TRIPLE).a

$(TARBALL): Makefile
	wget $(TARBALL_URL) -N --no-check-certificate

$(SRCDIR)%.c: $(TARBALL)
	tar -xf $(TARBALL)

$(SRCDIR)%: patches/%.patch
	tar -xf $(TARBALL) $@
	patch $@ $<
$(SRCDIR)%: patches/%
	cp $< $@

acpica-$(TRIPLE).a: $(PATCHED) $(OBJS)
	ar rcu $@ $(OBJS)
	
$(OBJDIR)%.o: $(COMPONENT_DIR)%.c
	$(TRIPLE)-gcc -o $@ -c $< -ffreestanding -I $(SRCDIR)include/

