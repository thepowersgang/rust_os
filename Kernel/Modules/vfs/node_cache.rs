// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/node_cache.rs
//! VFS node cache
use ::kernel::prelude::*;
use super::Path;
use super::node::InodeId;
use ::kernel::sync::mutex::LazyMutex;
use ::kernel::lib::byte_str::{ByteStr,ByteString};
use ::core::sync::atomic::{self,AtomicUsize};

static S_NODE_CACHE: LazyMutex<::kernel::lib::VecMap<(usize,InodeId),Box<CachedNode>>> = lazymutex_init!();

mod file;
mod dir;

pub fn init()
{
	S_NODE_CACHE.init(|| Default::default());
}

#[derive(Debug,PartialEq)]
pub enum NodeClass {
	File,
	Dir,
	Symlink,
	Special,
}

enum CacheNodeInfo
{
	File(file::CacheNodeInfoFile),
	Dir(dir::CacheNodeInfoDir),
	Symlink {
		target: ByteString,
		/// Filesystem's node handle
		fsnode: Box<dyn super::node::Symlink>
		},
	Special {
		/// Filesystem's node handle
		fsnode: Box<dyn super::node::Special>
		},
}
impl_from!{
	From<super::node::Node>(v) for CacheNodeInfo {
		match v
		{
		super::node::Node::File(f) => CacheNodeInfo::File(file::CacheNodeInfoFile::new(f)),
		super::node::Node::Dir(f) => CacheNodeInfo::Dir(dir::CacheNodeInfoDir::new(f)),
		super::node::Node::Symlink(f) => CacheNodeInfo::Symlink { target: f.read(), fsnode: f },
		super::node::Node::Special(f) => CacheNodeInfo::Special { fsnode: f },
		}
	}
}

struct CachedNode
{
	/// Number of outstanding references to this node
	refcount: AtomicUsize,
	/// Per-class info
	node: CacheNodeInfo,
	// TODO: Append lock (held while the size is being updated)
}

#[derive(Debug,Clone)]
pub struct CacheHandleFile(CacheHandle);
#[derive(Debug,Clone)]
pub struct CacheHandleDir(CacheHandle);
#[derive(Debug,Clone)]
pub struct CacheHandleSymlink(CacheHandle);
#[derive(Debug,Clone)]
pub struct CacheHandleSpecial(CacheHandle);

/// Handle to a node stored in the system's filesystem cache
pub struct CacheHandle
{
	mountpt: usize,
	inode: InodeId,
	ptr: *const CachedNode,
}
unsafe impl Sync for CacheHandle {}
unsafe impl Send for CacheHandle {}


::kernel::impl_fmt! {
	Debug(self, f) for CacheHandle {
		write!(f, "CacheHandle {{ {}:{:#x} {:p} }}", self.mountpt, self.inode, self.ptr)
	}
}

impl Clone for CacheHandle
{
	fn clone(&self) -> CacheHandle {
		// SAFE: self.ptr is always valid, and operation is atomic
		unsafe {
			(*self.ptr).refcount.fetch_add(1, atomic::Ordering::Relaxed);
		}
		CacheHandle {
			mountpt: self.mountpt,
			inode: self.inode,
			ptr: self.ptr,
			}
	}
}

