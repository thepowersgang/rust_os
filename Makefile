
run:
	@echo ">>> $@: libcore source"
	@make -C Kernel/ ../libcore/lib.rs --no-print-directory
	@echo ">>> $@: Usermode"
	@make -C Usermode/ --no-print-directory
	@echo ">>> $@: Kernel"
	@make -C Kernel/ run --no-print-directory

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
