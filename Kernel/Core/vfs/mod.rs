// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/mod.rs
//! Virtual File System
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
	/// Unknown (misc) error
	Unknown(&'static str),
}

pub use self::path::Path;

pub mod node;
pub mod mount;
mod handle;
mod path;
mod ramfs;

fn init()
{
	// 1. Initialise global structures
	mount::init();
	node::init();
	ramfs::init();
	// 2. Start the root/builtin filesystems
	mount::mount("/".as_ref(), VolumeHandle::ramdisk(0), "ramfs", &[]).unwrap();//"Unable to mount /");
	// 3. Initialise root filesystem layout
	let root = match handle::Handle::open( Path::new("/"), handle::OpenMode::Dir )
		{
		Ok(v) => v,
		Err(e) => panic!("BUG - Opening '/' failed: {:?}", e),
		};
	root.mkdir("system").unwrap();
	root.mkdir("volumes").unwrap();
	root.mkdir("temp").unwrap();
	
	// TODO: mount as directed by command line params SYSDISK and SYSROOT
	
	let h = handle::Handle::open( Path::new("/system"), handle::OpenMode::Any );
	log_debug!("VFS open test = {:?}", h);
}
