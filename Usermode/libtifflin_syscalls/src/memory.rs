
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
	panic!("TODO: allocate");
}
#[inline]
pub unsafe fn reprotect(addr: usize, protection: ProtectionMode) -> Result<(), Error> {
	panic!("TODO: reprotect");
}
#[inline]
pub unsafe fn deallocate(addr: usize) -> Result<(), Error> {
	panic!("TODO: deallocate");
}

