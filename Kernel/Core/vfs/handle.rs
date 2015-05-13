// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/handle.rs
//! Opened file interface
use prelude::*;
use super::node::{CacheHandle,NodeType};
use super::Path;

#[derive(Debug)]
pub struct Handle
{
	node: CacheHandle,
}

#[derive(Debug)]
pub enum OpenMode
{
	/// Open without caring what the file type is (e.g. enumeration)
	Any,
	/// Normal file
	File(FileOpenMode),
	/// Directory (for enumeration)
	Dir,
	/// Symbolic link (allows reading the link contents)
	Symlink,
	/// Special file (?API exposed)
	Special,
}
#[derive(Debug)]
pub enum FileOpenMode
{
	/// Shared read-only, multiple readers but no writers visible
	///
	/// When opened in this manner, the file contents cannot change, but it might extend
	SharedRO,
	/// Eclusive read-write, denies any other opens while held (except Append)
	///
	/// No changes to the file will be visible to the user (as the file is locked)
	ExclRW,
	/// Unique read-write, does Copy-on-write to create a new file
	///
	/// No changes to the file will be visible to the user (as it has its own copy)
	UniqueRW,
	/// Append only (allows multiple readers/writers)
	///
	/// Cannot read, all writes go to the end of the file (a write call is atomic)
	Append,
	/// Unsynchronised read-write
	///
	/// No synchronisation at all, fails if any other open type is active.
	Unsynch,
}

impl Handle
{
	pub fn open(path: &Path, mode: OpenMode) -> super::Result<Handle> {
		let node = try!(CacheHandle::from_path(path));
		match mode
		{
		OpenMode::Any => {},
		OpenMode::File(fm) => {
			todo!("Handle::open - mode=File({:?})", fm);
			},
		OpenMode::Dir =>
			if !node.is_dir() {
				return Err( super::Error::TypeMismatch )
			},
		OpenMode::Symlink => {
			todo!("Handle::open - mode=Symlink");
			},
		OpenMode::Special => {
			todo!("Handle::open - mode=Special");
			},
		}
		Ok(Handle { node: node })
	}
	
	// Directory methods
	pub fn mkdir(&self, name: &str) -> super::Result<Handle> {
		let node = try!(self.node.create(name.as_ref(), NodeType::Dir));
		assert!(node.is_dir());
		Ok( Handle { node: node } )
	}
}

impl ::core::ops::Drop for Handle
{
	fn drop(&mut self)
	{
		//todo!("Handle::drop()");
		// TODO: For files, we need to release the lock
	}
}

