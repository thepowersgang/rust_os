//
//
//
use arch::memory::{VAddr};
use core::option::Option::{self,None,Some};

pub use self::memorymap::{MAP_PAD, MemoryMapEnt, MemoryMapBuilder};
pub use self::memorymap::MemoryState;

pub use arch::memory::PAddr;
pub mod phys;
pub mod virt;
pub mod heap;

pub mod helpers;

pub mod memorymap;

/// Validate that a C string points to valid memory, and return a 'a slice to it
pub fn c_string_as_byte_slice<'a>(c_str: *const i8) -> Option<&'a [u8]>
{
	// 1. Check first page
	if ! ::arch::memory::virt::is_reserved(c_str) {
		return None;
	}
	
	unsafe
	{
		let mut ptr = c_str;
		while *ptr != 0
		{
			ptr = ptr.offset(1);
			if ptr as usize % ::PAGE_SIZE == 0
			{
				if ! ::arch::memory::virt::is_reserved(ptr) {
					return None;
				}
			}
		}
		
		Some( ::core::slice::from_raw_parts(c_str as *const u8, ptr as usize - c_str as usize) )
	}	
}
/// Validate a C string (legacy)
//#[deprecated="Use ::memory::c_string_as_byte_slice instead"]
pub fn c_string_valid(c_str: *const i8) -> bool
{
	c_string_as_byte_slice(c_str).is_some()
}

// UNSAFE: Lifetime is inferred, everything else is checked
pub unsafe fn buf_to_slice<'a, T>(ptr: *const T, size: usize) -> Option<&'a [T]> {
	
	if ptr as usize % ::core::mem::align_of::<T>() != 0 {
		None
	}
	else if ! buf_valid(ptr as *const (), size) {
		None
	}
	else {
		Some( ::core::slice::from_raw_parts(ptr, size) )
	}
}
pub unsafe fn buf_to_slice_mut<'a, T>(ptr: *mut T, size: usize) -> Option<&'a mut [T]> {
	
	if ptr as usize % ::core::mem::align_of::<T>() != 0 {
		None
	}
	else if ! buf_valid(ptr as *const (), size) {
		None
	}
	else {
		Some( ::core::slice::from_raw_parts_mut(ptr, size) )
	}
}

/// Validates that a buffer points to accessible memory
pub fn buf_valid(ptr: *const (), mut size: usize) -> bool
{
	let mut addr = ptr as VAddr;
	if ! ::arch::memory::virt::is_reserved(ptr) {
		return false;
	}
	let rem_ofs = ::PAGE_SIZE - addr % ::PAGE_SIZE;
	
	if size > rem_ofs
	{
		addr += rem_ofs;
		size -= rem_ofs;
		while size != 0
		{
			if ! ::arch::memory::virt::is_reserved(addr as *const ()) {
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

