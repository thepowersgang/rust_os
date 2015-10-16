
ifeq ($(TRIPLE),)
 $(error Please specify a target with TRIPLE)
endif

TARBALL_URL := https://acpica.org/sites/acpica/files/acpica-unix2-20150410.tar.gz
TARBALL := $(notdir $(TARBALL_URL))
TARBALL_DIR := $(TARBALL:%.tar.gz=%)/
SRCDIR := $(TARBALL_DIR)source/
OBJDIR := .obj/$(TRIPLE)/

SRCS := $(wildcard $(SRCDIR)components/*/*.c)
SRCS := $(filter-out $(SRCDIR)components/disassembler/%,$(SRCS))
SRCS += $(SRCDIR)components/hardware/hwacpi.c
#SRCS += $(SRCDIR)common/ahuuids.c $(SRCDIR)common/ahids.c
#SRCS += $(SRCDIR)compiler/aslmapenter.c
OBJS := $(SRCS:$(SRCDIR)%.c=$(OBJDIR)%.o)

PATCHED := $(SRCDIR)include/platform/acenv.h $(SRCDIR)include/platform/acrust.h

CPPFLAGS := -I $(SRCDIR)include/ -D RUST -D ACPI_LIBRARY
CPPFLAGS += -D ACPI_DEBUG_OUTPUT=1
# -D ACPI_DISASSEMBLER=1
CFLAGS := -ffreestanding
ifeq ($(ARCH),amd64)
CFLAGS += -mcmodel=kernel -mno-red-zone
endif
CFLAGS += -O2

.PHONY: all clean srcdir

all: acpica-$(TRIPLE).a

clean:
	rm -rf acpica-$(TRIPLE).a $(OBJDIR)

srcdir: $(SRCDIR)

$(TARBALL):
	if [ ! -e $(TARBALL) ]; then wget $(TARBALL_URL) --no-check-certificate -O $@; fi

$(SRCDIR): $(TARBALL)
	tar -xf $(TARBALL)

$(TARBALL_DIR)%: patches/%.patch $(TARBALL)
	tar -xf $(TARBALL) $@
	patch $@ $<
$(TARBALL_DIR)%: patches/%
	cp $< $@

acpica-$(TRIPLE).a: $(SRCDIR) $(PATCHED) $(OBJS)
	@echo [AR] $@
	@rm -f $@ && ar rc $@ $(OBJS)
	
$(OBJDIR)%.o: $(SRCDIR)%.c $(PATCHED) Makefile
	@mkdir -p $(dir $@)
	@echo [GCC] -o $@
	@$(TRIPLE)-gcc -o $@ -c $< $(CFLAGS) $(CPPFLAGS)

