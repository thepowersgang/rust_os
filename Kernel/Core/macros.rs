//
//
//
#![macro_escape]

macro_rules! while_let
{
	($p:pat = $e:expr $code:block) => ( loop { match $e { $p => {$code}, _ => break } } );
}
macro_rules! if_let
{
	($p:pat = $e:expr $code:block) => ( match $e { $p => $code, _ =>{}} );
}

macro_rules! _count
{
	() => {0};
	($a:ident) => {1};
	($a:ident, $($b:ident)+) => {1+_count!($b)};
}

macro_rules! module_define_int
{
	($name:ident, $count:expr, $deps:expr, $init:path) => (
		//#[assume_reachable]
		#[link_section = ".MODULE_LIST"]
		pub static mut _s_module: ::modules::ModuleInfo = ::modules::ModuleInfo {
			name: stringify!($name),
			init: $init,
			deps: &s_deps,
			_rsvd: 0,
		};
		static s_deps: [&'static str, ..($count)] = $deps;
	);
}

macro_rules! module_define
{
	($name:ident, [], $init:path) => (module_define_int!($name, 0, [], $init));
	($name:ident, [$($deps:ident),+], $init:path) => (module_define_int!($name, _count!($($deps),+), [$(stringify!($deps)),+], $init));
}

// vim: ft=rust
