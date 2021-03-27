
.PHONY: default
default: all

nop :=
space := $(nop) $(nop)
comma := ,

ARCH ?= amd64

ifeq ($(ARCH),amd64)
  TRIPLE ?= x86_64-none-elf
else ifeq ($(ARCH),armv7)
  TRIPLE ?= arm-elf-eabi
else ifeq ($(ARCH),armv8)
  TRIPLE ?= aarch64-none-elf
else ifeq ($(ARCH),riscv64)
  TRIPLE ?= riscv64-unknown-elf
else ifeq ($(ARCH),native)
  TRIPLE ?= 
else
  $(error Unknown architecture $(ARCH) in common.mk)
endif


ifeq ($(RUSTC_DATE),)
 RUSTUP_VER := nightly
else
 RUSTUP_VER := nightly-$(RUSTC_DATE)
endif

ROOTDIR := $(dir $(lastword $(MAKEFILE_LIST)))
PREFIX := $(ROOTDIR).prefix/

PATH := $(PATH):$(PREFIX)bin

CC := $(TRIPLE)-gcc
LD := $(TRIPLE)-ld
AS := $(TRIPLE)-as
OBJDUMP := $(TRIPLE)-objdump
OBJCOPY := $(TRIPLE)-objcopy
STRIP := $(TRIPLE)-strip


fn_getdeps = $(shell cat $1 | sed -nr 's/.*extern crate ([a-zA-Z_0-9]+)( as .*)?;.*/\1/p' | tr '\n' ' ')
fn_rustcmd = RUSTUP_HOME=$(abspath $(PREFIX)) CARGO_HOME=$(abspath $(PREFIX)) $(abspath $(PREFIX)bin/$1)

RUSTC := $(call fn_rustcmd,rustc)
RUSTDOC := $(call fn_rustcmd,rustdoc)
CARGO := $(call fn_rustcmd,cargo)

