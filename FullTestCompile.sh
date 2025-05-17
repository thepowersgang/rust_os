#!/bin/sh
# Quick script to test that the kernel builds properly for all architectures
make -C Kernel ARCH=armv7 .obj/armv7/libmain.a
make -C Kernel ARCH=armv8 .obj/armv8/libmain.a
make -C Kernel ARCH=riscv64 .obj/riscv64/libmain.a
make -C Kernel ARCH=amd64 .obj/amd64/libmain.a
