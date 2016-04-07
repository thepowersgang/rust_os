//
//
//

pub use self::path::Path;
pub use self::file::File;

//static ROOT_HANDLE: Dir = 

struct Node(::syscalls::vfs::Node);

impl Node
{
	fn open(path: &Path) -> ::io::Result<Node> {
		unimplemented!();
	}
	
	fn into_file(self) -> ::io::Result<::syscalls::vfs::File> {
		match self.0.into_file(::syscalls::vfs::FileOpenMode::ReadOnly)
		{
		Ok(v) => Ok(v),
		Err(e) => Err( From::from(e) ),
		}
	}
}

mod file;
mod path;

