// "Tifflin" Kernel - ext2/3/4 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_extN/dir.rs
//! Directory handling
use kernel::prelude::*;
use kernel::vfs;
use kernel::lib::byte_str::ByteStr;


pub struct Dir
{
	inode: ::inodes::Inode,
}


impl Dir
{
	pub fn new(inode: ::inodes::Inode) -> Dir
	{
		Dir {
			inode: inode,
			}
	}
}

impl vfs::node::NodeBase for Dir
{
	fn get_id(&self) -> vfs::node::InodeId {
		self.inode.get_id()
	}
}
impl vfs::node::Dir for Dir
{
	fn lookup(&self, name: &ByteStr) -> vfs::node::Result<vfs::node::InodeId> {
		todo!("");
	}
	fn read(&self, start_ofs: usize, callback: &mut vfs::node::ReadDirCallback) -> vfs::node::Result<usize> {
		todo!("");
	}
	fn create(&self, name: &ByteStr, nodetype: vfs::node::NodeType) -> vfs::node::Result<vfs::node::InodeId> {
		todo!("");
	}
	fn link(&self, name: &ByteStr, inode: vfs::node::InodeId) -> vfs::node::Result<()> {
		todo!("");
	}
	fn unlink(&self, name: &ByteStr) -> vfs::node::Result<()> {
		todo!("");
	}
}
