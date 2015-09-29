
-include ../../common.mk

.PHONY: run

QEMU_ARGS := -s -no-reboot
ifeq ($(ENABLE_VIDEO),)
	QEMU_ARGS += -nographic
else
	QEMU_ARGS += -serial stdio
endif
QEMU_ARGS += -d int,guest_errors -D qemu_int_log.txt
ifneq ($(QEMU_STALL),)
	QEMU_ARGS += -S
endif
CMDLINE := SYSDISK=ATA-0p0 SYSROOT=Tifflin

#
# --- AMD64 ---
#
ifeq ($(ARCH),amd64)
QEMU_ARGS += -net nic
QEMU_ARGS += -drive if=ide,index=0,file=DiskImages/hda.img,format=raw
#QEMU_ARGS += -drive if=ide,index=1,file=DiskImages/hdb.img,format=raw
#QEMU_ARGS += -trace file=qemu_trace
#QEMU_OPTS += -trace events=.qemu_trace,file=qemu_trace

MODE ?= pxe
ifeq ($(MODE),iso)
	SCRIPT := RunQemuISO
else
	SCRIPT := RunQemuPXE
endif

run: DiskImages/hda.img .qemu_trace
	./$(SCRIPT) ../bin/kernel-amd64.bin.elf32 "$(CMDLINE)" "$(QEMU_ARGS)" | tee QemuLog.txt
#
# --- ARMv7 ---
#
else ifeq ($(ARCH),armv7)
run:
	qemu-system-arm -machine virt $(QEMU_ARGS) -kernel ../bin/kernel-armv7.bin -append $(CMDLINE) | tee QemuLog.txt
endif


%.dtb: %.dts
	dtc $< -O dtb -o $@

DiskImages/hda_0.img:
	mkdir -p $(dir $@)
	# - 1MB of blank space 
	dd if=/dev/zero of=$@.img bs=1M count=1
DiskImages/hda_1.img: $(wildcard ../../Usermode/.output/$(ARCH)/*) Makefile
	# - 32MB FAT? partition on disk 0
	dd if=/dev/zero of=$@ bs=1M count=32
	/sbin/mkfs.vfat $@
	mmd -i $@ ::/Tifflin
	mcopy -s -D o -i $@ ../../Usermode/.output/$(ARCH)/bin ::/Tifflin/bin
	echo "Test content" | mcopy -i $@ - ::/1.txt
DiskImages/hda_2.img:
	# - a 16MB ext2 partition on disk 0
	#genext2fs -d ../../Usermode/.output/amd64 -b $((16*1024)) DiskImages/hda_2.img 
	dd if=/dev/zero of=$@ bs=1M count=16
	/sbin/mkfs.ext2 -q -F $@
DiskImages/hda.img: Makefile DiskImages/hda_0.img DiskImages/hda_1.img DiskImages/hda_2.img
	mkdir -p $(dir $@)
	# - Commit
	cat DiskImages/hda_0.img DiskImages/hda_1.img DiskImages/hda_2.img > DiskImages/hda.img
	echo "$(shell echo $$((1*1024*2)),$$((32*1024*2)),0x83)\n$(shell echo $$((33*1024*2)),+,0x7)" | /sbin/sfdisk --no-reread DiskImages/hda.img -C $$((1+32+16)) -H 4 -S 256 -u S -f -q

.qemu_trace: Makefile
	echo "" 
	echo "" > $@
	#echo "apic_deliver_irq" >> $@
	#echo "apic_local_deliver" >> $@
	#echo "apic_mem_readl" >> $@
	echo "bdrv_aio_readv" >> $@
	echo "bdrv_aio_writev" >> $@
	echo "bdrv_co_readv" >> $@
	echo "bdrv_co_do_copy_on_readv" >> $@


