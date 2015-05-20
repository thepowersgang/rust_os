// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/fs_fat/dir.rs
use kernel::prelude::*;
use kernel::lib::mem::aref::ArefBorrow;
use kernel::vfs::{self,node};

pub type FilesystemInner = super::FilesystemInner;

pub struct FileNode
{
	fs: ArefBorrow<FilesystemInner>,
	parent_dir: u32,
	first_cluster: u32,
	size: u32,
}

impl FileNode
{
	pub fn new_boxed(fs: ArefBorrow<FilesystemInner>, parent: u32, first_cluster: u32, size: u32) -> Box<FileNode> {	
		Box::new(FileNode {
			fs: fs,
			parent_dir: parent,
			first_cluster: first_cluster,
			size: size,
			})
	}
}
impl node::NodeBase for FileNode {
	fn get_id(&self) -> node::InodeId {
		todo!("")
	}
}
impl node::File for FileNode {
	fn size(&self) -> u64 {
		self.size as u64
	}
	fn truncate(&self, newsize: u64) -> node::Result<u64> {
		todo!("FileNode::truncate");
	}
	fn clear(&self, ofs: u64, size: u64) -> node::Result<()> {
		todo!("FileNode::clear");
	}
	fn read(&self, ofs: u64, buf: &mut [u8]) -> node::Result<usize> {
		todo!("FileNode::read");
	}
	/// Write data to the file, can only grow the file if ofs==size
	fn write(&self, ofs: u64, buf: &mut [u8]) -> node::Result<usize> {
		todo!("FileNode::write");
	}
}

