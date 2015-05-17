// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/fs_fat/dir.rs
use kernel::prelude::*;
use kernel::lib::mem::aref::ArefBorrow;
use kernel::vfs::{self,node};
use kernel::lib::byte_str::{ByteStr,ByteString};

pub type Filesystem = super::Filesystem;

pub struct DirNode
{
	fs: ArefBorrow<super::FilesystemInner>,
	start_cluster: u32,
	// - Uses the cluster chain
}
pub struct RootDirNode
{
	fs: ArefBorrow<super::FilesystemInner>,
}

struct DirCluster<'a>
{
	buf: &'a mut [u8],
}
struct DirEnts<'a>
{
	cluster: DirCluster<'a>,
	ofs: usize,
}

/// Directory entry returned by the DirEnts iterator
enum DirEnt {
	End,
	Empty,
	Short(DirEntShort),
	Long(DirEntLong),
}
struct DirEntShort {
	/// NUL-padded string with extention joined
	name: [u8; 11+1],
	cluster: u32,
	size: u32,
	attributes: u8,
	//creation_time: ::kernel::time::Timestamp,
	//modified_time: ::kernel::time::Timestamp,
	//accessed_time: ::kernel::time::Timestamp,
}
struct DirEntLong {
	id: u8,
	_type: u8,
	chars: [u16; 13],
}

impl DirNode {
	pub fn new(fs: &Filesystem, start_cluster: u32) -> DirNode {
		DirNode {
			fs: fs.inner.borrow(),
			start_cluster: start_cluster,
		}
	}
}
impl RootDirNode {
	pub fn new(fs: &Filesystem) -> RootDirNode {
		RootDirNode {
			fs: fs.inner.borrow(),
		}
	}
}

impl node::NodeBase for DirNode {
	fn get_id(&self) -> node::InodeId {
		todo!("")
	}
}
impl node::NodeBase for RootDirNode {
	fn get_id(&self) -> node::InodeId {
		todo!("")
	}
}

impl node::Dir for DirNode {
	fn lookup(&self, name: &ByteStr) -> node::Result<node::InodeId> {
		todo!("DirNode::lookup");
	}
	fn read(&self, ofs: usize, items: &mut [(node::InodeId,ByteString)]) -> node::Result<(usize,usize)> {
		todo!("DirNode::read");
	}
	fn create(&self, name: &ByteStr, nodetype: node::NodeType) -> node::Result<node::InodeId> {
		todo!("DirNode::create");
	}
	fn link(&self, name: &ByteStr, inode: node::InodeId) -> node::Result<()> {
		todo!("DirNode::link");
	}
	fn unlink(&self, name: &ByteStr) -> node::Result<()> {
		todo!("DirNode::unlink");
	}
}
impl node::Dir for RootDirNode {
	fn lookup(&self, name: &ByteStr) -> node::Result<node::InodeId> {
		todo!("DirNode::lookup");
	}
	fn read(&self, ofs: usize, items: &mut [(node::InodeId,ByteString)]) -> node::Result<(usize,usize)> {
		todo!("DirNode::read");
	}
	fn create(&self, name: &ByteStr, nodetype: node::NodeType) -> node::Result<node::InodeId> {
		todo!("DirNode::create");
	}
	fn link(&self, name: &ByteStr, inode: node::InodeId) -> node::Result<()> {
		todo!("DirNode::link");
	}
	fn unlink(&self, name: &ByteStr) -> node::Result<()> {
		todo!("DirNode::unlink");
	}
}

