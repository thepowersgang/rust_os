// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/mod.rs
//! Virtual File System
use crate::prelude::*;
use crate::vfs;
use super::{mount, node};
use crate::metadevs::storage::VolumeHandle;
use crate::lib::{VecMap,SparseVec};
use crate::lib::byte_str::{ByteStr,ByteString};
use crate::lib::mem::aref::{Aref,ArefInner,ArefBorrow};

pub struct Driver;
pub static S_DRIVER: Driver = Driver;

enum RamFile
{
	//File(RamFileFile),
	Dir(RamFileDir),
	Symlink(RamFileSymlink),
}
#[derive(Default)]
struct RamFileDir
{
	ents: crate::sync::RwLock<VecMap<ByteString,usize>>,
}
#[derive(Default)]
struct RamFileSymlink
{
	target: super::PathBuf,
}
//#[derive(Default)]
//struct RamFileFile
//{
//	ofs: usize,
//	size: usize,
//}
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
	nodes: crate::sync::Mutex< SparseVec<Aref<RamFile>> >,
}

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
	fn mount(&self, vol: VolumeHandle, _: mount::SelfHandle) -> super::Result<Box<dyn mount::Filesystem>> {
		let rv = Box::new(RamFS {
			// SAFE: ArefInner must not change addresses, but because you can't move out of a boxed trait, we're good
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
	fn root_inode(&self) -> node::InodeId {
		0
	}
	fn get_node_by_inode(&self, id: node::InodeId) -> Option<node::Node> {
		log_trace!("RamFS::get_node_by_inode({})", id);
		let nodes = self.inner.nodes.lock();
		if id >= nodes.len() as node::InodeId {
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
			RamFile::Dir(_) => Some(node::Node::Dir(fr)),
			RamFile::Symlink(_) => Some(node::Node::Symlink(fr)),
			//RamFile::File(_) => todo!("normal files"),
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
	fn get_id(&self) -> node::InodeId {
		unimplemented!()
	}
	fn get_any(&self) -> &dyn (::core::any::Any) {
		self
	}
}
impl node::Dir for FileRef {
	fn lookup(&self, name: &ByteStr) -> vfs::Result<node::InodeId> {
		let lh = self.dir().ents.read();
		match lh.get(name)
		{
		Some(&v) => Ok(v as node::InodeId),
		None => Err(vfs::Error::NotFound),
		}
	}
	
	fn read(&self, start_ofs: usize, callback: &mut node::ReadDirCallback) -> node::Result<usize> {
		let lh = self.dir().ents.read();
		let mut count = 0;
		// NOTE: This will skip/repeat entries if `create` is called between calls
		for (name, &inode) in lh.iter().skip(start_ofs)
		{
			count += 1;
			if ! callback(inode as node::InodeId, &mut name.as_bytes().iter().cloned()) {
				break ;
			}
		}
		Ok(start_ofs + count)
	}
	
	fn create(&self, name: &ByteStr, nodetype: node::NodeType) -> vfs::Result<node::InodeId> {
		use crate::lib::vec_map::Entry;
		let mut lh = self.dir().ents.write();
		match lh.entry(From::from(name))
		{
		Entry::Occupied(_) => Err(vfs::Error::AlreadyExists),
		Entry::Vacant(e) => {
			let nn = match nodetype
				{
				node::NodeType::Dir  => RamFile::Dir (Default::default()),
				//node::NodeType::File => RamFile::File(Default::default()),
				node::NodeType::File => return Err(vfs::Error::Unknown("TODO: Files")),
				node::NodeType::Symlink(v) =>
					RamFile::Symlink(RamFileSymlink{target: From::from(v)}),
				};
			let inode = self.0.nodes.lock().insert( Aref::new(nn) );
			e.insert(inode);
			Ok(inode as node::InodeId)
			},
		}
	}
	fn link(&self, name: &ByteStr, node: &dyn node::NodeBase) -> vfs::Result<()> {
		todo!("<FileRef as Dir>::link({:?}, inode={})", name, node.get_id())
	}
	fn unlink(&self, name: &ByteStr) -> vfs::Result<()> {
		todo!("<FileRef as Dir>::unlink({:?})", name)
	}
}
impl node::Symlink for FileRef {
	fn read(&self) -> ByteString {
		ByteString::from( ByteStr::new(&*self.symlink().target) )
	}
}

