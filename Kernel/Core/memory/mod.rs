//
//
//
use arch::memory::{VAddr};
use core::ptr::RawPtr;

pub use self::memorymap::{MAP_PAD, MemoryMapEnt, MemoryMapBuilder};
pub use self::memorymap::{StateReserved,StateUsed,StateFree};

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

// vim: ft=rust

