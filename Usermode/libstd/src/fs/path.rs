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

	pub fn is_absolute(&self) -> bool {
		self.0.as_bytes().len() > 0 && self.0.as_bytes()[0] == b'/'
	}

	pub fn starts_with<P: AsRef<Path>>(&self, base: P) -> bool {
		let base = base.as_ref();
		self.0.as_bytes().starts_with( base.0.as_bytes() )
	}



	pub fn split_off_first(&self) -> (&::std::ffi::OsStr, &Path) {
		let (a, b) = {
			let mut it = self.0.as_bytes().splitn(2, |&x| x == b'/');
			(it.next().unwrap(), it.next().unwrap_or(&[]))
			};

		(a.as_ref(), Path::new(b))
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

