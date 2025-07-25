// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/path.rs
//! `Path` type and helpers
#[allow(unused_imports)]
use ::kernel::prelude::*;
use ::kernel::lib::byte_str::{ByteStr,ByteString};

#[derive(Eq,PartialEq,PartialOrd,Ord)]
pub struct Path(ByteStr);

#[derive(Eq,PartialEq,PartialOrd,Ord,Default)]
pub struct PathBuf(ByteString);

::kernel::impl_fmt! {
	Debug(self,f) for Path {
		write!(f, "Path({:?})", &self.0)
	}
	Debug(self,f) for PathBuf {
		write!(f, "PathBuf({:?})", &*self.0)
	}
}

pub struct Components<'a>(&'a Path);

impl Path
{
	/// Create a new path from a byte string
	pub fn new<T: ?Sized + AsRef<[u8]>>(v: &T) -> &Path {
		// SAFE: Path is a wrapper around [u8]
		unsafe { ::core::mem::transmute(v.as_ref()) }
	}
	
	/// Determines if the path is absolute
	pub fn is_absolute(&self) -> bool {
		if self.0.len() > 0 {
			self.0.as_bytes()[0] == b'/'
		}
		else {
			false
		}
	}
	pub fn is_normalised(&self) -> bool {
		// If none of the components are .. or ., it's normalised
		!self.iter().any(|c| c == ".." || c == ".")
	}
	
	pub fn iter(&self) -> Components<'_> {
		Components(self)
	}

	pub fn file_name(&self) -> Option<&ByteStr> {
		self.split_off_last().map(|(_p,f)| f)
	}
	pub fn parent(&self) -> Option<&Path> {
		self.split_off_last().map(|(p,_f)| p)
	}
	
	/// Return the last element of the path, and the remainder
	pub fn split_off_last(&self) -> Option<(&Path, &ByteStr)> {
		if self.0.len() == 0 {
			None
		}
		else {
			let mut i = self.0.as_bytes().rsplitn(2, |&c| c == b'/');
			let filename = i.next().unwrap();
			let parent = match i.next()
				{
				None => Path::new(""),
				Some(rest) if rest == b"" => Path::new("/"),
				Some(rest) => {
					assert!(i.next().is_none());
					Path::new(rest)
					},
				};
			Some( (parent, ByteStr::new(filename), ) )
		}
	}
	/// Return the first element of the path, and the remainder
	pub fn split_off_first(&self) -> Option<(&ByteStr, &Path)> {
		if self.0.len() == 0 {
			None
		}
		else {
			let mut i = self.0.as_bytes().splitn(2, |&c| c == b'/');
			let first = i.next().unwrap();
			match i.next()
			{
			Some(rest) => {
				assert!(i.next().is_none());
				Some( (ByteStr::new(first), Path::new(rest)) )
				},
			None => Some( (ByteStr::new(first), Path::new("")) ),
			}
		}
	}
	
	/// Returns Some(remainder) if this path starts with another path
	pub fn starts_with<P: AsRef<Path>>(&self, other: P) -> Option<&Path> {
		let other: &Path = other.as_ref();
		log_trace!("Path::starts_with(self={:?}, other={:?})", self, other);
		if self.is_absolute() != other.is_absolute() {
			None
		}
		else {
			let mut tail = self;
			let mut oi = other.iter();
			while let Some( (comp,t) ) = tail.split_off_first()	
			{
				log_trace!("tail={:?} :: comp={:?}, t={:?}", tail, comp, t);
				if let Some(ocomp) = oi.next() {
					log_trace!("ocomp={:?}, comp={:?}", ocomp, comp);
					if comp != ocomp {
						return None;
					}
					tail = t;
				}
				else {
					return Some(tail);
				}
			}
			if oi.next().is_some() {
				None
			}
			else {
				Some(Path::new(""))
			}
		}
	}
}

impl AsRef<[u8]> for Path {
	fn as_ref(&self) -> &[u8] {
		self.0.as_ref()
	}
}
impl AsRef<ByteStr> for Path {
	fn as_ref(&self) -> &ByteStr {
		&self.0
	}
}
impl AsRef<Path> for str {
	fn as_ref(&self) -> &Path {
		Path::new(self)
	}
}
impl AsRef<Path> for String {
	fn as_ref(&self) -> &Path {
		Path::new(self)
	}
}

impl PathBuf
{
}
impl<'a> From<&'a Path> for PathBuf {
	fn from(v: &Path) -> PathBuf {
		PathBuf(v.0.to_owned())
	}
}
impl ::core::ops::Deref for PathBuf {
	type Target = Path;
	fn deref(&self) -> &Path {
		Path::new( &self.0 )
	}
}
impl AsRef<Path> for PathBuf {
	fn as_ref(&self) -> &Path {
		Path::new( &self.0 )
	}
}

impl<'a> ::core::iter::IntoIterator for &'a Path {
	type IntoIter = Components<'a>;
	type Item = <Self::IntoIter as Iterator>::Item;
	fn into_iter(self) -> Components<'a> {
		self.iter()
	}
}

impl<'a> ::core::iter::Iterator for Components<'a>
{
	type Item = &'a ByteStr;
	
	fn next(&mut self) -> Option<Self::Item> {
		match self.0.split_off_first()
		{
		Some( (v,t) ) => {
			self.0 = t;
			Some(v)
			},
		None => None,
		}
	}
}

