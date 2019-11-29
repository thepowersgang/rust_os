-include ../common.mk

.PHONY: all

SHARED_DIR := .output/shared/

all: $(addprefix $(SHARED_DIR),power.r8 options.r8 background.r24 menu.r8 panic.rs logo.rs)

$(SHARED_DIR)logo.rs: TifflinLogoV1-128.png Makefile
	@$(CARGO) run --bin to_32bpp_rs -- $< $@ S_LOGO
$(SHARED_DIR)panic.rs: PanicImageNA.png Makefile
	@$(CARGO) run --bin to_rle32_rs -- $< $@ PANIC_IMAGE

%.r8:
	@mkdir -p $(dir $@)
	@$(CARGO) run --bin to_raw8 $< $@
%.r24:
	@mkdir -p $(dir $@)
	@$(CARGO) run --bin to_rle24 $< $@


$(SHARED_DIR)background.r24: rustacean-orig.png Makefile
$(SHARED_DIR)power.r8: open-iconic_moon-2x.png Makefile
$(SHARED_DIR)options.r8: open-iconic_cog-2x.png Makefile
$(SHARED_DIR)menu.r8: TifflinLogoV1-BW-16.png Makefile

