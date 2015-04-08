//
//
//
/// Returns true if the passed exppression matches the pattern
#[macro_export]
macro_rules! is
{
	($val:expr, $p:pat) => ( match $val { $p => true, _ => false } );
}

#[doc(hidden)]
#[macro_export]
macro_rules! _count
{
	() => {0};
	($a:ident) => {1};
	($a:ident, $($b:ident)+) => {1+_count!($($b),+)};
}

/// Define a kernel module (creates the module header, containg the name and dependency strings)
#[macro_export]
macro_rules! module_define
{
	($name:ident, [$($deps:ident),*], $init:path) => (
		//#[assume_reachable]
		#[doc(hidden)]
		#[link_section = ".MODULE_LIST"]
		pub static mut _s_module: $crate::modules::ModuleInfo = $crate::modules::ModuleInfo {
			name: stringify!($name),
			init: $init,
			deps: &S_DEPS,
			_rsvd: 0,
		};
		#[doc(hidden)]
		static S_DEPS: [&'static str; _count!($($deps),*)] = [$(stringify!($deps)),*];
	);
}

/// Ensure that a type implments the provided trait
///
/// Useful for "Send" and "Sync"
#[macro_export]
macro_rules! assert_trait
{
	($t:ty : $tr:ident) => { #[allow(warnings)] fn assert_trait<T: $tr>() { assert_trait::<$t>() } }
}

// vim: ft=rust
