
-include common.mk

run: all
	make -C Kernel/rundir run

all:
	@echo ">>> $@: Graphics"
	@make -C Graphics/ all
	@echo ">>> $@: Usermode"
	@+make -C Usermode/ --no-print-directory
	@echo ">>> $@: Kernel"
	@+make -C Kernel/ all --no-print-directory

clean:
	@echo ">>> $@: Usermode"
	@+make -C Usermode/ $@ --no-print-directory
	@echo ">>> $@: Kernel"
	@+make -C Kernel/ $@ --no-print-directory

UPDATE:
	@echo ">>> Updating rustc and libcore"
	@mkdir -p .prefix
	curl https://static.rust-lang.org/rustup/rustup-init.sh -sSf | RUSTUP_HOME=$(abspath .prefix) CARGO_HOME=$(abspath .prefix) sh -s -- --default-toolchain none --no-modify-path -y
	$(call fn_rustcmd,rustup) update $(RUSTUP_VER) --force
	$(call fn_rustcmd,rustup) default $(RUSTUP_VER)
	$(call fn_rustcmd,rustup) component add rust-src
fn_checkout = (test -e "$1" || git clone `cat "$1.repo"` "$1")
EXTERNALS:
	cd externals/crates.io && $(call fn_checkout,stack_dst)
	cd externals/crates.io && $(call fn_checkout,cmdline_words_parser)
	cd externals/crates.io && $(call fn_checkout,utf16_literal)
	cd externals/crates.io && $(call fn_checkout,va_list)
