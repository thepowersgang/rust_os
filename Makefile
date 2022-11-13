
-include common.mk

run: all
	$(MAKE) -C Kernel/rundir run

all:
	@echo ">>> $@: Graphics"
	@$(MAKE) -C Graphics/ all
	@echo ">>> $@: Usermode"
	@+$(MAKE) -C Usermode/ all --no-print-directory
	@echo ">>> $@: Kernel"
	@+$(MAKE) -C Kernel/ all --no-print-directory

clean:
	@echo ">>> $@: Usermode"
	@+$(MAKE) -C Usermode/ $@ --no-print-directory
	@echo ">>> $@: Kernel"
	@+$(MAKE) -C Kernel/ $@ --no-print-directory

UPDATE:
	@echo ">>> Updating rustc and libcore"
	@mkdir -p .prefix
	curl https://static.rust-lang.org/rustup/rustup-init.sh -sSf | RUSTUP_HOME=$(abspath .prefix) CARGO_HOME=$(abspath .prefix) sh -s -- --default-toolchain none --no-modify-path -y
	$(call fn_rustcmd,rustup) update $(RUSTUP_VER) --force
	$(call fn_rustcmd,rustup) default $(RUSTUP_VER)
	$(call fn_rustcmd,rustup) component add rust-src
fn_checkout = (test -e "$1" || git clone `cat "$1.repo"` "$1")
EXTERNALS:
	cd externals/crates.io && $(call fn_checkout,cmdline_words_parser)
	cd externals/crates.io && $(call fn_checkout,utf16_literal)
	cd externals/crates.io && $(call fn_checkout,va_list)
