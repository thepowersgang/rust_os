
TARBALL_URL := https://acpica.org/sites/acpica/files/acpica-unix2-20150410.tar.gz
TARBALL := $(notdir $(TARBALL_URL))
TARBALL_DIR := $(TARBALL:%.tar.gz=%)/
SRCDIR := $(TARBALL_DIR)source/
COMPONENT_DIR := $(SRCDIR)components/
OBJDIR := .obj/$(TRIPLE)/

SRCS := $(wildcard $(COMPONENT_DIR)*/*.c) $(COMPONENT_DIR)hardware/hwacpi.c
OBJS := $(SRCS:$(COMPONENT_DIR)%.c=$(OBJDIR)%.o)

PATCHED := $(SRCDIR)include/platform/acenv.h $(SRCDIR)include/platform/acrust.h


all: acpica-$(TRIPLE).a

$(TARBALL): Makefile
	wget $(TARBALL_URL) -N --no-check-certificate

$(SRCDIR)../Makefile: $(TARBALL)
	tar -xf $(TARBALL)

$(TARBALL_DIR)%: patches/%.patch $(TARBALL)
	tar -xf $(TARBALL) $@
	patch $@ $<
$(TARBALL_DIR)%: patches/%
	cp $< $@

acpica-$(TRIPLE).a: $(SRCDIR)../Makefile $(PATCHED) $(OBJS)
	ar rcu $@ $(OBJS)
	
$(OBJDIR)%.o: $(COMPONENT_DIR)%.c
	mkdir -p $(dir $@)
	$(TRIPLE)-gcc -o $@ -c $< -ffreestanding -I $(SRCDIR)include/ -D RUST

