//
//
//

use memory::virt::ProtectionMode;
use arch::memory::PAddr;

pub fn is_fixed_alloc<T>(addr: *const T, size: usize) -> bool {
	const BASE : usize = super::addresses::KERNEL_BASE;
	const LIMIT: usize = super::addresses::KERNEL_BASE + 4*1024*1024;
	let addr = addr as usize;
	if addr < BASE {
		false
	}
	else if addr >= LIMIT {
		false
	}
	else if addr + size > LIMIT {
		false
	}
	else {
		true
	}
}

pub fn is_reserved<T>(addr: *const T) -> bool {
	todo!("is_reserved")
}
pub fn get_phys<T>(addr: *const T) -> ::arch::memory::PAddr {
	todo!("get_phys")
}

pub fn get_info<T>(addr: *const T) -> Option<(u32, ::memory::virt::ProtectionMode)> {
	todo!("get_info")
}

pub unsafe fn fixed_alloc(p: PAddr, count: usize) -> Option<*mut ()> {
	None
}
pub unsafe fn map(a: *mut (), p: PAddr, mode: ProtectionMode) {
}
pub unsafe fn reprotect(a: *mut (), mode: ProtectionMode) {
}
pub unsafe fn unmap(a: *mut ()) -> Option<PAddr> {
	todo!("unmap")
}

#[derive(Debug)]
pub struct AddressSpace(u32);
impl AddressSpace
{
	pub fn pid0() -> AddressSpace {
		extern "C" {
			static kernel_table0: [u32; 4096];
		}
		AddressSpace( &kernel_table0 as *const _ as usize as u32 )
	}
	pub fn new(clone_start: usize, clone_end: usize) -> Result<AddressSpace,::memory::virt::MapError> {
		todo!("AddressSpace::new({:#x} -- {:#x})", clone_start, clone_end);
	}

	pub fn get_ttbr0(&self) -> u32 { self.0 }
}

