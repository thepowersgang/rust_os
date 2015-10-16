
.PHONY: all

SHARED_DIR := .output/shared/

all: $(addprefix $(SHARED_DIR),power.r8 options.r8 background.r24)


# NOTE: This is actually done in Kernel/Makefile
$(SHARED_DIR)logo.rs: TifflinLogoV1-128.png Makefile ConvertTo32bppRS
	./ConvertTo32bppRS $< $@ S_LOGO

%.r8:
	mkdir -p $(dir $@)
	convert $< -depth 8 -channel A -separate gray:.tmp.$(notdir $@)
	$(eval DIMS16=$(shell printf '%04x %04x' $(shell identify $< | sed -r 's/.* ([0-9]+)x([0-9]+) .*/\1 \2/') | sed -r 's/(..)(..) (..)(..)/\\x\2\\x\1\\x\4\\x\3/'))
	/bin/echo -ne "\x7FR8M$(DIMS16)" | cat - .tmp.$(notdir $@) > $@

%.r24:
	mkdir -p $(dir $@)
	convert $< -depth 8 rgb:.tmp.$(notdir $@)
	$(eval DIMS16=$(shell printf '%04x %04x' $(shell identify $< | sed -r 's/.* ([0-9]+)x([0-9]+) .*/\1 \2/') | sed -r 's/(..)(..) (..)(..)/\\x\2\\x\1\\x\4\\x\3/'))
	/bin/echo -ne "\x7FR24$(DIMS16)" | cat - .tmp.$(notdir $@) > $@

$(SHARED_DIR)background.r24: rustacean-orig.png Makefile
$(SHARED_DIR)power.r8: open-iconic_moon-2x.png Makefile
$(SHARED_DIR)options.r8: open-iconic_cog-2x.png Makefile

