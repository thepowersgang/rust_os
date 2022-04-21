// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/node.rs
//! VFS vode management
use crate::prelude::*;
use super::Path;
use crate::sync::mutex::LazyMutex;
use crate::lib::byte_str::{ByteStr,ByteString};
use core::sync::atomic::{self,AtomicUsize};

pub type InodeId = u64;
pub type Result<T> = ::core::result::Result<T,super::Error>;

/// Node type used by `Dir::create`
#[derive(Debug,PartialEq)]
pub enum NodeType<'a> {
	File,
	Dir,
	Symlink(&'a super::Path),
}
#[derive(Debug,PartialEq)]
pub enum NodeClass {
	File,
	Dir,
	Symlink,
	Special,
}

/// Base trait for a VFS node, defines common operation on nodes
pub trait NodeBase: Send {
	/// Return the volume's inode number
	fn get_id(&self) -> InodeId;
	/// Return an &Any associated with this node (not nessesarily same as `self`, up to the driver)
	fn get_any(&self) -> &dyn Any;
}
/// Trait for "File" nodes
pub trait File: NodeBase {
	/// Returns the size (in bytes) of this file
	fn size(&self) -> u64;
	/// Update the size of the file (zero padding or truncating)
	fn truncate(&self, newsize: u64) -> Result<u64>;
	/// Clear the specified range of the file (replace with zeroes)
	fn clear(&self, ofs: u64, size: u64) -> Result<()>;
	/// Read data from the file
	fn read(&self, ofs: u64, buf: &mut [u8]) -> Result<usize>;
	/// Write data to the file, can only grow the file if ofs==size
	fn write(&self, ofs: u64, buf: &[u8]) -> Result<usize>;
}

// TODO: Should this be &ByteStr instead of an iterator?
// - For non-byte on-disk filenames (FAT LFN, NTFS) it would lead to excessive allocations.
/// Return `false` when read should stop
pub type ReadDirCallback<'a> = dyn FnMut(InodeId, &mut dyn Iterator<Item=u8>)->bool + 'a;

/// Trait for "Directory" nodes, containers for files.
pub trait Dir: NodeBase {
	/// Acquire a node given the name
	fn lookup(&self, name: &ByteStr) -> Result<InodeId>;
	
	/// Read Entry
	/// 
	/// Returns:
	/// - Ok(Next Offset)
	/// - Err(e) : Any error
	fn read(&self, start_ofs: usize, callback: &mut ReadDirCallback) -> Result<usize>;
	
	/// Create a new file in this directory
	/// 
	/// Returns the newly created node
	fn create(&self, name: &ByteStr, nodetype: NodeType) -> Result<InodeId>;
	/// Create a new name for the provided inode
	fn link(&self, name: &ByteStr, inode: &dyn NodeBase) -> Result<()>;
	/// Remove the specified name
	fn unlink(&self, name: &ByteStr) -> Result<()>;
}
/// Trait for symbolic link nodes.
pub trait Symlink: NodeBase {
	/// Reads the contents of the symbolic link into a string
	///
	/// TODO: I'm not sure about this signature... as it requires an allocation
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
	File(Box<dyn File>),
	Dir(Box<dyn Dir>),
	Symlink(Box<dyn Symlink>),
	Special(Box<dyn Special>),
}

enum CacheNodeInt
{
	File {
		fsnode: Box<dyn File>
		
		// File memory map data
		//mapped_pages: HashMap<u64,FrameHandle>,
		},
	Dir {
		mountpoint: AtomicUsize,	// 0 is invalid (that's root), so means "no mount"
		fsnode: Box<dyn Dir>
		},
	Symlink {
		target: ByteString,
		fsnode: Box<dyn Symlink>
		},
	Special {
		fsnode: Box<dyn Special>
		},
}
impl_from!{
	From<Node>(v) for CacheNodeInt {
		match v
		{
		Node::File(f) => CacheNodeInt::File { fsnode: f },
		Node::Dir(f) => CacheNodeInt::Dir { fsnode: f, mountpoint: AtomicUsize::new(0) },
		Node::Symlink(f) => CacheNodeInt::Symlink { target: f.read(), fsnode: f },
		Node::Special(f) => CacheNodeInt::Special { fsnode: f },
		}
	}
}

struct CachedNode
{
	refcount: AtomicUsize,
	node: CacheNodeInt,
}

pub struct CacheHandle
{
	mountpt: usize,
	inode: InodeId,
	ptr: *const CachedNode,
}
unsafe impl Sync for CacheHandle {}
unsafe impl Send for CacheHandle {}

static S_NODE_CACHE: LazyMutex<crate::lib::VecMap<(usize,InodeId),Box<CachedNode>>> = lazymutex_init!();

pub fn init()
{
	S_NODE_CACHE.init(|| Default::default());
}

