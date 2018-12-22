
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


ifeq ($(RUSTC_DATE),)
 RUSTUP_VER := nightly
 RUSTC_SRC_URL := https://static.rust-lang.org/dist/rustc-nightly-src.tar.gz
else
 RUSTUP_VER := nightly-$(RUSTC_DATE)
 RUSTC_SRC_URL := https://static.rust-lang.org/dist/$(RUSTC_DATE)/rustc-nightly-src.tar.gz
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
fn_rustcmd = RUSTUP_HOME=$(abspath $(PREFIX)) CARGO_HOME=$(abspath $(PREFIX)) $(abspath $(PREFIX)bin/$1)

RUSTC := $(call fn_rustcmd,rustc)
RUSTDOC := $(call fn_rustcmd,rustdoc)
CARGO := $(call fn_rustcmd,cargo)
XARGO := XARGO_HOME=$(abspath $(PREFIX)xargo) $(call fn_rustcmd,xargo)

#RUSTUP_SRC_DIR = $(firstword $(wildcard $(PREFIX)toolchains/nightly-*/lib/rustlib/src/rust/src))/
RUSTUP_SRC_DIR := $(abspath $(ROOTDIR)/rustc-nightly-src/src)/
$(warning $(RUSTUP_SRC_DIR))

