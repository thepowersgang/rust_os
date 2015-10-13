//
//
//

pub struct Path(::std::ffi::OsStr);

impl<'a> ::std::fmt::Debug for &'a Path {
	fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		write!(f, "Path({:?})", &self.0 )
	}
}

impl AsRef<Path> for str {
	fn as_ref(&self) -> &Path {
		// SAFE: Any valid str is a valid Path
		unsafe { ::core::mem::transmute(self) }
	}
}
impl AsRef<[u8]> for Path {
	fn as_ref(&self) -> &[u8] {
		self.0.as_ref()
	}
}
impl AsRef<Path> for Path {
	fn as_ref(&self) -> &Path {
		self
	}
}

impl Path
{
}
