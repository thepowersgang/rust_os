
run:
	make -C Kernel/ ../libcore/lib.rs
	make -C Usermode/
	make -C Kernel/ run

all:
	make -C Kernel/ ../libcore/lib.rs
	make -C Usermode/ all
	make -C Kernel/ all

UPDATE:
	make -C Kernel/ UPDATE
