// "Tifflin" Kernel - ext2/3/4 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_extN/file.rs
//! Regular file
use kernel::prelude::*;
use kernel::vfs;

pub struct File
{
	inode: ::inodes::Inode,
}


impl File
{
	pub fn new(inode: ::inodes::Inode) -> File
	{
		File {
			inode: inode,
			}
	}
}

impl vfs::node::NodeBase for File
{
	fn get_id(&self) -> vfs::node::InodeId {
		self.inode.get_id()
	}
}
impl vfs::node::File for File
{
	fn size(&self) -> u64 {
		todo!("size");
	}
	fn truncate(&self, newsize: u64) -> vfs::node::Result<u64> {
		todo!("truncate");
	}
	fn clear(&self, ofs: u64, size: u64) -> vfs::node::Result<()> {
		todo!("clear");
	}
	fn read(&self, ofs: u64, buf: &mut [u8]) -> vfs::node::Result<usize> {
		todo!("read");
	}
	fn write(&self, ofs: u64, buf: &mut [u8]) -> vfs::node::Result<usize> {
		todo!("write");
	}
}

