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
	($a:expr) => {1};
	($a:expr, $($b:expr)+) => {1+_count!($($b),+)};
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

/// A safe wrapper around the `type_name` intrinsic
#[macro_export]
macro_rules! type_name
{
	($t:ty) => ( unsafe { ::core::intrinsics::type_name::<$t>() } )
}


macro_rules! todo
{
	( $s:expr ) => ( panic!( concat!("TODO: ",$s) ) );
	( $s:expr, $($v:tt)* ) => ( panic!( concat!("TODO: ",$s), $($v)* ) );
}

/// Provides a less boiler-plate way to implement fmt traits for simple types
macro_rules! impl_fmt
{
	( $( $tr:ident ($s:ident, $f:ident) for $ty:ty { $code:expr } )+ ) => { $(
		impl ::core::fmt::$tr for $ty {
			fn fmt(&$s, $f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
				$code
			}
		}
		)+
		}
}

// vim: ft=rust
