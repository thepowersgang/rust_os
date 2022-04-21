// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/mod.rs
//! Virtual File System
#[allow(unused_imports)]
use crate::prelude::*;
use crate::metadevs::storage::VolumeHandle;

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
	/// The item already exists
	AlreadyExists,

	/// Path was malformed (too long, not absolute, not normalised, ... depends)
	MalformedPath,
	/// A general parameter was malformed (empty filename, offset out of range, ...)
	InvalidParameter,
	/// Node was not the requested type (or selected FS driver doesn't support that volume)
	TypeMismatch,
	/// A component of the path was not a directory
	NonDirComponent,
	/// Symbolic link recursion limit reached
	RecursionDepthExceeded,


	/// Block-level IO Error
	BlockIoError(crate::metadevs::storage::IoError),
	/// Filesystem is read-only
	ReadOnlyFilesystem,
	/// Filesystem driver hit an internal consistency error
	InconsistentFilesystem,
	/// Volume ran out of space
	OutOfSpace,

	/// System has run out of memory
	OutOfMemory,

	/// Operation failed due to a transient error, can can be retried
	TransientError,

	/// Unknown (misc) error
	Unknown(&'static str),
}
impl From<crate::metadevs::storage::IoError> for Error {
	fn from(v: crate::metadevs::storage::IoError) -> Error {
		Error::BlockIoError(v)
	}
}
//impl_fmt! {
//	Display(self, f) for Error {
//		match self
//		{
//		&Error::NotFound => "File not found",
//		&Error::PermissionDenied => "Permission denied",
//		}
//	}
//}

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
	mount::mount("/".as_ref(), VolumeHandle::new_ramdisk(0), "ramfs", &[]).expect("Unable to mount /");
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

