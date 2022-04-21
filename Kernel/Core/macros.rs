//
//
//
/// Returns true if the passed exppression matches the pattern
#[macro_export]
macro_rules! is
{
	($val:expr, $p:pat) => ( match $val { $p => true, _ => false } );
}

/// Asserts a constant condition at compile-time
#[macro_export]
macro_rules! static_assert {
	($cnd:expr) => { static _STATIC_ASSERT: [(); ($cnd) as usize] = [()]; }
}

#[doc(hidden)]
#[macro_export]
macro_rules! _count
{
	() => {0};
	($a:expr) => {1};
	($a:expr, $($b:expr),+) => {1+_count!($($b),+)};
}

/// Define a kernel module (creates the module header, containg the name and dependency strings)
///
/// For external modules, at the module must be defined in the root (lib.rs)
#[macro_export]
macro_rules! module_define
{
	($name:ident, [ $( $(#[$da:meta])* $deps:ident ),*], $init:path) => (
		#[doc(hidden)]
		#[link_section = ".MODULE_LIST"]
		#[allow(dead_code)]
		pub static S_MODULE: $crate::modules::ModuleInfo = $crate::modules::ModuleInfo {
			name: stringify!($name),
			init: $init,
			deps: &S_DEPS,
			_rsvd: [0,0,0],
		};
		#[doc(hidden)]
		const S_DEPS: &'static [&'static str] = &[$( $(#[$da])* stringify!($deps) ),*];
		// External linkage symbol, to force the module info to be maintained
		#[linkage="external"]
		#[doc(hidden)]
		#[allow(dead_code)]
		pub static S_MODULE_P: &$crate::modules::ModuleInfo = &S_MODULE;
	);
}

/// Ensure that a type implments the provided trait
///
/// Useful for "Send" and "Sync"
#[macro_export]
macro_rules! assert_trait
{
	($t:ty : $tr:ident) => { #[allow(warnings)] fn assert_trait<T: $tr>() { } #[allow(dead_code)] fn call_assert_trait() { assert_trait::<$t>() } }
}

#[doc(hidden)]
//#[cfg_attr(not(test_shim),is_safe(irq))]
pub fn type_name<T: ?::core::marker::Sized>() -> &'static str {
	::core::any::type_name::<T>()
}
/// A safe wrapper around the `type_name` intrinsic
#[macro_export]
macro_rules! type_name
{
	($t:ty) => ( $crate::macros::type_name::<$t>() );
}



/// Iterator helper, desugars to a.zip(b)
#[macro_export]
macro_rules! zip
{
	($a:expr, $b:expr) => ( $a.zip($b) );
}
/// Iterator helper, desugars to a.chain(b).chain(b2)
#[macro_export]
macro_rules! chain
{
	($a:expr, $($b:expr),+) => ( $a$(.chain($b))+ );
}

/// Provides a short and noticable "TODO: " message
#[macro_export]
macro_rules! todo
{
	( $s:expr ) => ( panic!( concat!("TODO: ",$s) ) );
	( $s:expr, $($v:tt)* ) => ( panic!( concat!("TODO: ",$s), $($v)* ) );
}

/// Provides a less boiler-plate way to implement fmt traits for simple types
///
/// Only supports non-generic types and unbounded types (due to challenges in matching generic definitions)
///
/// ```
/// impl_fmt! {
///     Debug(self, f) for Type {
///         write!(f, "Hello world!")
///     }
///     <T> Display(self, f) for Container {
///         write!(f, "Hello world!")
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_fmt
{
	( $( <$($g:ident),+> $tr:ident ($s:ident, $f:ident) for $ty:ty { $code:expr } )+ ) => { $(
		impl<$($g),+> ::core::fmt::$tr for $ty {
			fn fmt(&$s, $f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
				$code
			}
		}
		)+
		};
	
	( $( $tr:ident ($s:ident, $f:ident) for $ty:ty { $code:expr } )+ ) => { $(
		impl ::core::fmt::$tr for $ty {
			fn fmt(&$s, $f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
				$code
			}
		}
		)+
		};
}

/// Implements the From trait for the provided type, avoiding boilerplate
#[macro_export]
macro_rules! impl_from {
	(@as_item $($i:item)*) => {$($i)*};

	($( $(<($($params:tt)+)>)* From<$src:ty>($v:ident) for $t:ty { $($code:stmt)*} )+) => {
		$(impl_from!{ @as_item 
			impl$(<$($params)+>)* ::core::convert::From<$src> for $t {
				fn from($v: $src) -> $t {
					$($code)*
				}
			}
		})+
	};
}

// NOTE: This should really be in ::threads::wait_queue, but it also needs to be early in parse
/// Wait on a wait queue contained within a spinlock
///
/// Due to lifetime issues, the more erganomical `lock.queue.wait(lock)` does not pass borrow checking.
#[macro_export]
macro_rules! waitqueue_wait_ext {
	($lock:expr, $(.$field:ident)+) => ({
		let mut lock: $crate::arch::sync::HeldSpinlock<_> = $lock;
		let irql = lock$(.$field)+.wait_int();
		::core::mem::drop(lock);
		::core::mem::drop(irql);
		$crate::threads::reschedule();
		});
}

/// Override libcore's `try!` macro with one that backs onto `From`
#[macro_export]
macro_rules! r#try {
	($e:expr) => (
		match $e {
		Ok(v) => v,
		Err(e) => return Err(From::from(e)),
		}
		);
}


/// Initialise a static Mutex
#[macro_export]
macro_rules! mutex_init{ ($val:expr) => ($crate::sync::mutex::Mutex::new($val)) }
/// Initialise a static LazyMutex
#[macro_export]
macro_rules! lazymutex_init{ () => ($crate::sync::mutex::LazyMutex::new())}

// vim: ft=rust
