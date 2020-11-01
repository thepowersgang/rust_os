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
/// Directory iterator
pub struct DirIter(::ObjectHandle);
/// Symbolic link
pub struct Symlink(super::ObjectHandle);

pub use ::values::VFSError as Error;
pub use ::values::VFSNodeType as NodeType;
pub use ::values::VFSFileOpenMode as FileOpenMode;
pub use ::values::VFSMemoryMapMode as MemoryMapMode;

pub fn root() -> &'static Dir {
	use ::core::sync::atomic::{Ordering,AtomicBool};
	static mut ROOT: Option<Dir> = None;
	static ROOT_SETTING: AtomicBool = AtomicBool::new(false);
	// SAFE: Single-write (enforced by the atomic)
	unsafe {
		if ROOT.is_none() {
			assert!( ROOT_SETTING.swap(true, Ordering::SeqCst) == false, "TODO: Race in initialising root handle" );
			ROOT = Some(crate::object_from_raw(1).expect("Bad RO root handle"));
		}
		ROOT.as_ref().unwrap()
	}
}


#[inline]
fn to_obj(val: usize) -> Result<super::ObjectHandle, Error> {
	super::ObjectHandle::new(val).map_err(|code| Error::try_from(code).expect("Bad VFS Error"))
}
#[inline]
fn to_result(val: usize) -> Result<u32, Error> {
	super::to_result(val).map_err(|code| Error::try_from(code).expect("Bad VFS Error"))
}

impl Node
{
	/// Query the class/type of the node
	#[inline]
	pub fn class(&self) -> NodeType {
		// SAFE: Syscall with no side-effects
		NodeType::try_from( unsafe { self.0.call_0(::values::VFS_NODE_GETTYPE) } as u32 ).expect("Bad VFS Node Type")
	}

	/// Convert handle to a directory handle
	#[inline]
	pub fn into_dir(self) -> Result<Dir,Error> {
		// SAFE: Syscall
		to_obj( unsafe { self.0.call_0_v(::values::VFS_NODE_TODIR) } as usize )
			.map(|h| Dir(h))
	}
	/// Convert handle to a file handle (with the provided mode)
	#[inline]
	pub fn into_file(self, mode: FileOpenMode) -> Result<File,Error> {
		// SAFE: Syscall
		to_obj( unsafe { self.0.call_1_v(::values::VFS_NODE_TOFILE, mode as u8 as usize) } as usize )
			.map(|h| File(h, 0))
	}
	/// Convert handle to a symbolic link handle
	#[inline]
	pub fn into_symlink(self) -> Result<Symlink,Error> {
		// SAFE: Syscall
		to_obj( unsafe { self.0.call_0_v(::values::VFS_NODE_TOLINK) } as usize )
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
	/// Query the size of the file
	#[inline]
	pub fn get_size(&self) -> u64 {
		// SAFE: Syscall with no sideffects
		unsafe { self.0.call_0(::values::VFS_FILE_GETSIZE) }
	}

	/// Query the current cursor position
	#[inline]
	pub fn get_cursor(&self) -> u64 { self.1 }
	/// Set te current cursor position (no checking)
	#[inline]
	pub fn set_cursor(&mut self, pos: u64) { self.1 = pos; }
	
	/// Read bytes at the cursor (incrementing)
	#[inline]
	pub fn read(&mut self, data: &mut [u8]) -> Result<usize,Error> {
		let count = self.read_at(self.1, data)?;
		self.1 += count as u64;
		Ok(count)
	}
	/// Read from an arbitary location in the file
	#[inline]
	pub fn read_at(&self, ofs: u64, data: &mut [u8]) -> Result<usize,Error> {
		// SAFE: Passes valid arguments to READAT
		to_result( unsafe { self.0.call_3l(::values::VFS_FILE_READAT, ofs, data.as_ptr() as usize, data.len()) as usize } )
			.map(|v| v as usize)
	}
	
	/// Write to an arbitary location in the file
	#[inline]
	pub fn write_at(&self, ofs: u64, data: &[u8]) -> Result<usize,Error> {
		// SAFE: All validated
		to_result( unsafe { self.0.call_3l( ::values::VFS_FILE_WRITEAT, ofs, data.as_ptr() as usize, data.len() ) } as usize )
			.map(|v| v as usize)
	}
	
	// Actualy safe, as it uses the aliasing restrictions from the file, and ensures that the provided address is free
	/// Map a portion of this file into this process's address space.
	#[inline]
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
	/// Obtain a handle to enumerate the contents of the directory
	pub fn enumerate(&self) -> Result<DirIter, Error> {
		// SAFE: Syscall
		match super::ObjectHandle::new( unsafe { self.0.call_0(::values::VFS_DIR_ENUMERATE) } as usize )
		{
		Ok(rv) => Ok( DirIter(rv) ),
		Err(code) => Err( Error::try_from(code).expect("Bad VFS Error") ),
		}
	}

	/// Open an immediate child of this directory
	#[inline]
	pub fn open_child<P: ?Sized+AsRef<[u8]>>(&self, name: &P) -> Result<Node, Error> {
		let name = name.as_ref();
		// SAFE: Syscall
		match super::ObjectHandle::new( unsafe { self.0.call_2(::values::VFS_DIR_OPENCHILD, name.as_ptr() as usize, name.len()) } as usize )
		{
		Ok(rv) => Ok( Node(rv) ),
		Err(code) => Err( Error::try_from(code).expect("Bad VFS Error") ),
		}
	}

	/// Open a path relative to this directory
	#[inline]
	pub fn open_child_path<P: ?Sized+AsRef<[u8]>>(&self, path: &P) -> Result<Node, Error> {
		let name = path.as_ref();
		// SAFE: Syscall
		match super::ObjectHandle::new( unsafe { self.0.call_2(::values::VFS_DIR_OPENPATH, name.as_ptr() as usize, name.len()) } as usize )
		{
		Ok(rv) => Ok( Node(rv) ),
		Err(code) => Err( Error::try_from(code).expect("Bad VFS Error") ),
		}
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
impl Clone for Dir {
	fn clone(&self) -> Self {
		Dir( self.0.try_clone().expect("Failed to clone vfs::Dir (should have been able to)") )
	}
}

impl DirIter
{
	/// Obtain the name of the next entry in the directory
	#[inline]
	pub fn read_ent<'a>(&mut self, namebuf: &'a mut [u8]) -> Result<Option<&'a [u8]>, Error> {
		// SAFE: Syscall
		let len = to_result(unsafe { self.0.call_2(::values::VFS_DIRITER_READENT, namebuf.as_ptr() as usize, namebuf.len()) } as usize )?;
		if len > 0 {
			Ok( Some( &namebuf[ .. len as usize] ) )
		}
		else {
			Ok(None)
		}
	}
}
impl ::Object for DirIter {
	const CLASS: u16 = ::values::CLASS_VFS_DIRITER;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		DirIter(handle)
	}
	fn into_handle(self) -> ::ObjectHandle { self.0 }
	fn handle(&self) -> &::ObjectHandle { &self.0 }

	type Waits = ();
}


impl Symlink
{
	/// Read the target path from the link
	///
	/// If the buffer is not long enough, the return value is truncated.
	// TODO: Return an error if the buffer wasn't large enough
	#[inline]
	pub fn read_target<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], Error> {
		// SAFE: Syscall with correct args
		let len = to_result( unsafe { self.0.call_2(::values::VFS_LINK_READ, buf.as_mut_ptr() as usize, buf.len()) } as usize )?;
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
