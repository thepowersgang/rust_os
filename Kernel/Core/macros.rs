//
//
//
#[macro_export]
macro_rules! is
{
	($val:expr, $p:pat) => ( match $val { $p => true, _ => false } );
}

#[macro_export]
macro_rules! _count
{
	() => {0};
	($a:ident) => {1};
	($a:ident, $($b:ident)+) => {1+_count!($($b),+)};
}

#[macro_export]
macro_rules! module_define
{
	($name:ident, [$($deps:ident),*], $init:path) => (
		//#[assume_reachable]
		#[link_section = ".MODULE_LIST"]
		pub static mut _s_module: $crate::modules::ModuleInfo = $crate::modules::ModuleInfo {
			name: stringify!($name),
			init: $init,
			deps: &S_DEPS,
			_rsvd: 0,
		};
		static S_DEPS: [&'static str; _count!($($deps),*)] = [$(stringify!($deps)),*];
	);
}

// vim: ft=rust
