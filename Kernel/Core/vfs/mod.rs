// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/mod.rs
//! Virtual File System
#[allow(unused_imports)]
use prelude::*;
use metadevs::storage::VolumeHandle;

module_define!(VFS, [], init);

pub type Result<T> = ::core::result::Result<T,Error>;

#[derive(Debug)]
pub enum Error
{
	/// File not found
	NotFound,
	/// Permission denied
	PermissionDenied,
	/// File exclusively locked
	Locked,
	/// Node was not the requested type
	TypeMismatch,
	/// A component of the path was not a directory
	NonDirComponent,
	/// Symbolic link recursion limit reached
	RecursionDepthExceeded,
	/// Block-level IO Error
	BlockIoError(::metadevs::storage::IoError),
	/// Path was malformed (too long, not absolute, not normalised, ... depends)
	MalformedPath,
	/// Unknown (misc) error
	Unknown(&'static str),
}
impl From<::metadevs::storage::IoError> for Error {
	fn from(v: ::metadevs::storage::IoError) -> Error {
		Error::BlockIoError(v)
	}
}

pub use self::path::{Path,PathBuf};

pub mod node;
pub mod mount;
pub mod handle;
mod path;
mod ramfs;

fn init()
{
	// 1. Initialise global structures
	mount::init();
	node::init();
	ramfs::init();
	// 2. Start the root/builtin filesystems
	mount::mount("/".as_ref(), VolumeHandle::new_ramdisk(0), "ramfs", &[]).unwrap();//"Unable to mount /");
	// 3. Initialise root filesystem layout
	let root = match handle::Dir::open( Path::new("/") )
		{
		Ok(v) => v,
		Err(e) => panic!("BUG - Opening '/' failed: {:?}", e),
		};
	root.mkdir("system").unwrap();
	root.mkdir("volumes").unwrap();
	root.mkdir("temp").unwrap();
}

