
ARCH ?= amd64

ifeq ($(ARCH),amd64)
  TRIPLE ?= x86_64-none-elf
else ifeq ($(ARCH),armv7)
  TRIPLE ?= arm-elf-eabi
else
  $(error Unknown architecture $(ARCH) in common.mk)
endif

CC := $(TRIPLE)-gcc
LD := $(TRIPLE)-ld
AS := $(TRIPLE)-as
OBJDUMP := $(TRIPLE)-objdump

ROOTDIR := $(dir $(lastword $(MAKEFILE_LIST)))

PREFIX := $(ROOTDIR).prefix/

fn_getdeps = $(shell cat $1 | sed -nr 's/.*extern crate ([a-zA-Z_]+);/\1/p' | tr '\n' ' ')
fn_rustcmd = LD_LIBRARY_PATH=$(PREFIX)lib/ $(PREFIX)bin/$1

RUSTC := $(call fn_rustcmd,rustc)
RUSTDOC := $(call fn_rustcmd,rustdoc)
CARGO := $(call fn_rustcmd,cargo)

