//
//
//
#![macro_escape]
pub use self::queue::Queue;
pub use self::vec::Vec;

pub mod clone;

pub mod mem;
pub mod queue;
pub mod vec;

pub mod num
{
	pub fn round_up(val: uint, target: uint) -> uint
	{
		return (val + target-1) / target * target;
	}
}

pub mod collections
{
	pub trait MutableSeq<T>
	{
		fn push(&mut self, t: T);
		fn pop(&mut self) -> ::core::option::Option<T>;
	}
}

#[macro_export]
macro_rules! tern(
	($cnd:expr ? $ok:expr : $nok:expr) => (if $cnd { $ok } else { $nok })//,
//	($cnd:expr ? $ok:expr : $($cnd2:expr ? $val2:tt :)* $false:expr ) => (if $cnd { $ok } $(else if $cnd2 { $val2 })* else { $false })
	)

// vim: ft=rust

