# Tifflin "Native" makefile
#
#
-include ../common.mk

CARGO_PROFILE ?= debug
CARGO_FLAGS :=

ifeq ($(CARGO_PROFILE),debug)
  # No --release
else ifeq ($(CARGO_PROFILE),release)
  CARGO_FLAGS += --release
else
  $(error Unknown cargo profile $(CARGO_PROFILE))
endif

# - List non-file targets
.PHONY: all clean
# - Don't delete intermediate targets
.SECONDARY:
# - (debugging) Remove most default targets
.SUFFIXES:

# DEFAULT: Compile all applications
all:
	$(MAKE) -C ../Usermode ARCH=native
	$(CARGO) build $(CARGO_FLAGS)

# Delete all artifacts
clean:
	$(RM) -rf target

run: all
	@mkdir -p .native_fs/Tifflin
	@cp -r ../Usermode/.output/native/bin .native_fs/Tifflin
	@mkdir -p .native_fs/Tifflin/shared/images
	cp ../Graphics/.output/shared/*.r8 .native_fs/Tifflin/shared/images/
	cp ../Graphics/.output/shared/*.r24 .native_fs/Tifflin/shared/images/
	LD_LIBRARY_PATH=../Usermode/.output/native/bin $(CARGO) run | tee log.txt
