
use memory::virt::ProtectionMode;

pub struct AddressSpace(u64);

pub fn post_init()
{
}


pub fn is_reserved<T>(addr: *const T) -> bool
{
	false
}
pub fn get_phys<T>(addr: *const T) -> u64
{
	0
}
pub fn get_info<T>(addr: *const T) -> Option<(u64, ProtectionMode)>
{
	None
}


pub fn can_map_without_alloc(addr: *mut ()) -> bool
{
	false
}
pub unsafe fn map(addr: *const (), phys: u64, prot: ProtectionMode)
{
	todo!("map");
}
pub unsafe fn reprotect(addr: *const (), prot: ProtectionMode)
{
	todo!("reprotect");
}
pub unsafe fn unmap(addr: *const ()) -> Option<u64>
{
	None
}


pub unsafe fn fixed_alloc(phys: u64, count: usize) -> Option<*mut ()>
{
	None
}
pub fn is_fixed_alloc(addr: *const (), count: usize) -> bool
{
	false
}


pub unsafe fn temp_map<T>(phys: u64) -> *mut T
{
	todo!("");
}
pub unsafe fn temp_unmap<T>(addr: *mut T)
{
	todo!("");
}


impl AddressSpace
{
	pub fn pid0() -> AddressSpace
	{
		extern "C" {
			static kernel_root: [u64; 2048];
		}
		AddressSpace(kernel_root[2048-2] & !0x3FFF)
	}
	pub fn new(start: usize, end: usize) -> Result<AddressSpace,()>
	{
		todo!("");
	}

	pub fn as_phys(&self) -> u64 {
		self.0
	}
}

