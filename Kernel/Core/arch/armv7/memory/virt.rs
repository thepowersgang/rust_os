
use memory::virt::ProtectionMode;
use arch::memory::PAddr;

pub fn is_fixed_alloc<T>(addr: *const T, size: usize) -> bool {
	false
}

pub fn is_reserved<T>(addr: *const T) -> bool {
	todo!("is_reserved")
}
pub fn get_phys<T>(addr: *const T) -> ::arch::memory::PAddr {
	0
}

pub fn get_info(addr: usize) -> () {
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
pub struct AddressSpace;
impl AddressSpace
{
	pub fn pid0() -> AddressSpace {
		AddressSpace
	}
	pub fn new(clone_start: usize, clone_end: usize) -> Result<AddressSpace,::memory::virt::MapError> {
		Ok(AddressSpace)
	}
}

