
-include ../../common.mk

.PHONY: run

QEMU_ARGS := -serial stdio -s -no-reboot
QEMU_ARGS += -d int,guest_errors -D qemu.log
ifneq ($(QEMU_STALL),)
	QEMU_ARGS += -S
endif
CMDLINE := "SYSDISK=ATA-0p0 SYSROOT=Tifflin"

ifeq ($(ARCH),armv7)
run:
	qemu-system-arm -machine virt $(QEMU_ARGS) -kernel ../bin/kernel-armv7.bin -append $(CMDLINE)
endif

%.dtb: %.dts
	dtc $< -O dtb -o $@
