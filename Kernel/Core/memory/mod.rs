//
//
//
use arch::memory::{VAddr};
use core::ptr::RawPtr;

pub use self::memorymap::{MAP_PAD, MemoryMapEnt, MemoryMapBuilder};
pub use self::memorymap::MemoryState;

pub mod phys;
pub mod virt;
pub mod heap;

pub mod memorymap;

pub fn c_string_valid(c_str: *const i8) -> bool
{
	// 1. Check first page
	if ! ::arch::memory::virt::is_reserved(c_str as VAddr) {
		return false;
	}
	
	unsafe
	{
		let mut ptr = c_str;
		while *ptr != 0
		{
			ptr = ptr.offset(1);
			if ptr as uint % ::PAGE_SIZE == 0
			{
				if ! ::arch::memory::virt::is_reserved(ptr as VAddr) {
					return false;
				}
			}
		}
	}
	
	true
}

pub fn buf_valid(ptr: *const (), mut size: uint) -> bool
{
	let mut addr = ptr as VAddr;
	if ! ::arch::memory::virt::is_reserved(addr) {
		return false;
	}
	let rem_ofs = ::PAGE_SIZE - addr % ::PAGE_SIZE;
	
	if size > rem_ofs
	{
		addr += rem_ofs;
		size -= rem_ofs;
		while size != 0
		{
			if ! ::arch::memory::virt::is_reserved(addr) {
				return false;
			}
			if size > ::PAGE_SIZE {
				size -= ::PAGE_SIZE;
				addr += ::PAGE_SIZE;
			}
			else {
				break;
			}
		}
	}
	
	true
}

// vim: ft=rust

