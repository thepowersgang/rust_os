//
//
//
//! Low-level filesystem access

/// Arbitary VFS node handle
pub struct Node(super::ObjectHandle);
/// File handle (seekable buffer)
pub struct File(super::ObjectHandle, u64);
/// Directory handle
pub struct Dir(super::ObjectHandle);
/// Symbolic link
pub struct Symlink(super::ObjectHandle);

pub use ::values::VFSError as Error;
pub use ::values::VFSNodeType as NodeType;
pub use ::values::VFSFileOpenMode as FileOpenMode;
pub use ::values::VFSMemoryMapMode as MemoryMapMode;


fn to_obj(val: usize) -> Result<super::ObjectHandle, Error> {
	super::ObjectHandle::new(val).map_err(|code| Error::from(code))
}
fn to_result(val: usize) -> Result<u32, Error> {
	super::to_result(val).map_err(|code| Error::from(code))
}

impl Node
{
	/// Open an arbitary node
	pub fn open<T: AsRef<[u8]>>(path: T) -> Result<Node, Error> {
		let path = path.as_ref();
		// SAFE: Syscall
		to_obj( unsafe { syscall!(VFS_OPENNODE, path.as_ptr() as usize, path.len()) } as usize )
			.map(|h| Node(h))
	}

	/// Query the class/type of the node
	pub fn class(&self) -> NodeType {
		// SAFE: Syscall with no side-effects
		NodeType::from( unsafe { self.0.call_0(::values::VFS_NODE_GETTYPE) } as u32 )
	}

	/// Convert handle to a directory handle
	pub fn into_dir(self) -> Result<Dir,Error> {
		// SAFE: Syscall
		to_obj( unsafe { self.0.call_0(::values::VFS_NODE_TODIR) } as usize )
			.map(|h| Dir(h))
	}
	/// Convert handle to a file handle (with the provided mode)
	pub fn into_file(self, mode: FileOpenMode) -> Result<File,Error> {
		// SAFE: Syscall
		to_obj( unsafe { self.0.call_1(::values::VFS_NODE_TOFILE, mode as u32 as usize) } as usize )
			.map(|h| File(h, 0))
	}
	/// Convert handle to a symbolic link handle
	pub fn into_symlink(self) -> Result<Symlink,Error> {
		// SAFE: Syscall
		to_obj( unsafe { self.0.call_0(::values::VFS_NODE_TOLINK) } as usize )
			.map(|h| Symlink(h))
	}
}
impl ::Object for Node {
	const CLASS: u16 = ::values::CLASS_VFS_NODE;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		Node(handle)
	}
	fn into_handle(self) -> ::ObjectHandle { self.0 }
	fn handle(&self) -> &::ObjectHandle { &self.0 }

	type Waits = ();
}

impl File
{
	/// Open a file with the provided mode
	pub fn open<T: AsRef<[u8]>>(path: T, mode: FileOpenMode) -> Result<File,Error> {
		let path = path.as_ref();
		kernel_log!("path={:?}, mode={:?}", path, mode);
		// SAFE: Syscall
		to_obj( unsafe { syscall!(VFS_OPENFILE, path.as_ptr() as usize, path.len(), Into::<u8>::into(mode) as usize) } as usize )
			.map(|h| File(h, 0))
	} 
	
	/// Query the size of the file
	pub fn get_size(&self) -> u64 {
		// SAFE: Syscall with no sideffects
		unsafe { self.0.call_0(::values::VFS_FILE_GETSIZE) }
	}

	/// Query the current cursor position
	pub fn get_cursor(&self) -> u64 { self.1 }
	/// Set te current cursor position (no checking)
	pub fn set_cursor(&mut self, pos: u64) { self.1 = pos; }
	
