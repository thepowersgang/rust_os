// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/thunk.rs
//! Box<FnOnce> support, similar to std::thunk
use prelude::*;

/// Trait that provides a consuming invoke function for boxed closures
pub trait Invoke<A=(), R=()>
{
	/// Call the wrapped closure
	fn invoke(self: Box<Self>, arg: A) -> R;
}

impl<A,R,F> Invoke<A,R> for F
	where F : FnOnce(A) -> R
{
	fn invoke(self: Box<F>, arg: A) -> R {
		let f = *self;
		f(arg)
	}
}
