use core::prelude::*;

pub struct File(super::ObjectHandle, u64);

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
}

