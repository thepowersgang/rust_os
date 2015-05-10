//
use prelude::*;
use super::node::CacheHandle;
use super::Path;

#[derive(Debug)]
pub struct Handle
{
	node: CacheHandle,
}

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
		todo!("Handle::open()");
	}
}


