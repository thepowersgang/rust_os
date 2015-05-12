// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/mod.rs
//! Virtual File System
use prelude::*;
use super::{mount, node};
use super::node::Result as IoResult;
use super::node::{Node,InodeId,IoError};
use metadevs::storage::VolumeHandle;
use lib::{BTreeMap,SparseVec};
use lib::byte_str::{ByteStr,ByteString};
use lib::mem::Arc;

struct Driver;

enum RamFile
{
	Dir(RamFileDir),
	Symlink(RamFileSymlink),
}
#[derive(Default)]
struct RamFileDir
{
	ents: ::sync::RwLock<BTreeMap<ByteString,usize>>,
}
struct RamFileSymlink
{
	target: String,
}
struct FileRef(*const RamFS,Arc<RamFile>);

struct RamFS
{
	_vh: VolumeHandle,
	// TODO: Store as much data (and metadata) as possible on the volume
	nodes: ::sync::Mutex< SparseVec<Arc<RamFile>> >,
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
		let mut rv = Box::new(RamFS {
			_vh: vol,
			nodes: Default::default(),
			});
		let root_inode = rv.nodes.lock().insert( Arc::new(RamFile::Dir(Default::default())) );
		assert_eq!(root_inode, 0);
		Ok(rv)
	}
}

impl mount::Filesystem for RamFS
{
	fn root_inode(&self) -> InodeId {
		0
	}
	fn get_node_by_inode(&self, id: InodeId) -> Option<Node> {
		log_trace!("RamFS::get_node_by_inode({})", id);
		if id >= self.nodes.len() as InodeId {
			log_log!("RamFile::get_node_by_inode - Inode {} out of range", id);
			None
		}
		else {
			let fr = Box::new(FileRef(self.nodes[id as usize].clone()));
			match *self.nodes[id as usize]
			{
			RamFile::Dir(_) => Some(Node::Dir(fr)),
			RamFile::Symlink(_) => Some(Node::Symlink(fr)),
			}
		}
	}
}

impl FileRef {
	fn dir(&self) -> &RamFileDir {
		match &*self.0
		{
		&RamFile::Dir(ref e) => e,
		_ => panic!("Called FileRef::dir() on non-dir"),
		}
	}
}
impl node::NodeBase for FileRef {
	fn get_id(&self) -> InodeId {
		unimplemented!()
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
		use lib::btree_map::Entry;
		let mut lh = self.dir().ents.write();
		match lh.entry(From::from(name))
		{
		Entry::Occupied(_) => Err(IoError::AlreadyExists),
		Entry::Vacant(e) => {
			unimplemented!(); /*
			let inode = self.vol
			match nodetype
			{
			node::NodeType::Dir  => e.insert(RamFile::Dir (Default::default())),
			node::NodeType::File => e.insert(RamFile::File(Default::default())),
			}
			// */
			},
		}
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

