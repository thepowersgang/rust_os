//
//
//
#![macro_escape]

macro_rules! _count
{
	() => {0};
	($a:ident) => {1};
	($a:ident, $($b:ident)+) => {1+_count!($b)};
}

macro_rules! module_define
{
	($name:ident, [], $init:path) => (
		#[assume_reachable]
		#[link_section = ".MODULE_LIST"]
		pub static mut _s_module: ::modules::ModuleInfo = ::modules::ModuleInfo {
			name: stringify!($name),
			init: $init,
			deps: &s_deps,
			_rsvd: 0,
		};
		static s_deps: [&'static str, ..0] = [];
	);
	($name:ident, [$($deps:ident),+], $init:path) => (
		#[assume_reachable]
		#[link_section = ".MODULE_LIST"]
		pub static _s_module: ::modules::ModuleInfo = ::modules::ModuleInfo {
			name: stringify!($name),
			init: $init,
			deps: &s_deps,
			_rsvd: 0,
		};
		static s_deps: [&'static str, .._count!( $($deps),+ )] = [$(stringify!($deps)),+];
	);
}

// vim: ft=rust
