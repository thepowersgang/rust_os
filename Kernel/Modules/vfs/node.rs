// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/node.rs
//! VFS node management
use ::kernel::prelude::*;
use ::kernel::lib::byte_str::{ByteStr,ByteString};

pub type InodeId = u64;
pub type Result<T> = ::core::result::Result<T,super::Error>;

/// Node type used by `Dir::create`
#[derive(Debug,PartialEq,Copy,Clone)]
pub enum NodeType<'a> {
	File,
	Dir,
	Symlink(&'a super::Path),
}

/// Base trait for a VFS node, defines common operation on nodes
pub trait NodeBase: Send {
	/// Return the volume's inode number
	fn get_id(&self) -> InodeId;
	/// Return an &Any associated with this node (not necessarily same as `self`, up to the driver)
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
	
	/// Read a directory entry, and pass it to the provided callback.
	/// - If the callback returns `false`, the routine should end and return `Ok` with the `start_ofs` for the next call
	/// - If an offset is passed that is at or past the end of the directory, return immediately with the end of directory value
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

