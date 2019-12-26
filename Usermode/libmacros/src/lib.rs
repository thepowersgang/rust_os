// Tifflin OS - Usermode Macros
// - By John Hodge (thePowersGang)
//
//! Provides a collection of helpful macros
#![feature(core_intrinsics)]
#![no_std]

#[macro_export]
macro_rules! impl_fmt
{
	(@as_item $($i:item)*) => {$($i)*};

	($( /*$(<($($params:tt)+)>)* */ $tr:ident($s:ident, $f:ident) for $t:ty { $($code:stmt)*} )+) => {
		$(impl_from!{ @as_item
			impl/*$(<$($params)+>)* */ ::std::fmt::$tr for $t {
				fn fmt(&$s, $f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
					$( $code )*
				}
			}
		})+
		};
}

#[macro_export]
macro_rules! impl_conv {
	(@as_item $($i:item)*) => {$($i)*};

	(@match_ $( $(<($($params:tt)+)>)* From<$src:ty>($v:ident) for $t:ty { $($code:stmt)*} )+) => {
		$(impl_from!{ @as_item 
			impl$(<$($params)+>)* ::std::convert::From<$src> for $t {
				fn from($v: $src) -> $t {
					$($code)*
				}
			}
		})+
	};
	(@match_ $( $(<($($params:tt)+)>)* Into<$dst:ty>($self_:ident) for $t:ty { $($code:stmt)*} )+) => {
		$(impl_from!{ @as_item 
			impl$(<$($params)+>)* ::std::convert::Into<$dst> for $t {
				fn into($self_) -> $dst {
					$($code)*
				}
			}
		})+
	};
	(@match_ $( $(<($($params:tt)+)>)* AsRef<$dst:ty>($self_:ident) for $t:ty { $($code:stmt)*} )+) => {
		$(impl_from!{ @as_item 
			impl$(<$($params)+>)* ::std::convert::AsRef<$dst> for $t {
				fn as_ref($self_) -> &$dst {
					$($code)*
				}
			}
		})+
	};

	($( $(<($($params:tt)+)>)* $name:ident<$src:ty>($v:ident) for $t:ty { $($code:stmt)*} )+) => {
		$(impl_conv!{ @match_ $(<($($params:tt)+)>)* $name<$src>($v) for $t { $($code)* } })+
	};
}

#[macro_export]
macro_rules! impl_from {
	(@as_item $($i:item)*) => {$($i)*};

	($( $(<($($params:tt)+)>)* From<$src:ty>($v:ident) for $t:ty { $($code:stmt)*} )+) => {
		$(impl_from!{ @as_item 
			impl$(<$($params)+>)* ::std::convert::From<$src> for $t {
				fn from($v: $src) -> $t {
					$($code)*
				}
			}
		})+
	};
}

pub fn type_name<T: ?::core::marker::Sized>() -> &'static str {
	::core::intrinsics::type_name::<T>()
}
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
