
-include common.mk

run: all
	make -C Kernel/rundir run

all:
	@echo ">>> $@: Graphics"
	@make -C Graphics/ all
	@echo ">>> $@: Usermode"
	@+make -C Usermode/ xargo --no-print-directory
	@echo ">>> $@: Kernel"
	@+make -C Kernel/ -f Makefile-xargo all --no-print-directory

clean:
	@echo ">>> $@: Usermode"
	@+make -C Usermode/ $@ --no-print-directory
	@echo ">>> $@: Kernel"
	@+make -C Kernel/ $@ --no-print-directory

UPDATE:
	@echo ">>> Updating rustc and libcore"
	@mkdir -p .prefix
	curl https://static.rust-lang.org/rustup/rustup-init.sh -sSf | RUSTUP_HOME=$(abspath .prefix) CARGO_HOME=$(abspath .prefix) sh -s -- --default-toolchain none --no-modify-path -y
	$(call fn_rustcmd,rustup) update $(RUSTUP_VER)
	$(call fn_rustcmd,rustup) default $(RUSTUP_VER)
	$(call fn_rustcmd,rustup) component add rust-src
	$(CARGO) install xargo --git https://github.com/thepowersgang/xargo --force
