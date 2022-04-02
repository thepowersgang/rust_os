
-include ../../common.mk
V ?= @

.PHONY: run all images

all: run

QEMU_ARGS := -s -no-reboot
ifeq ($(ENABLE_VIDEO),)
  #QEMU_ARGS += -nographic
  QEMU_ARGS += -vnc 127.0.0.1:99
  QEMU_ARGS += -serial mon:stdio
else
  QEMU_ARGS += -serial stdio
endif
ifeq ($(ENABLE_USB),)
else
  QEMU_ARGS += -usb
  QEMU_ARGS += -device pci-ohci,id=ohci
  QEMU_ARGS += -drive if=none,id=usbdisk,file=DiskImages/$(ARCH)/$(IMGDIR)usb.img,format=raw
  QEMU_ARGS += -device usb-hub,bus=ohci.0,port=1
  QEMU_ARGS += -device usb-storage,bus=ohci.0,port=1.1,drive=usbdisk
images: DiskImages/$(ARCH)/$(IMGDIR)usb.img
  QEMU_ARGS += -device usb-tablet,bus=ohci.0,port=2
#  QEMU_ARGS += -device usb-kbd,bus=ohci.0
endif
QEMU_ARGS += -d int,guest_errors -D qemu_int_log.txt
#QEMU_ARGS += -d int,guest_errors,exec -D qemu_int_log.txt
ifneq ($(QEMU_STALL),)
	QEMU_ARGS += -S
endif
ifneq ($(QEMU_REPLAY),)
QEMU_ARGS += -icount shift=7,rr=$(QEMU_REPLAY),rrfile=replay.bin
endif
CMDLINE := SYSROOT=/system/Tifflin

#QEMU_ARGS += -tcg-plugin $(shell dirname $(shell type -P qemu-system-x86_64))/../libexec/tcg-plugin-oprofile.so -singlestep

ifneq ($(TIFFLIN_INIT),)
	CMDLINE += INIT=$(TIFFLIN_INIT)
endif
ifneq ($(TIFFLIN_TESTS),)
	CMDLINE += TEST=$(TIFFLIN_TESTS)
endif


ifeq ($(NOTEE),)
 TEE := 2>&1 | tee QemuLog.txt
else
 TEE := 
endif

IMGDIR := DiskImages/$(ARCH)/

QEMU_TRACE_SPEC := .qemu_trace_spec
ifneq ($(ENABLE_TRACE),)
  # qemu-source/scripts/simpletrace.py qemu-build/trace/trace-events trace-XXXXXX
  QEMU_ARGS += -trace events=$(QEMU_TRACE_SPEC)
#,file=qemu_trace_output	# < Doesn't work :(
endif

#
# --- AMD64 ---
#
ifeq ($(ARCH),amd64)
CMDLINE += SYSDISK=ATA-0p0
QEMU_ARGS += -vga virtio
QEMU_ARGS += -net nic,model=rtl8139
# -net dump
QEMU_ARGS += -drive if=ide,index=0,file=$(IMGDIR)hda.img,format=raw
QEMU_ARGS += -cdrom $(IMGDIR)test.iso
QEMU_ARGS += -device ahci,id=ahci
QEMU_ARGS += -drive if=none,id=sata1,file=$(IMGDIR)hdb.img,format=raw -device ide-hd,drive=sata1,bus=ahci.0
QEMU_ARGS += -drive if=none,id=sata2,file=$(IMGDIR)test.iso,format=raw -device ide-cd,drive=sata2,bus=ahci.1

MODE ?= pxe
ifeq ($(MODE),iso)
	SCRIPT := RunQemuISO
else ifeq ($(MODE),efi)
	SCRIPT := RunQemuEfiHdd
else
	SCRIPT := RunQemuPXE
endif

run: $(QEMU_TRACE_SPEC) images
	./$(SCRIPT) ../bin/kernel-amd64.bin.elf32 "$(CMDLINE)" "$(QEMU_ARGS)" $(TEE)
images: $(IMGDIR)hda.img $(IMGDIR)hdb.img $(IMGDIR)test.iso
#
# --- ARMv7 ---
#
else ifeq ($(ARCH),armv7)
MACHINE_TYPE ?= virt
#MACHINE_TYPE ?= realview-pb-a8

CMDLINE += SYSDISK=virtio0p0
ifeq ($(MACHINE_TYPE),virt)
QEMU_ARGS += -device virtio-net-device
QEMU_ARGS += -device virtio-blk-device,drive=hda
QEMU_ARGS += -device virtio-keyboard-device
QEMU_ARGS += -device virtio-gpu-device
endif
QEMU_ARGS += -drive if=none,id=hda,index=0,file=$(IMGDIR)hda.img,format=raw
#QEMU_ARGS += -device pl110


run: $(IMGDIR)hda.img
	make -C ../../Bootloaders/arm PLATFORM=$(MACHINE_TYPE) CMDLINE="$(CMDLINE)"
	qemu-system-arm -machine $(MACHINE_TYPE) $(QEMU_ARGS) -kernel ../../Bootloaders/arm/loader-$(MACHINE_TYPE).bin -append "$(CMDLINE)" $(TEE)
images: $(IMGDIR)hda.img
# ---
# ARM 64
# ---
else ifeq ($(ARCH),armv8)
MACHINE_TYPE ?= virt
CMDLINE += SYSDISK=virtio0p0
QEMU_ARGS += -device virtio-net-device
QEMU_ARGS += -device virtio-blk-device,drive=hda
QEMU_ARGS += -device virtio-keyboard-device
QEMU_ARGS += -drive if=none,id=hda,index=0,file=$(IMGDIR)hda.img,format=raw

