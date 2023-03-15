
use crate::prelude::*;
use super::CacheHandleFile;
use crate::vfs;

pub struct CacheNodeInfoFile
{
	pub fsnode: Box<dyn vfs::node::File>,
	//mapped_pages: HashMap<u64,FrameHandle>,
	lock_info: crate::sync::Mutex<CacheNodeInfoFileLock>,
	append_lock: crate::sync::Mutex<()>,
}
impl CacheNodeInfoFile {
	pub fn new(fsnode: Box<dyn vfs::node::File>) -> Self {
		CacheNodeInfoFile {
			fsnode,
			lock_info: Default::default(),
			append_lock: Default::default()
		}
	}
}
#[derive(Default)]
enum CacheNodeInfoFileLock {
	/// Nothing has the file open, but references may exist (as `Any` handles)
	#[default]
	Unlocked,
	/// Reader/Append, stores number of open handles
	Shared(usize),
	/// Unique access (only one can exist)
	Unique,
	/// Unsynchronised access, stores the number of open handles
	Unsynch(usize),
}

/// Normal file methods
impl CacheHandleFile
{
	fn get_info(&self) -> vfs::Result<&CacheNodeInfoFile> {
		match self.0.as_ref()
		{
		&super::CacheNodeInfo::File(ref rv) => Ok(rv),
		_ => Err(vfs::Error::InvalidParameter),
		}
	}
	/// Take out a sharable lock on the file
	pub fn file_lock_shared(&self) -> vfs::Result<()> {
		let info = self.get_info()?;
		let mut lh = info.lock_info.lock();
		match *lh {
		CacheNodeInfoFileLock::Unlocked => {
			*lh = CacheNodeInfoFileLock::Shared(1);
			Ok( () )
			}
		CacheNodeInfoFileLock::Shared(ref mut count) => {
			*count += 1;
			Ok( () )
			},
		_ => Err(vfs::Error::Locked),
		}
	}

	/// Valid size = maximum offset in the file
	pub fn get_valid_size(&self) -> u64 {
		self.get_info().map(|v| v.fsnode.size()).unwrap_or(0)
	}
	pub fn read(&self, ofs: u64, dst: &mut [u8]) -> vfs::Result<usize> {
		Ok( self.get_info()?.fsnode.read(ofs, dst)? )
	}
	pub fn write(&self, ofs: u64, src: &[u8]) -> vfs::Result<usize> {
		// TODO: Ensure that the handle is writable?
		Ok( self.get_info()?.fsnode.write(ofs, src)? )
	}
	pub fn append(&self, data: &[u8]) -> vfs::Result<usize> {
		let info = self.get_info()?;
		let _lh = info.append_lock.lock();
		let ofs = info.fsnode.size();
		Ok( info.fsnode.write(ofs, data)? )
	}
}
