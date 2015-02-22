
use core::marker::{Sized,Send};

pub use self::rc::Rc;
pub use self::rc::Arc;

mod rc;

#[lang = "owned_box"]
pub struct Box<T>(*mut T);

unsafe impl<T: ?Sized+Send> Send for Box<T> { }

impl<T> Box<T>
{
	pub fn new(v: T) -> Box<T> {
		box v
	}
}

impl<T: ?Sized> ::core::fmt::Debug for Box<T>
where
	T: ::core::fmt::Debug
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::result::Result<(),::core::fmt::Error>
	{
		(**self).fmt(f)
	}
}

// vim: ft=rust

