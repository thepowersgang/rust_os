
use core::kinds::{Sized,Send};

pub use self::rc::Rc;

mod rc;

#[lang = "owned_box"]
pub struct Box<T>(*mut T);

unsafe impl<T: ?Sized+Send> Send for Box<T>
{
}

impl<T: ?Sized> ::core::fmt::Show for Box<T>
where
	T: ::core::fmt::Show
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::result::Result<(),::core::fmt::Error>
	{
		(**self).fmt(f)
	}
}

// vim: ft=rust

