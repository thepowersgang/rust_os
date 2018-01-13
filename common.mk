
.PHONY: default
default: all

ARCH ?= amd64

ifeq ($(ARCH),amd64)
  TRIPLE ?= x86_64-none-elf
else ifeq ($(ARCH),armv7)
  TRIPLE ?= arm-elf-eabi
else ifeq ($(ARCH),armv8)
  TRIPLE ?= aarch64-none-elf
else
  $(error Unknown architecture $(ARCH) in common.mk)
endif

CC := $(TRIPLE)-gcc
LD := $(TRIPLE)-ld
AS := $(TRIPLE)-as
OBJDUMP := $(TRIPLE)-objdump
OBJCOPY := $(TRIPLE)-objcopy
STRIP := $(TRIPLE)-strip

ROOTDIR := $(dir $(lastword $(MAKEFILE_LIST)))

PREFIX := $(ROOTDIR).prefix/

fn_getdeps = $(shell cat $1 | sed -nr 's/.*extern crate ([a-zA-Z_0-9]+)( as .*)?;.*/\1/p' | tr '\n' ' ')
fn_rustcmd = RUSTUP_HOME=$(abspath $(PREFIX)) CARGO_HOME=$(abspath $(PREFIX)) $(PREFIX)bin/$1

RUSTC := $(call fn_rustcmd,rustc)
RUSTDOC := $(call fn_rustcmd,rustdoc)
CARGO := $(call fn_rustcmd,cargo)
XARGO := $(call fn_rustcmd,xargo)

$(patsubst %,../rustc_src/lib%/lib.rs,core collections std_unicode alloc): ../rustc-nightly-src.tar.gz
	tar -C .. -xmf $< --wildcards 'rustc-nightly-src/src/lib*' rustc-nightly-src/src/rt --transform 's~^rustc-nightly-src/src/~rustc_src/~'

