//
//
//

use super::path::Path;

pub struct File(::syscalls::vfs::File);

impl File
{
	pub fn open<P: AsRef<Path>>(path: P) -> ::io::Result<File> {
		let p = path.as_ref();
		Ok( File(super::Node::open(p)?.into_file()?) )
	}
}

impl ::io::Read for File
{
	fn read(&mut self, buf: &mut [u8]) -> ::io::Result<usize> {
		::io::Read::read( &mut self.0, buf )
	}
}

