
use core::prelude::*;

#[repr(C,u8)]
pub enum ProtectionMode
{
	ReadOnly,
	ReadWrite,
	Executable,
	ReadWriteExecute,
	CopyOnWrite,
	CopyOnWriteExec,
}

#[derive(Debug)]
pub struct Error;

#[inline]
pub unsafe fn allocate(addr: usize, protection: ProtectionMode) -> Result<(), Error> {
	super::to_result( syscall!(MEM_ALLOCATE, addr, protection as u8 as usize) as usize )
		.map(|_| ())
		.map_err(|_| Error)
}
#[inline]
pub unsafe fn reprotect(addr: usize, protection: ProtectionMode) -> Result<(), Error> {
	super::to_result( syscall!(MEM_REPROTECT, addr, protection as u8 as usize) as usize )
		.map(|_| ())
		.map_err(|_| Error)
}
#[inline]
pub unsafe fn deallocate(addr: usize) -> Result<(), Error> {
	super::to_result( syscall!(MEM_DEALLOCATE, addr) as usize )
		.map(|_| ())
		.map_err(|_| Error)
}

