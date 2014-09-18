//
//
//
#![macro_escape]

macro_rules! module_define
{
	($name:ident, [], $init:path) => (
		#[assume_reachable]
		#[link_section = ".MODULE_LIST"]
		pub static mut _s_module: ::modules::ModuleInfo = ::modules::ModuleInfo {
			name: stringify!($name),
			init: $init,
		};
	);
	($name:ident, [$($deps:ident),+], $init:path) => (
		#[assume_reachable]
		#[link_section = ".MODULE_LIST"]
		static mut _s_module: ::modules::ModuleInfo = {
			name: stringify!($name),
			init: $init,
		};
	);
}

// vim: ft=rust
