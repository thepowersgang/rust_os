
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
	#@make -C Kernel/ UPDATE --no-print-directory
	#@make -C Kernel/ ../libcore/lib.rs --no-print-directory
	#
	@mkdir -p ../.prefix
	curl https://static.rust-lang.org/rustup/rustup-init.sh -sSf | RUSTUP_HOME=$(abspath ../.prefix) CARGO_HOME=$(abspath ../.prefix) sh -s -- --default-toolchain none --no-modify-path -y
	$(call fn_rustcmd,rustup) update $(RUSTUP_VER)
	$(call fn_rustcmd,rustup) default $(RUSTUP_VER)
	#$(call fn_rustcmd,rustup) component add rust-src
	curl $(RUSTC_SRC_URL) -o rustc-nightly-src.tar.gz
	tar -xf rustc-nightly-src.tar.gz --wildcards rustc-nightly-src/src/lib\* rustc-nightly-src/src/stdsimd rustc-nightly-src/vendor/compiler_builtins
	rm -rf rustc-nightly-src/src/libcompiler_builtins; mv rustc-nightly-src/vendor/compiler_builtins rustc-nightly-src/src/libcompiler_builtins
	test -f .prefix/bin/xargo || $(CARGO) install xargo
