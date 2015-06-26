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
		match super::ObjectHandle::new( unsafe { syscall!(VFS_OPENFILE, path.as_ptr() as usize, path.len(), mode as u32 as usize) } as u32 )
		{
		Ok(rv) => Ok( File(rv) ),
		Err(code) => {
			panic!("TODO: Error code {}", code);
			},
		}
	} 
	
	pub fn read_at(&self, ofs: u64, data: &mut [u8]) -> Result<usize,Error> {
		unimplemented!();
	}
}

