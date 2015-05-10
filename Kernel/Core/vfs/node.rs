// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/node.rs
//! VFS vode management
use prelude::*;
use super::Path;
use sync::mutex::LazyMutex;
use lib::byte_str::{ByteStr,ByteString};
use core::atomic::{self,AtomicUsize};

pub type InodeId = u64;

/// Base trait for a VFS node, defines common operation on nodes
pub trait NodeBase: Send {
	/// Return the volume's inode number
	fn get_id(&self) -> InodeId;
}
/// Trait for "File" nodes
pub trait File: NodeBase {
	/// Returns the size (in bytes) of this file
	fn size(&self) -> u64;
}
/// Trait for "Directory" nodes, containers for files.
pub trait Dir: NodeBase {
	/// Acquire a node given the name
	fn lookup(&self, name: &ByteStr) -> Result<InodeId,()>;
}
/// Trait for symbolic link nodes.
pub trait Symlink: NodeBase {
	/// Reads the contents of the symbolic link into a string
	fn read(&self) -> ByteString;
}
/// Trait for special files (e.g. unix device files, named pipes)
pub trait Special: NodeBase {
	/// Returns a string indicating the type of special node
	fn typename(&self) -> &str;
	
	// TODO: Include an API (similar to the syscall API) to communicate with the node
}

/// VFS Node
pub enum Node
{
	File(Box<File>),
	Dir(Box<Dir>),
	Symlink(Box<Symlink>),
	Special(Box<Special>),
}

struct CachedNode
{
	node: Node,
	refcount: AtomicUsize,
}

#[derive(Debug)]
pub struct CacheHandle
{
	mountpt: usize,
	inode: InodeId,
	ptr: *const CachedNode,
}

static S_NODE_CACHE: LazyMutex<::lib::VecMap<(usize,InodeId),Box<CachedNode>>> = lazymutex_init!();

pub fn init()
{
	S_NODE_CACHE.init(|| Default::default());
}

impl CacheHandle
{
	/// Obtain a node handle using a mountpoint ID and inode number
	pub fn from_ids(mountpoint: usize, inode: InodeId) -> super::Result<CacheHandle> {
		use lib::vec_map::Entry;
		let ptr: *const _ = &**match S_NODE_CACHE.lock().entry( (mountpoint, inode) )
			{
			Entry::Occupied(mut e) =>
				{
				e.get_mut().refcount.fetch_add(1, atomic::Ordering::Relaxed);
				e.into_mut()
				},
			Entry::Vacant(e) =>
				match super::mount::Handle::from_id(mountpoint).get_node(inode)
				{
				Some(node) => e.insert(Box::new(CachedNode { node: node, refcount: AtomicUsize::new(1) })),
				None => return Err( super::Error::NotFound ),
				},
			};
		Ok(CacheHandle {
			mountpt: mountpoint,
			inode: inode,
			ptr: ptr,
			})
	}
	
	/// Obtain a node handle using a path
	pub fn from_path(path: &Path) -> super::Result<CacheHandle>
	{
		// TODO: Support path caching?
		
		// Locate mountpoint for the path
		// - This does a longest prefix match on the path
		let (mph,tail) = try!(super::mount::Handle::for_path(path));
		// Acquire the mountpoint's root node
		let mut node_h = try!(CacheHandle::from_ids( mph.id(), mph.root_inode() ));
		// Iterate components of the path that were not consumed by the mountpoint
		for seg in tail
		{
			node_h = match *node_h.as_ref()
				{
				Node::Dir(ref dir) => {
					let next_id = match dir.lookup(seg)
						{
						Ok(v) => v,
						Err(_) => return Err(super::Error::NotFound),
						};
					try!(CacheHandle::from_ids( mph.id(), next_id ))
					},
				Node::Symlink(_) => {
					todo!("Symbolic links")
					},
				_ => return Err(super::Error::NonDirComponent),
				};
		}
		Ok( node_h )
	}
}

impl ::core::convert::AsRef<Node> for CacheHandle
{
	fn as_ref(&self) -> &Node {
		let lh = S_NODE_CACHE.lock();
		// SAFE: While this handle is active, the box will be present
		unsafe {
			let box_r = lh.get( &(self.mountpt, self.inode) ).expect("Cached node with open handle absent");
			let cn_ref: &CachedNode = &**box_r;
			::core::mem::transmute(&cn_ref.node)
		}
	}
}

