#!/bin/bash
set -eu
export ARCH=amd64  ; make .obj/$ARCH/libmain.a
export ARCH=riscv64; make .obj/$ARCH/libmain.a
export ARCH=armv8  ; make .obj/$ARCH/libmain.a
export ARCH=armv7  ; make .obj/$ARCH/libmain.a