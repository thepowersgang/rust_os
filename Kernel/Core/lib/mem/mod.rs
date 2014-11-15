
pub use self::rc::Rc;

mod rc;

#[lang = "owned_box"]
pub struct Box<T>(*mut T);

impl<T: ::core::fmt::Show> ::core::fmt::Show for Box<T>
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::result::Result<(),::core::fmt::FormatError>
	{
		(**self).fmt(f)
	}
}

// vim: ft=rust

