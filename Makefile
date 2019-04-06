
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
#	curl $(RUSTC_SRC_URL) -o rustc-nightly-src.tar.gz
#	tar -xf rustc-nightly-src.tar.gz --wildcards rustc-nightly-src/src/lib\* rustc-nightly-src/src/stdsimd rustc-nightly-src/vendor/compiler_builtins
#	rm -rf rustc-nightly-src/src/libcompiler_builtins; mv rustc-nightly-src/vendor/compiler_builtins rustc-nightly-src/src/libcompiler_builtins
	$(CARGO) install xargo --git https://github.com/thepowersgang/xargo --force
