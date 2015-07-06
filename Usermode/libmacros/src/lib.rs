#![feature(core,no_std,core_intrinsics)]
#![no_std]
extern crate core;

#[macro_export]
macro_rules! impl_fmt
{
	( $( <$($g:ident),+> $tr:ident ($s:ident, $f:ident) for $ty:ty { $($code:stmt)* } )+ ) => { $(
		impl<$($g),+> ::std::fmt::$tr for $ty {
			fn fmt(&$s, $f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
				$( $code )*
			}
		}
		)+
		};
	
	( $( $tr:ident ($s:ident, $f:ident) for $ty:ty { $($code:stmt)* } )+ ) => { $(
		impl ::std::fmt::$tr for $ty {
			fn fmt(&$s, $f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
				$( $code )*
			}
		}
		)+
		};
}

#[macro_export]
macro_rules! impl_from {
	($(From<$src:ty>($v:ident) for $t:ty { $($code:stmt)*} )+) => {
		$(
			impl ::std::convert::From<$src> for $t {
				fn from($v: $src) -> $t {
					$($code)*
				}
			}
		)+
	}
}

pub fn type_name<T: ?::core::marker::Sized>() -> &'static str { unsafe { ::core::intrinsics::type_name::<T>() } }
#[macro_export]
macro_rules! type_name {
	($t:ty) => ( $crate::type_name::<$t>() );
}

#[macro_export]
macro_rules! todo
{
	( $s:expr ) => ( panic!( concat!("TODO: ",$s) ) );
	( $s:expr, $($v:tt)* ) => ( panic!( concat!("TODO: ",$s), $($v)* ) );
}

/// Override libcore's `try!` macro with one that backs onto `From`
#[macro_export]
macro_rules! try {
	($e:expr) => (
		match $e {
		Ok(v) => v,
		Err(e) => return Err(From::from(e)),
		}
		);
}
