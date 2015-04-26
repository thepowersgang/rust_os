
ifeq ($(TRIPLE),)
 $(error Please specify a target with TRIPLE)
endif

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

clean:
	rm -rf acpica-$(TRIPLE).a $(OBJDIR)

$(TARBALL):
	if [[ ! -e $(TARBALL) ]]; then wget $(TARBALL_URL) --no-check-certificate -O $@; fi

$(SRCDIR):
	tar -xf $(TARBALL)

$(TARBALL_DIR)%: patches/%.patch $(TARBALL)
	tar -xf $(TARBALL) $@
	patch $@ $<
$(TARBALL_DIR)%: patches/%
	cp $< $@

acpica-$(TRIPLE).a: $(SRCDIR) $(PATCHED) $(OBJS)
	@echo [AR] $@
	@ar rcu $@ $(OBJS)
	
$(OBJDIR)%.o: $(COMPONENT_DIR)%.c Makefile
	@mkdir -p $(dir $@)
	@echo [GCC] -o $@
	@$(TRIPLE)-gcc -o $@ -c $< -ffreestanding -I $(SRCDIR)include/ -D RUST -D ACPI_LIBRARY -mcmodel=kernel