	/// Read bytes at the cursor (incrementing)
	pub fn read(&mut self, data: &mut [u8]) -> Result<usize,Error> {
		let count = try!( self.read_at(self.1, data) );
		self.1 += count as u64;
		Ok(count)
	}
	/// Read from an arbitary location in the file
	pub fn read_at(&self, ofs: u64, data: &mut [u8]) -> Result<usize,Error> {
		// SAFE: Passes valid arguments to READAT
		to_result( unsafe { self.0.call_3l(::values::VFS_FILE_READAT, ofs, data.as_ptr() as usize, data.len()) as usize } )
			.map(|v| v as usize)
	}
	
	/// Write to an arbitary location in the file
	pub fn write_at(&self, ofs: u64, data: &[u8]) -> Result<usize,Error> {
		// SAFE: All validated
		to_result( unsafe { self.0.call_3l( ::values::VFS_FILE_WRITEAT, ofs, data.as_ptr() as usize, data.len() ) } as usize )
			.map(|v| v as usize)
	}
	
	// Actualy safe, as it uses the aliasing restrictions from the file, and ensures that the provided address is free
	/// Map a portion of this file into this process's address space.
	pub fn memory_map(&self, ofs: u64, read_size: usize, mem_addr: *const ::Void, mode: MemoryMapMode) -> Result<(),Error> {
		// SAFE: Passes valid arguments to MEMMAP
		to_result( unsafe { self.0.call_4l(::values::VFS_FILE_MEMMAP, ofs, read_size, mem_addr as usize, mode as u8 as usize) } as usize )
			.map( |_| () )
	}
}
impl ::Object for File {
	const CLASS: u16 = ::values::CLASS_VFS_FILE;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		File(handle, 0)
	}
	fn into_handle(self) -> ::ObjectHandle { self.0 }
	fn handle(&self) -> &::ObjectHandle { &self.0 }

	type Waits = ();
}


impl Dir
{
	/// Open a directory for iteration
	pub fn open<T: AsRef<[u8]>>(path: T) -> Result<Dir, Error> {
		let path = path.as_ref();
		// SAFE: Syscall
		match super::ObjectHandle::new( unsafe { syscall!(VFS_OPENDIR, path.as_ptr() as usize, path.len()) } as usize )
		{
		Ok(rv) => Ok( Dir(rv) ),
		Err(code) => Err( From::from(code) ),
		}
	}

	/// Obtain the name of the next entry in the directory
	pub fn read_ent<'a>(&mut self, namebuf: &'a mut [u8]) -> Result<&'a [u8], Error> {
		// SAFE: Syscall
		let len = try!(to_result(unsafe { self.0.call_2(::values::VFS_DIR_READENT, namebuf.as_ptr() as usize, namebuf.len()) } as usize ));
		Ok( &namebuf[ .. len as usize] )
	}
}
impl ::Object for Dir {
	const CLASS: u16 = ::values::CLASS_VFS_DIR;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		Dir(handle)
	}
	fn into_handle(self) -> ::ObjectHandle { self.0 }
	fn handle(&self) -> &::ObjectHandle { &self.0 }

	type Waits = ();
}


impl Symlink
{
	/// Open a symbolic link
	pub fn open<T: AsRef<[u8]>>(path: T) -> Result<Symlink, Error> {
		let path = path.as_ref();
		// SAFE: Syscall
		to_obj( unsafe { syscall!(VFS_OPENLINK, path.as_ptr() as usize, path.len()) } as usize )
			.map(|h| Symlink(h))
	}

	/// Read the target path from the link
	///
	/// If the buffer is not long enough, the return value is truncated.
	// TODO: Return an error if the buffer wasn't large enough
	pub fn read_target<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], Error> {
		// SAFE: Syscall with correct args
		let len = try!(to_result( unsafe { self.0.call_2(::values::VFS_LINK_READ, buf.as_mut_ptr() as usize, buf.len()) } as usize ));
		Ok( &buf[ .. len as usize] )
	}
}
impl ::Object for Symlink {
	const CLASS: u16 = ::values::CLASS_VFS_LINK;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		Symlink(handle)
	}
	fn into_handle(self) -> ::ObjectHandle { self.0 }
	fn handle(&self) -> &::ObjectHandle { &self.0 }

	type Waits = ();
}
