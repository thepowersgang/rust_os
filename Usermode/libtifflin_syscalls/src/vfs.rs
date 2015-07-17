use core::prelude::*;

pub struct File(super::ObjectHandle, u64);
pub struct Node(super::ObjectHandle);
pub struct Dir(super::ObjectHandle);

#[derive(Debug)]
pub enum Error
{
	NotFound,
	PermissionDenied,
}

#[repr(C,u32)]
pub enum FileOpenMode
{
	None     = 0,
	ReadOnly = 1,
	Execute  = 2,
	// TODO: Write modes
}
#[repr(C,u8)]
pub enum MemoryMapMode
{
	/// Read-only mapping of a file
	ReadOnly = 0,
	/// Executable mapping of a file
	Execute = 1,
	/// Copy-on-write (used for executable files)
	COW = 2,
	/// Allows writing to the backing file
	WriteBack = 3,
}

impl Node
{
	pub fn open<T: AsRef<[u8]>>(path: T) -> Result<Node, Error> {
		let path = path.as_ref();
		match super::ObjectHandle::new( unsafe { syscall!(VFS_OPENNODE, path.as_ptr() as usize, path.len()) } as usize )
		{
		Ok(rv) => Ok( Node(rv) ),
		Err(code) => {
			panic!("TODO: Error code {}", code);
			},
		}
	}
}

impl File
{
	pub fn open<T: AsRef<[u8]>>(path: T, mode: FileOpenMode) -> Result<File,Error> {
		let path = path.as_ref();
		match super::ObjectHandle::new( unsafe { syscall!(VFS_OPENFILE, path.as_ptr() as usize, path.len(), mode as u32 as usize) } as usize )
		{
		Ok(rv) => Ok( File(rv, 0) ),
		Err(code) => {
			panic!("TODO: Error code {}", code);
			},
		}
	} 
	
	pub fn get_size(&self) -> u64 { panic!("TODO: File::get_size") }
	pub fn get_cursor(&self) -> u64 { self.1 }
	pub fn set_cursor(&mut self, pos: u64) { self.1 = pos; }
	
	pub fn read(&mut self, data: &mut [u8]) -> Result<usize,Error> {
		let count = try!( self.read_at(self.1, data) );
		self.1 += count as u64;
		Ok(count)
	}
	pub fn read_at(&self, ofs: u64, data: &mut [u8]) -> Result<usize,Error> {
		assert!(::core::mem::size_of::<usize>() == ::core::mem::size_of::<u64>());
		// SAFE: Passes valid arguments to READAT
		unsafe {
			match ::to_result( self.0.call_3(::values::VFS_FILE_READAT, ofs as usize, data.as_ptr() as usize, data.len()) as usize )
			{
			Ok(v) => Ok(v as usize),
			Err(v) => {
				panic!("TODO: Error code {}", v);
				}
			}
		}
	}
	
	pub fn write_at(&self, ofs: u64, data: &[u8]) -> Result<usize,Error> {
		assert!(::core::mem::size_of::<usize>() == ::core::mem::size_of::<u64>());
		// SAFE: All validated
		unsafe {
			match ::to_result( self.0.call_3(
				::values::VFS_FILE_WRITEAT,
				ofs as usize, data.as_ptr() as usize, data.len()
				) as usize )
			{
			Ok(v) => Ok(v as usize),
			Err(v) => {
				panic!("TODO: Error code {}", v);
				}
			}
		}
	}
	
	// Actualy safe, as it uses the aliasing restrictions from the file, and checks memory ownership
	pub fn memory_map(&self, ofs: u64, read_size: usize, mem_addr: usize, mode: MemoryMapMode) -> Result<(),Error> {
		assert!(::core::mem::size_of::<usize>() == ::core::mem::size_of::<u64>());
		// SAFE: Passes valid arguments to MEMMAP
		unsafe {
			match ::to_result( self.0.call_4(::values::VFS_FILE_MEMMAP, ofs as usize, read_size, mem_addr, mode as u8 as usize) as usize )
			{
			Ok(_) => Ok( () ),
			Err(v) => {
				panic!("TODO: Error code {}", v);
				}
			}
		}
	}
}


impl ::std_io::Read for File {
	fn read(&mut self, buf: &mut [u8]) -> ::std_io::Result<usize> {
		match self.read(buf)
		{
		Ok(v) => Ok(v),
		Err(v) => {
			panic!("VFS File read err: {:?}", v);
			},
		}
	}
}
impl ::std_io::Seek for File {
	fn seek(&mut self, pos: ::std_io::SeekFrom) -> ::std_io::Result<u64> {
		use std_io::SeekFrom;
		match pos
		{
		SeekFrom::Start(pos) => self.set_cursor(pos),
		SeekFrom::End(ofs) => {
			let pos = if ofs < 0 {
				self.get_size() - (-ofs) as u64
				} else {
				self.get_size() + ofs as u64
				};
			self.set_cursor(pos);
			},
		SeekFrom::Current(ofs) => {
			let pos = if ofs < 0 {
				self.get_cursor() - (-ofs) as u64
				} else {
				self.get_cursor() + ofs as u64
				};
			self.set_cursor(pos);
			},
		}
		Ok(self.get_cursor())
	}
}

impl Dir
{
	pub fn open<T: AsRef<[u8]>>(path: T) -> Result<Dir, Error> {
		let path = path.as_ref();
		match super::ObjectHandle::new( unsafe { syscall!(VFS_OPENDIR, path.as_ptr() as usize, path.len()) } as usize )
		{
		Ok(rv) => Ok( Dir(rv) ),
		Err(code) => {
			panic!("TODO: Error code {}", code);
			},
		}
	}
}
