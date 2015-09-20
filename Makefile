
-include common.mk

run: all
ifeq ($(ARCH),amd64)
	cd Kernel/rundir && ./RunQemuPXE ../bin/kernel-amd64.elf32 "SYSDISK=ATA-0p0 SYSROOT=Tifflin"
else ifeq ($(ARCH),armv7)
	make -C Kernel/rundir run
endif

all:
	@echo ">>> $@: libcore source"
	@make -C Kernel/ ../libcore/lib.rs --no-print-directory
	@echo ">>> $@: Usermode"
	@make -C Usermode/ all --no-print-directory
	@echo ">>> $@: Kernel"
	@make -C Kernel/ all --no-print-directory

clean:
	@echo ">>> $@: Usermode"
	@make -C Usermode/ $@ --no-print-directory
	@echo ">>> $@: Kernel"
	@make -C Kernel/ $@ --no-print-directory

UPDATE:
	@echo ">>> Updating rustc and libcore"
	@make -C Kernel/ UPDATE --no-print-directory
