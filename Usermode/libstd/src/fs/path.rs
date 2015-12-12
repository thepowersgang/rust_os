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
	pub fn new<S: ?Sized + AsRef<::std::ffi::OsStr>>(s: &S) -> &Path {
		// SAFE: Assume all OsStrs are valid Paths
		unsafe { ::core::mem::transmute(s.as_ref()) }
	}
	pub fn display(&self) -> Display {
		Display(self)
	}
}

pub struct Display<'a>(&'a Path);

impl<'a> ::std::fmt::Display for Display<'a>
{
	fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		let mut b: &[u8] = self.0.as_ref();
		while b.len() > 0
		{
			match ::std::str::from_utf8(b)
			{
			Ok(v) => return ::std::fmt::Display::fmt(v, f),
			Err(e) => {
				let l = e.valid_up_to();
				try!( ::std::fmt::Display::fmt( ::std::str::from_utf8(&b[..l]).unwrap(), f  ) );
				try!( write!(f, "\\u{{?{:#02x}}}", b[l]) );
				b = &b[ l+1 .. ];
				},
			}
		}
		Ok( () )
	}
}

