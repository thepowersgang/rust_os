//
//
//

use super::path::Path;

pub struct File(::syscalls::vfs::File);

impl File
{
	pub fn open<P: AsRef<Path>>(path: P) -> ::std_io::Result<File> {
		let p = path.as_ref();
		match ::syscalls::vfs::File::open(p, ::syscalls::vfs::FileOpenMode::ReadOnly)
		{
		Ok(f) => Ok( File(f) ),
		Err(e) => Err( From::from(e) ),
		}
	}
}

impl ::io::Read for File
{
	fn read(&mut self, buf: &mut [u8]) -> ::std_io::Result<usize> {
		::io::Read::read( &mut self.0, buf )
	}
}