impl CacheHandle
{
	/// Obtain a node handle using a mountpoint ID and inode number
	pub fn from_ids(mountpoint: usize, inode: InodeId) -> super::Result<CacheHandle>
	{
		use ::kernel::lib::vec_map::Entry;
		// TODO: Use a hashmap of some form and use a fixed-range allocation (same as VMM code)
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
				Some(node) => e.insert(Box::new(CachedNode { node: node.into(), refcount: AtomicUsize::new(1) })),
				None => return Err( super::Error::NotFound ),
				},
			};

		// SAFE: Reference count has been incremented by this function, and will be valid until return
		let ent_ref = unsafe { &*ptr };

		// Create handle before checking for mountpoint
		// - This handles the edge case where a volume is being unmounted
		let rv = CacheHandle {
			mountpt: mountpoint,
			inode: inode,
			ptr: ptr,
			};

		// If this newly opened node is actually a mountpoint
		if let CacheNodeInfo::Dir(ref info) = ent_ref.node {
			let new_mountpoint = info.mountpoint.load(atomic::Ordering::Relaxed);
			if new_mountpoint != 0 {
				// Then recurse (hopefully only once) with the new mountpoint
				let new_inode = super::mount::Handle::from_id(new_mountpoint).root_inode();
				log_trace!("CacheHandle::from_ids({},{}) => Mount {}, {}",
					mountpoint, inode,  new_mountpoint, new_inode);
				return CacheHandle::from_ids(new_mountpoint, new_inode);
			}
		}

		log_trace!("CacheHandle::from_ids() {:?}", rv);
		Ok(rv)
	}
	
	
	/// Obtain a node handle using a parent directory node and a relative path
	pub fn from_path_at_node(node_h: CacheHandleDir, path: &Path) -> super::Result<CacheHandle>
	{
		let mut node_h = node_h.0;
		log_function!("CacheHandle::from_path_at_node(node_h={:?}, {:?})", node_h, path);
		let path = if path.is_absolute() {
				path.split_off_first().ok_or(super::Error::MalformedPath)?.1
			}
			else {
				path
			};
		// Iterate path components
		for seg in path
		{
			log_trace!("seg = {:?}", seg);
			// Loop resolving symbolic links
			loop
			{
				// TODO: Should symlinks be handled in this function? Or should the passed path be without symlinks?
				node_h = if let CacheNodeInfo::Symlink { ref target, .. } = *node_h.as_ref() {
					//log_debug!("- seg={:?} : SYMLINK {:?}", seg, name);
					let linkpath = Path::new(&target);
					if linkpath.is_absolute() {
						CacheHandle::from_path(linkpath)?
					}
					else {
						//TODO: To make this work (or any path-relative symlink), the current position in
						//      `path` needs to be known.
						// It can be hacked up though...
						let parent_segs_len = seg.as_bytes().as_ptr() as usize - AsRef::<[u8]>::as_ref(path).as_ptr() as usize;
						let parent = ByteStr::new( &AsRef::<[u8]>::as_ref(path)[..parent_segs_len] );

						//let segs = [ &path[..pos], &link ];
						//let p = PathChain::new(&segs);
						//try!( CacheHandle::from_path(p) )
						// Recurse with a special chained path type
						// (that iterates but can't be sliced).
						todo!("Relative symbolic links {:?} relative to {:?}", target, parent)
					}
				}
				else {
					break;
				};
			}

			// Look up this component in the current node
			node_h = match *node_h.as_ref()
				{
				CacheNodeInfo::Dir(ref info) => {
					//log_debug!("- seg={:?} : DIR", seg);
					let next_id = match info.fsnode.lookup(seg)
						{
						Ok(v) => v,
						Err(_) => return Err(super::Error::NotFound),
						};
					CacheHandle::from_ids( node_h.mountpt, next_id )?
					},
				_ => return Err(super::Error::NonDirComponent),
				};
		}
		log_trace!("CacheHandle::from_path_at_node() {:?}", node_h);
		Ok( node_h )
	}

	/// Obtain a node handle using a path
	pub fn from_path(path: &Path) -> super::Result<CacheHandle>
	{
		log_function!("CacheHandle::from_path({:?})", path);
		// TODO: Support path caching?
		
		// - Remove the leading / from the absolute path
		//  > Also checks that it's actually abolsute
		let (first_comp, path) = path.split_off_first().ok_or(super::Error::MalformedPath)?;
		if first_comp.len() != 0 {
			return Err(super::Error::MalformedPath);
		}

		// Acquire the root vnode
		let mph = super::mount::Handle::from_id(0);
		let node_h = CacheHandle::from_ids( mph.id(), mph.root_inode() )?;

		CacheHandle::from_path_at_node(node_h.into_dir()?, path)
	}
	
	pub fn get_class(&self) -> NodeClass {
		match self.as_ref()
		{
		&CacheNodeInfo::Dir { .. } => NodeClass::Dir,
		&CacheNodeInfo::File { .. } => NodeClass::File,
		&CacheNodeInfo::Symlink { .. } => NodeClass::Symlink,
		&CacheNodeInfo::Special { .. } => NodeClass::Special,
		}
	}
	pub fn is_dir(&self) -> bool {
		self.get_class() == NodeClass::Dir
	}
	pub fn into_dir(self) -> super::Result<CacheHandleDir> {
		match self.get_class() {
		NodeClass::Dir => Ok(CacheHandleDir(self)),
		_ => Err(super::Error::TypeMismatch),
		}
	}
	pub fn is_file(&self) -> bool {
		self.get_class() == NodeClass::File
	}
	pub fn into_file(self) -> super::Result<CacheHandleFile> {
		match self.get_class() {
		NodeClass::File => Ok(CacheHandleFile(self)),
		_ => Err(super::Error::TypeMismatch),
		}
	}
	pub fn is_symlink(&self) -> bool {
		self.get_class() == NodeClass::Symlink
	}

	pub fn get_node_any(&self) -> &dyn Any {
		match self.as_ref()
		{
		&CacheNodeInfo::Dir(ref inner) => inner.fsnode.get_any(),
		&CacheNodeInfo::File(ref inner) => inner.fsnode.get_any(),
		&CacheNodeInfo::Special { ref fsnode, .. } => fsnode.get_any(),
		&CacheNodeInfo::Symlink { ref fsnode, .. } => fsnode.get_any(),
		}
	}
}


/// Symbolic link methods
impl CacheHandle
{
	pub fn get_target(&self) -> super::Result<ByteString> {
		match self.as_ref()
		{
		&CacheNodeInfo::Symlink { ref fsnode, .. } => Ok(fsnode.read()),
		_ => Err( super::Error::TypeMismatch ),
		}
	}
}

impl CacheHandle
{
	fn as_ref(&self) -> &CacheNodeInfo {
		let lh = S_NODE_CACHE.lock();
		// SAFE: While this handle is active, the box will be present
		unsafe {
			let box_r = lh.get( &(self.mountpt, self.inode) ).expect("Cached node with open handle absent");
			let cn_ref: &CachedNode = &**box_r;
			// Dereference a raw pointer to disconnect the lifetimes
			&*(&cn_ref.node as *const _)
		}
	}
}
//impl ::core::convert::AsMut<Node> for CacheHandle
//{
//	fn as_ref(&mut self) -> &mut Node {
//		let lh = S_NODE_CACHE.lock();
//		// SAFE: While this handle is active, the box will be present
//		unsafe {
//			let box_r = lh.get( &(self.mountpt, self.inode) ).expect("Cached node with open handle absent");
//			let cn_ref: &CachedNode = &**box_r;
//			// Dereference a raw pointer to disconnect the lifetimes
//			&*(&cn_ref.node as *const _)
//		}
//	}
//}

