
TARBALL_URL := https://acpica.org/sites/acpica/files/acpica-unix2-20150410.tar.gz
TARBALL := $(notdir $(TARBALL_URL))
SRCDIR := $(TARBALL:%.tar.gz=%)/source/components/
OBJDIR := .obj/$(TRIPLE)/

SRCS := $(wildcard $(SRCDIR)*/*.c) $(SRCDIR)hardware/hwacpi.c
OBJS := $(SRCS:$(SRCDIR)%.c=$(OBJDIR)%.o)

all: acpica-$(TRIPLE).a

$(TARBALL): Makefile
	wget $(TARBALL_URL) -N --no-check-certificate

$(SRCDIR)%.c: $(TARBALL)
	tar -xf $(TARBALL)

acpica-$(TRIPLE).a: $(OBJS)
	ar rcu $@ $(OBJS)
	
$(OBJDIR)%.o: $(SRCDIR)%.c
	$(TRIPLE)-gcc -o $@ -c $< -ffreestanding -I $(SRCDIR)/../include/

