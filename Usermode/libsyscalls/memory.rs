//
//
//
use crate::values as v;

#[repr(u8)]
#[derive(Debug,PartialEq)]
pub enum ProtectionMode
{
	ReadOnly   = 0,
	ReadWrite  = 1,
	Executable = 2,
	ReadWriteExecute = 3,
}

#[derive(Debug)]
pub struct Error;

#[inline]
pub unsafe fn allocate(addr: usize, count: usize) -> Result<(), Error> {
	super::to_result( crate::syscall(v::MEM_ALLOCATE { addr, count }) as usize )
		.map(|_| ())
		.map_err(|_| Error)
}
#[inline]
pub unsafe fn reprotect(addr: usize, protection: ProtectionMode) -> Result<(), Error> {
	super::to_result( crate::syscall(v::MEM_REPROTECT { addr, protection: protection as u8}) as usize )
		.map(|_| ())
		.map_err(|_| Error)
}
#[inline]
pub unsafe fn deallocate(addr: usize) -> Result<(), Error> {
	super::to_result( crate::syscall(v::MEM_DEALLOCATE { addr }) as usize )
		.map(|_| ())
		.map_err(|_| Error)
}

