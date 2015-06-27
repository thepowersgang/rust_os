use core::prelude::*;

pub struct File(super::ObjectHandle);

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
		Ok(rv) => Ok( File(rv) ),
		Err(code) => {
			panic!("TODO: Error code {}", code);
			},
		}
	} 
	
	pub fn read_at(&self, ofs: u64, data: &mut [u8]) -> Result<usize,Error> {
		assert!(::core::mem::size_of::<usize>() == ::core::mem::size_of::<u64>());
		// SAFE: Passes valid arguments to READAT
		unsafe {
			match ::to_result( self.0.call_3(::values::VFS_FILE_READAT, ofs as usize, data.len(), data.as_ptr() as usize) as usize )
			{
			Ok(v) => Ok(v as usize),
			Err(v) => {
				panic!("TODO: Error code {}", v);
				}
			}
		}
	}
}

