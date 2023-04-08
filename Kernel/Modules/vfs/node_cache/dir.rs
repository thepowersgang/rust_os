
use ::kernel::prelude::*;
use super::{CacheHandleDir};
use crate as vfs;
use ::core::sync::atomic::{self,AtomicUsize};
use ::kernel::lib::byte_str::{ByteStr};

pub struct CacheNodeInfoDir
{
	/// Filesystem's node handle
	pub fsnode: Box<dyn vfs::node::Dir>,

	pub mountpoint: AtomicUsize,	// 0 is invalid (that's root), so means "no mount"
}
impl CacheNodeInfoDir {
	pub fn new(fsnode: Box<dyn vfs::node::Dir>) -> Self {
		CacheNodeInfoDir {
			fsnode,
			mountpoint: AtomicUsize::new(0),
		}
	}
}

/// Directory methods
impl CacheHandleDir
{
	fn get_info(&self) -> vfs::Result<&CacheNodeInfoDir> {
		match self.0.as_ref()
		{
		&super::CacheNodeInfo::Dir(ref inner) => Ok(inner),
		_ => Err( vfs::Error::Unknown("BUG: CacheHandleDir for non-directory") ),
		}
	}
	pub fn create(&self, name: &ByteStr, ty: vfs::node::NodeType) -> vfs::Result<super::CacheHandle> {
		let inode = self.get_info()?.fsnode.create(name, ty)?;
		Ok( super::CacheHandle::from_ids(self.0.mountpt, inode)? )
	}
	pub fn read_dir(&self, ofs: usize, items: &mut vfs::node::ReadDirCallback) -> vfs::Result<usize> {
		Ok( self.get_info()?.fsnode.read(ofs, items)? )
	}
	pub fn open_child(&self, name: &ByteStr) -> vfs::Result<super::CacheHandle> {
		let inode = self.get_info()?.fsnode.lookup(name)?;
		Ok( super::CacheHandle::from_ids(self.0.mountpt, inode)? )
	}
}
/// Directory methods (mountpoint)
impl CacheHandleDir
{
	pub fn is_mountpoint(&self) -> bool {
		match self.get_info()
		{
		Ok(info) => info.mountpoint.load(atomic::Ordering::Relaxed) != 0,
		_ => false,
		}
	}
	/// Returns `true` if the mount binding succeeded
	pub fn mount(&self, filesystem_id: usize) -> bool {
		match self.get_info()
		{
		Ok(info) => {
			info.mountpoint.compare_exchange(0, filesystem_id, atomic::Ordering::Relaxed, atomic::Ordering::Relaxed).is_ok()
			},
		_ => false,
		}
	}
}
