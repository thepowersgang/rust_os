//
//
//

pub struct Path(::std::ffi::OsStr);

impl<'a> ::std::fmt::Debug for &'a Path {
	fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		write!(f, "Path({:?})", &self.0 )
	}
}

impl Path
{
}

