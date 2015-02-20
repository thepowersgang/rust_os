// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/thunk.rs
///! Box<FnOnce> support, similar to std::thunk
use _common::*;

pub trait Invoke<A=(), R=()>
{
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
