// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/node.rs
//! VFS vode management
use prelude::*;
use super::Path;
use sync::mutex::LazyMutex;
use lib::byte_str::{ByteStr,ByteString};
use core::sync::atomic::{self,AtomicUsize};

pub type InodeId = u64;
pub enum IoError {
	NoSpace,
	NoNodes,
	ReadFail(::metadevs::storage::IoError),
	OutOfRange,
	ReadOnly,
	Timeout,
	Transient,
	NotFound,
	AlreadyExists,
	Corruption,
	Unknown(&'static str),
}
impl IoError {
	fn as_str(&self) -> &'static str {
		match self
		{
		&IoError::NoSpace => "NoSpace",
		&IoError::NoNodes => "NoNodes",
		&IoError::ReadFail(_) => "ReadFail",
		&IoError::OutOfRange => "OutOfRange",
		&IoError::ReadOnly => "ReadOnly",
		&IoError::Timeout => "Timeout",
		&IoError::Transient => "Transient",
		&IoError::NotFound => "NotFound",
		&IoError::AlreadyExists => "AlreadyExists",
		&IoError::Corruption => "Corruption",
		&IoError::Unknown(_) => "Unknown",
		}
	}
}
impl From<IoError> for super::Error {
	fn from(v: IoError) -> super::Error {
		match v
		{
		IoError::NotFound => super::Error::NotFound,
		IoError::Unknown(s) => super::Error::Unknown(s),
		_ => super::Error::Unknown(v.as_str()),
		}
	}
}
impl From<::metadevs::storage::IoError> for IoError {
	fn from(v: ::metadevs::storage::IoError) -> IoError {
		IoError::ReadFail(v)
	}
}
pub type Result<T> = ::core::result::Result<T,IoError>;

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
	fn write(&self, ofs: u64, buf: &mut [u8]) -> Result<usize>;
}

// TODO: Should this be &ByteStr instead of an iterator?
// - For non-byte on-disk filenames (FAT LFN, NTFS) it would lead to excessive allocations.
/// Return `false` when read should stop
pub type ReadDirCallback<'a> = FnMut(InodeId, &mut Iterator<Item=u8>)->bool + 'a;

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
	fn link(&self, name: &ByteStr, inode: InodeId) -> Result<()>;
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
	File(Box<File>),
	Dir(Box<Dir>),
	Symlink(Box<Symlink>),
	Special(Box<Special>),
}

struct CachedNode
{
	node: Node,
	refcount: AtomicUsize,
	
	// File memory map data
	//mapped_pages: HashMap<u64,FrameHandle>,
}

pub struct CacheHandle
{
	mountpt: usize,
	inode: InodeId,
	ptr: *const CachedNode,
}
unsafe impl Sync for CacheHandle {}
unsafe impl Send for CacheHandle {}

static S_NODE_CACHE: LazyMutex<::lib::VecMap<(usize,InodeId),Box<CachedNode>>> = lazymutex_init!();

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
		log_function!("CacheHandle::from_path({:?})", path);
		// TODO: Support path caching?
		
		// Locate mountpoint for the path
		// - This does a longest prefix match on the path
		let (mph,tail) = try!(super::mount::Handle::for_path(path));
		// Acquire the mountpoint's root node
		let mut node_h = try!(CacheHandle::from_ids( mph.id(), mph.root_inode() ));
		log_debug!("- tail={:?}", tail);
		// Iterate components of the path that were not consumed by the mountpoint
		for seg in tail
		{
			loop {
				// TODO: Should symlinks be handled in this function? Or should the passed path be without symlinks?
				node_h = if let Node::Symlink(ref link) = *node_h.as_ref() {
					let name = link.read();
					//log_debug!("- seg={:?} : SYMLINK {:?}", seg, name);
					let linkpath = Path::new(&name);
					if linkpath.is_absolute() {
						try!(CacheHandle::from_path(linkpath))
					}
					else {
						//TODO: To make this work (or any path-relative symlink), the current position in
						//      `path` needs to be known.
						//let segs = [ &path[..pos], &link ];
						//let p = PathChain::new(&segs);
						//try!( CacheHandle::from_path(p) )
						// Recurse with a special chained path type
						// (that iterates but can't be sliced).
						todo!("Relative symbolic links {:?}", name)
					}
				}
				else {
					break;
				};
			}
			node_h = match *node_h.as_ref()
				{
				Node::Dir(ref dir) => {
					//log_debug!("- seg={:?} : DIR", seg);
					let next_id = match dir.lookup(seg)
						{
						Ok(v) => v,
						Err(_) => return Err(super::Error::NotFound),
						};
					try!(CacheHandle::from_ids( node_h.mountpt, next_id ))
					},
				_ => return Err(super::Error::NonDirComponent),
				};
		}
		log_debug!("return {:?}", node_h);
		Ok( node_h )
	}
	
	pub fn get_class(&self) -> NodeClass {
		match self.as_ref()
		{
		&Node::Dir(_) => NodeClass::Dir,
		&Node::File(_) => NodeClass::File,
		&Node::Symlink(_) => NodeClass::Symlink,
		&Node::Special(_) => NodeClass::Special,
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
}
/// Directory methods
impl CacheHandle
{
	pub fn create(&self, name: &ByteStr, ty: NodeType) -> super::Result<CacheHandle> {
		match self.as_ref()
		{
		&Node::Dir(ref r) => {
			let inode = try!(r.create(name, ty));
			Ok( try!(CacheHandle::from_ids(self.mountpt, inode)) )
			},
		_ => Err( super::Error::Unknown("Calling create on non-directory") ),
		}
	}
	pub fn read_dir(&self, ofs: usize, items: &mut ReadDirCallback) -> super::Result<usize> {
		match self.as_ref()
		{
		&Node::Dir(ref r) => Ok( try!(r.read(ofs, items)) ),
		_ => Err( super::Error::Unknown("Calling read_dir on non-directory") ),
		}
	}
	pub fn open_child(&self, name: &ByteStr) -> super::Result<CacheHandle> {
		match self.as_ref()
		{
		&Node::Dir(ref r) => {
			let inode = try!(r.lookup(name));
			Ok( try!(CacheHandle::from_ids(self.mountpt, inode)) )
			},
		_ => Err( super::Error::Unknown("Calling open_child on non-directory") ),
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
		&Node::File(ref r) => r.size(),
		_ => 0,
		}
	}
	pub fn read(&self, ofs: u64, dst: &mut [u8]) -> super::Result<usize> {
		match self.as_ref()
		{
		&Node::File(ref f) => Ok( try!(f.read(ofs, dst)) ),
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
		&Node::Symlink(ref l) => Ok(l.read()),
		_ => Err( super::Error::TypeMismatch ),
		}
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
//impl ::core::convert::AsMut<Node> for CacheHandle
//{
//	fn as_ref(&mut self) -> &mut Node {
//		let lh = S_NODE_CACHE.lock();
//		// SAFE: While this handle is active, the box will be present
//		unsafe {
//			let box_r = lh.get( &(self.mountpt, self.inode) ).expect("Cached node with open handle absent");
//			let cn_ref: &CachedNode = &**box_r;
//			::core::mem::transmute(&cn_ref.node)
//		}
//	}
//}

