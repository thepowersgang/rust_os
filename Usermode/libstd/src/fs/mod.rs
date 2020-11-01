//
//
//

pub use self::path::Path;
pub use self::file::File;

//static ROOT_HANDLE: Dir = 
//static DIR_HANDLES: [::syscalls::vfs::Dir; 4] = [
//	];

struct Node(::syscalls::vfs::Node);

impl Node
{
	fn open(path: &Path) -> ::io::Result<Node> {
		// NOTES:
		// - Absolute paths refer to a fake root? (or use Windows-style // paths?)
		if path.is_absolute()
		{
			// - First node is empty, before leadig /
			let (_, path) = path.split_off_first();
			let pb: &[u8] = path.as_ref();
			// If the path DOESN't start with /: - hand off to root
			if pb.len() > 0 && pb[0] != b':' {
				// - Fully absolute path (open readonly relative to the program's root)
				let n = try!( ::syscalls::vfs::root().open_child_path(path) );
				Ok(Node( n ))
			}
			else {
				// - Prefixed path, relative to the handle-set
				let (firstnode, path) = path.split_off_first();
				match &firstnode.as_bytes()[1..]
				{
				//b"AppBin" => 
				//b"AppData" => 
				b"CommonData" => todo!("Load from CommonData - {:?}", path),
				//b"Output" => 
				_ => Err( ::syscalls::vfs::Error::FileNotFound.into() ),
				}
			}
		}
		else {
			// Open relative to CWD
			unimplemented!();
		}
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