run: $(IMGDIR)hda.img
	make -C ../../Bootloaders/aarch64 PLATFORM=$(MACHINE_TYPE) CMDLINE="$(CMDLINE)" KERNEL=../../Kernel/bin/kernel-armv8.bin
	qemu-system-aarch64 -cpu cortex-a57 -machine $(MACHINE_TYPE) $(QEMU_ARGS) -kernel ../../Bootloaders/aarch64/loader-$(MACHINE_TYPE).bin -append "$(CMDLINE)" $(TEE)
# ---
# RISC-V 64
# ---
else ifeq ($(ARCH),riscv64)
CMDLINE += SYSDISK=virtio0p0
QEMU_ARGS += -device virtio-blk-device,drive=hda
QEMU_ARGS += -device virtio-gpu-device
QEMU_ARGS += -device virtio-net-device
QEMU_ARGS += -device virtio-keyboard-device
QEMU_ARGS += -device virtio-tablet-device
QEMU_ARGS += -drive if=none,id=hda,index=0,file=$(IMGDIR)hda.img,format=raw
#QEMU_ARGS += -smp 4
#QEMU_ARGS += -bios none

run: $(IMGDIR)hda.img
	qemu-system-riscv64 -machine virt -cpu rv64 -m 64M $(QEMU_ARGS) -kernel ../../Kernel/bin/kernel-riscv64.bin -append "$(CMDLINE)" $(TEE)
else
  $(error Unknown architecture in Kernel/rundir/Makefile)
endif

.DELETE_ON_ERROR:

$(IMGDIR)test.iso: $(wildcard ../../Usermode/.output/$(ARCH)/bin/*) Makefile
	@mkdir -p $(dir $@)
	@echo "[mkisofs] -o $@"
	$Vmkisofs -input-charset utf-8 -quiet -o $@ -r -graft-points /Tifflin/bin=../../Usermode/.output/$(ARCH)/bin /Tifflin/shared/images=../../Graphics/.output/shared
$(IMGDIR)usb.img:
	@mkdir -p $(dir $@)
	@echo "[MkDisk] ZERO 1MB $@"
	$Vdd if=/dev/zero of=$@ bs=1M count=1 status=noxfer
$(IMGDIR)hd%_0.img:
	@mkdir -p $(dir $@)
	@echo "[MkDisk] ZERO 1MB $@"
	@# - 1MB of blank space 
	$Vdd if=/dev/zero of=$@ bs=1M count=1 status=noxfer
$(IMGDIR)hda_1.img: $(wildcard ../../Usermode/.output/$(ARCH)/bin/*) Makefile $(wildcard ../../Graphics/.output/shared/*)
	@mkdir -p $(dir $@)
	@echo "[MkDisk] FAT 32MB $@"
	@# - 32MB FAT? partition on disk 0
	$Vdd if=/dev/zero of=$@ bs=1M count=32 status=noxfer
	$V/sbin/mkfs.vfat $@
	$Vmmd -i $@ ::/Tifflin
	$Vmmd -i $@ ::/Tifflin/shared
	$Vmmd -i $@ ::/Tifflin/shared/images
	$Vmcopy -s -D o -i $@ ../../Usermode/.output/$(ARCH)/bin ::/Tifflin/bin
	$Vmcopy -s -D o -i $@ ../../Graphics/.output/shared/* ::/Tifflin/shared/images/
	$Vecho "Test content" | mcopy -i $@ - ::/1.txt
$(IMGDIR)hda_2.img:
	@mkdir -p $(dir $@)
	@echo "[MkDisk] ext2 16MB $@"
	@# - a 16MB ext2 partition on disk 0
	$Vdd if=/dev/zero of=$@ bs=1M count=16 status=noxfer
	$V/sbin/mkfs.ext2 -q -F $@
$(IMGDIR)hda.img: Makefile $(IMGDIR)hda_0.img $(IMGDIR)hda_1.img $(IMGDIR)hda_2.img
	@mkdir -p $(dir $@)
	@echo "[MkDisk] mbr $@"
	@# - Commit
	$Vcat $(IMGDIR)hda_0.img $(IMGDIR)hda_1.img $(IMGDIR)hda_2.img > $@
	$Vprintf "$(shell echo $$((1*1024*2)),$$((32*1024*2)),0x83)\n$(shell echo $$((33*1024*2)),+,0x7)" | /sbin/sfdisk --no-reread $@ -u S -f -q > /dev/null

$(IMGDIR)hdb_1.img: $(wildcard ../../Usermode/.output/$(ARCH)/*) Makefile
	@echo "[MkDisk] ext2 $@"
	$Vdd if=/dev/zero of=$@ bs=1M count=32
	$V/sbin/mkfs.ext2 $@
	$Vsudo guestfish -a $@ launch : mount /dev/sda / : mkdir /Tifflin : copy-in ../../Usermode/.output/$(ARCH)/bin /Tifflin/
$(IMGDIR)hdb.img: Makefile $(IMGDIR)hdb_0.img $(IMGDIR)hdb_1.img
	@mkdir -p $(dir $@)
	@echo "[MkDisk] mbr $@"
	@# - Commit
	$Vcat $(IMGDIR)hdb_0.img $(IMGDIR)hdb_1.img > $@
	$Vecho "$(shell echo $$((1*1024*2)),+,0x83)" | /sbin/sfdisk --no-reread $@ -u S -f -q > /dev/null

$(QEMU_TRACE_SPEC): Makefile
	echo "" > $@
	#echo pci_* >> $@
	echo virtio_* >> $@
	#echo virtqueue_* >> $@
	#echo usb_* >> $@