impl_fmt! {
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
		use crate::lib::vec_map::Entry;
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
		if let CacheNodeInt::Dir { mountpoint: ref new_mountpoint, .. } = ent_ref.node {
			let new_mountpoint = new_mountpoint.load(atomic::Ordering::Relaxed);
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
	
	
	pub fn from_path_at_node(mut node_h: CacheHandle, path: &Path) -> super::Result<CacheHandle>
	{
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
				node_h = if let CacheNodeInt::Symlink { ref target, .. } = *node_h.as_ref() {
					//log_debug!("- seg={:?} : SYMLINK {:?}", seg, name);
					let linkpath = Path::new(&target);
					if linkpath.is_absolute() {
						CacheHandle::from_path(linkpath)?
					}
					else {
						//TODO: To make this work (or any path-relative symlink), the current position in
						//      `path` needs to be known.
						// It can be hacked up though...
						let parent_segs_len = seg.as_ref().as_ptr() as usize - AsRef::<[u8]>::as_ref(path).as_ptr() as usize;
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
				CacheNodeInt::Dir { fsnode: ref dir, .. } => {
					//log_debug!("- seg={:?} : DIR", seg);
					let next_id = match dir.lookup(seg)
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

		CacheHandle::from_path_at_node(node_h, path)
	}
	
	pub fn get_class(&self) -> NodeClass {
		match self.as_ref()
		{
		&CacheNodeInt::Dir { .. } => NodeClass::Dir,
		&CacheNodeInt::File { .. } => NodeClass::File,
		&CacheNodeInt::Symlink { .. } => NodeClass::Symlink,
		&CacheNodeInt::Special { .. } => NodeClass::Special,
		}
	}
	pub fn is_dir(&self) -> bool {
		self.get_class() == NodeClass::Dir
	}
	pub fn is_file(&self) -> bool {
		self.get_class() == NodeClass::File
	}
	pub fn is_symlink(&self) -> bool {
		self.get_class() == NodeClass::Symlink
	}

	pub fn get_any(&self) -> &dyn Any {
		match self.as_ref()
		{
		&CacheNodeInt::Dir { ref fsnode, .. } => fsnode.get_any(),
		&CacheNodeInt::File { ref fsnode, .. } => fsnode.get_any(),
		&CacheNodeInt::Special { ref fsnode, .. } => fsnode.get_any(),
		&CacheNodeInt::Symlink { ref fsnode, .. } => fsnode.get_any(),
		}
	}
}
/// Directory methods
impl CacheHandle
{
	pub fn create(&self, name: &ByteStr, ty: NodeType) -> super::Result<CacheHandle> {
		match self.as_ref()
		{
		&CacheNodeInt::Dir { ref fsnode, .. } => {
			let inode = fsnode.create(name, ty)?;
			Ok( CacheHandle::from_ids(self.mountpt, inode)? )
			},
		_ => Err( super::Error::Unknown("Calling create on non-directory") ),
		}
	}
	pub fn read_dir(&self, ofs: usize, items: &mut ReadDirCallback) -> super::Result<usize> {
		match self.as_ref()
		{
		&CacheNodeInt::Dir { ref fsnode, .. } => Ok( fsnode.read(ofs, items)? ),
		_ => Err( super::Error::Unknown("Calling read_dir on non-directory") ),
		}
	}
	pub fn open_child(&self, name: &ByteStr) -> super::Result<CacheHandle> {
		match self.as_ref()
		{
		&CacheNodeInt::Dir { ref fsnode, .. } => {
			let inode = fsnode.lookup(name)?;
			Ok( CacheHandle::from_ids(self.mountpt, inode)? )
			},
		_ => Err( super::Error::Unknown("Calling open_child on non-directory") ),
		}
	}
}
/// Directory methods (mountpoint)
impl CacheHandle
{
	pub fn is_mountpoint(&self) -> bool {
		match self.as_ref()
		{
		&CacheNodeInt::Dir { ref mountpoint, .. } => {
			mountpoint.load(atomic::Ordering::Relaxed) != 0
			},
		_ => false,
		}
	}
	/// Returns `true` if the mount binding succeeded
	pub fn mount(&self, filesystem_id: usize) -> bool {
		match self.as_ref()
		{
		&CacheNodeInt::Dir { ref mountpoint, .. } => {
			mountpoint.compare_exchange(0, filesystem_id, atomic::Ordering::Relaxed, atomic::Ordering::Relaxed).is_ok()
			},
		_ => false,
		}
	}
}
/// Normal file methods
impl CacheHandle
{
	/// Valid size = maximum offset in the file
	pub fn get_valid_size(&self) -> u64 {
		match self.as_ref()
		{
		&CacheNodeInt::File { ref fsnode, .. } => fsnode.size(),
		_ => 0,
		}
	}
	pub fn read(&self, ofs: u64, dst: &mut [u8]) -> super::Result<usize> {
		match self.as_ref()
		{
		&CacheNodeInt::File { ref fsnode, .. } => Ok( fsnode.read(ofs, dst)? ),
		_ => Err( super::Error::Unknown("Calling read on non-file") ),
		}
	}
}


/// Symbolic link methods
impl CacheHandle
{
	pub fn get_target(&self) -> super::Result<ByteString> {
		match self.as_ref()
		{
		&CacheNodeInt::Symlink { ref fsnode, .. } => Ok(fsnode.read()),
		_ => Err( super::Error::TypeMismatch ),
		}
	}
}

impl CacheHandle
{
	fn as_ref(&self) -> &CacheNodeInt {
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

