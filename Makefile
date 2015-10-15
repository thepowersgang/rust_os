
-include common.mk

run: all
	make -C Kernel/rundir run

all:
	@echo ">>> $@: libcore source"
	@+make -C Kernel/ ../libcore/lib.rs --no-print-directory
	@echo ">>> $@: Graphics"
	@make -C Graphics/ all
	@echo ">>> $@: Usermode"
	@+make -C Usermode/ all --no-print-directory
	@echo ">>> $@: Kernel"
	@+make -C Kernel/ all --no-print-directory

clean:
	@echo ">>> $@: Usermode"
	@+make -C Usermode/ $@ --no-print-directory
	@echo ">>> $@: Kernel"
	@+make -C Kernel/ $@ --no-print-directory

UPDATE:
	@echo ">>> Updating rustc and libcore"
	@make -C Kernel/ UPDATE --no-print-directory
