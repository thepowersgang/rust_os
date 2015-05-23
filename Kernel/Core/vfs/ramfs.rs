// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/mod.rs
//! Virtual File System
use prelude::*;
use super::{mount, node};
use super::node::Result as IoResult;
use super::node::{Node,InodeId,IoError};
use metadevs::storage::{self,VolumeHandle};
use lib::{VecMap,SparseVec};
use lib::byte_str::{ByteStr,ByteString};
use lib::mem::aref::{Aref,ArefInner,ArefBorrow};

struct Driver;

enum RamFile
{
	File(RamFileFile),
	Dir(RamFileDir),
	Symlink(RamFileSymlink),
}
#[derive(Default)]
struct RamFileDir
{
	ents: ::sync::RwLock<VecMap<ByteString,usize>>,
}
#[derive(Default)]
struct RamFileSymlink
{
	target: super::PathBuf,
}
#[derive(Default)]
struct RamFileFile
{
	ofs: usize,
	size: usize,
}
struct FileRef(ArefBorrow<RamFSInner>,ArefBorrow<RamFile>);

struct RamFS
{
	inner: ArefInner<RamFSInner>,
}
struct RamFSInner
{
	_vh: VolumeHandle,
	// TODO: Store as much data (and metadata) as possible on the volume
	// - Possibly by using an allocation pool backed onto the volume
	nodes: ::sync::Mutex< SparseVec<Aref<RamFile>> >,
}

static S_DRIVER: Driver = Driver;

pub fn init()
{
	let h = mount::DriverRegistration::new("ramfs", &S_DRIVER);
	::core::mem::forget(h);
}

impl mount::Driver for Driver
{
	fn detect(&self, _vol: &VolumeHandle) -> super::Result<usize> {
		// RAMFS should never bind to an arbitary volume
		Ok(0)
	}
	fn mount(&self, vol: VolumeHandle) -> super::Result<Box<mount::Filesystem>> {
		let rv = Box::new(RamFS {
			// SAFE: ArefInner must not change addresses, but because you can't move out
			// of a boxed trait, we're good
			inner: unsafe { ArefInner::new( RamFSInner {
				_vh: vol,
				nodes: Default::default(),
				}) },
			});
		let root_inode = rv.inner.nodes.lock().insert( Aref::new(RamFile::Dir(Default::default())) );
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
		let nodes = self.inner.nodes.lock();
		if id >= nodes.len() as InodeId {
			log_log!("RamFile::get_node_by_inode - Inode {} out of range", id);
			None
		}
		else {
			let fr = Box::new(FileRef(
				self.inner.borrow(),
				nodes[id as usize].borrow()
				));
			match *nodes[id as usize]
			{
			RamFile::Dir(_) => Some(Node::Dir(fr)),
			RamFile::Symlink(_) => Some(Node::Symlink(fr)),
			RamFile::File(_) => todo!("normal files"),
			}
		}
	}
}

impl FileRef {
	fn dir(&self) -> &RamFileDir {
		match &*self.1
		{
		&RamFile::Dir(ref e) => e,
		_ => panic!("Called FileRef::dir() on non-dir"),
		}
	}
	fn symlink(&self) -> &RamFileSymlink {
		match &*self.1
		{
		&RamFile::Symlink(ref e) => e,
		_ => panic!("Called FileRef::symlink() on non-symlink"),
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
		use lib::vec_map::Entry;
		let mut lh = self.dir().ents.write();
		match lh.entry(From::from(name))
		{
		Entry::Occupied(_) => Err(IoError::AlreadyExists),
		Entry::Vacant(e) => {
			let nn = match nodetype
				{
				node::NodeType::Dir  => RamFile::Dir (Default::default()),
				node::NodeType::File => RamFile::File(Default::default()),
				node::NodeType::Symlink(v) =>
					RamFile::Symlink(RamFileSymlink{target: From::from(v)}),
				};
			let inode = self.0.nodes.lock().insert( Aref::new(nn) );
			e.insert(inode);
			Ok(inode as InodeId)
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
		ByteString::from( ByteStr::new(&*self.symlink().target) )
	}
}

