// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/mod.rs
//! Virtual File System
use prelude::*;
use super::{mount, node};
use super::node::Result as IoResult;
use super::node::{Node,InodeId};
use metadevs::storage::VolumeHandle;
use lib::{BTreeMap,SparseVec};
use lib::byte_str::{ByteStr,ByteString};

struct Driver;

enum RamFile
{
	Dir(RamFileDir),
	Symlink(RamFileSymlink),
}
struct RamFileDir
{
	ents: BTreeMap<String,usize>,
}
struct RamFileSymlink
{
	target: String,
}
struct FileRef(usize);

struct RamFS
{
	_vh: VolumeHandle,
	// TODO: Store as much data (and metadata) as possible on the volume
	nodes: SparseVec<RamFile>,
}

static S_DRIVER: Driver = Driver;

pub fn init()
{
	let h = mount::DriverRegistration::new("ramfs", &S_DRIVER);
	unsafe { ::core::mem::forget(h); }
}

impl mount::Driver for Driver
{
	fn detect(&self, _vol: &VolumeHandle) -> usize {
		// RAMFS should never bind to an arbitary volume
		0
	}
	fn mount(&self, vol: VolumeHandle) -> Result<Box<mount::Filesystem>, ()> {
		Ok( Box::new(RamFS {
			_vh: vol,
			nodes: Default::default(),
			}) )
	}
}

impl mount::Filesystem for RamFS
{
	fn root_inode(&self) -> InodeId {
		0
	}
	fn get_node_by_inode(&self, id: InodeId) -> Option<Node> {
		if id >= self.nodes.len() as InodeId {
			None
		}
		else {
			let fr = Box::new(FileRef(id as usize));
			match self.nodes[id as usize]
			{
			RamFile::Dir(_) => Some(Node::Dir(fr)),
			RamFile::Symlink(_) => Some(Node::Symlink(fr)),
			}
		}
	}
}

impl node::NodeBase for FileRef {
	fn get_id(&self) -> InodeId {
		self.0 as InodeId
	}
}
impl node::Dir for FileRef {
	fn lookup(&self, name: &ByteStr) -> IoResult<InodeId> {
		unimplemented!()
	}
	
	fn read(&self, ofs: usize, items: &mut [(InodeId,ByteString)]) -> IoResult<(usize,usize)> {
		unimplemented!()
	}
	
	fn create(&self, name: &ByteStr, nodetype: node::NodeType) -> IoResult<InodeId> {
		unimplemented!()
	}
	fn link(&self, name: &ByteStr, inode: InodeId) -> IoResult<()> {
		unimplemented!()
	}
	fn unlink(&self, name: &ByteStr) -> IoResult<()> {
		unimplemented!()
	}
}
impl node::Symlink for FileRef {
	fn read(&self) -> ByteString {
		unimplemented!()
	}
}

