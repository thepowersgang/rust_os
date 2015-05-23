// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/handle.rs
//! Opened file interface
use prelude::*;
use super::node::{CacheHandle,NodeType};
use super::Path;

#[derive(Debug)]
/// Open without caring what the file type is (e.g. enumeration)
pub struct Any {
	node: CacheHandle,
}
#[derive(Debug)]
/// Normal file
pub struct File {
	node: CacheHandle,
	mode: FileOpenMode,
}
#[derive(Debug)]
/// Directory (for enumeration)
pub struct Dir {
	node: CacheHandle,
}
#[derive(Debug)]
/// Symbolic link (allows reading the link contents)
pub struct Symlink {
	node: CacheHandle,
}
#[derive(Debug)]
/// Special file (?API exposed)
pub struct Special {
	node: CacheHandle,
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

impl Any
{
	/// Open the specified path (not caring what the actual type is)
	pub fn open(path: &Path) -> super::Result<Any> {
		let node = try!(CacheHandle::from_path(path));
		Ok(Any { node: node })
	}
	
	/// Upgrade the handle to a directory handle
	pub fn to_dir(self) -> super::Result<Dir> {
		if self.node.is_dir() {
			Ok(Dir { node: self.node })
		}
		else {
			Err(super::Error::TypeMismatch)
		}
	}
}

impl File
{
	/// Open the specified path as a file
	pub fn open(path: &Path, mode: FileOpenMode) -> super::Result<File> {
		let node = try!(CacheHandle::from_path(path));
		if !node.is_file() {
			return Err(super::Error::TypeMismatch);
		}
		match mode
		{
		FileOpenMode::SharedRO => {},
		_ => todo!("Acquire lock depending on mode({:?})", mode),
		}
		Ok(File { node: node, mode: mode })
	}
	
	/// Read data from the file at the specified offset
	///
	/// Returns the number of read bytes (which might be less than the size of the input
	/// slice).
	pub fn read(&self, ofs: u64, dst: &mut [u8]) -> super::Result<usize> {
		assert!(self.node.is_file());
		self.node.read(ofs, dst)
	}
}
impl ::core::ops::Drop for File
{
	fn drop(&mut self) {
		match self.mode
		{
		FileOpenMode::SharedRO => {},
		_ => todo!("File::drop() - mode={:?}", self.mode),
		}
		// TODO: For files, we need to release the lock
	}
}

impl Dir
{
	/// Open a provided path as a directory
	pub fn open(path: &Path) -> super::Result<Dir> {
		try!(Any::open(path)).to_dir()
	}
	/// Create a new directory
	pub fn mkdir(&self, name: &str) -> super::Result<Dir> {
		let node = try!(self.node.create(name.as_ref(), NodeType::Dir));
		assert!(node.is_dir());
		Ok( Dir { node: node } )
	}
	/// Create a new symbolic link
	pub fn symlink(&self, name: &str) -> super::Result<()> {
		todo!("Dir::symlink");
	}
}

