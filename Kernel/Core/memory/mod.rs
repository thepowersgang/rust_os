//
//
//
use arch::memory::{VAddr};
use core::ptr::PtrExt;
use core::option::Option::{self,None,Some};

pub use self::memorymap::{MAP_PAD, MemoryMapEnt, MemoryMapBuilder};
pub use self::memorymap::MemoryState;

pub mod phys;
pub mod virt;
pub mod heap;

pub mod memorymap;

/// Validate that a C string points to valid memory, and return a 'static slice to it
pub fn c_string_as_byte_slice(c_str: *const i8) -> Option<&'static [u8]>
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
			if ptr as uint % ::PAGE_SIZE == 0
			{
				if ! ::arch::memory::virt::is_reserved(ptr) {
					return None;
				}
			}
		}
		
		Some( ::core::mem::transmute( ::core::raw::Slice {
			data: c_str,
			len: ptr as uint - c_str as uint,
			}) )
	}	
}
/// Validate a C string (legacy)
#[deprecated="Use ::memory::c_string_as_byte_slice instead"]
pub fn c_string_valid(c_str: *const i8) -> bool
{
	c_string_as_byte_slice(c_str).is_some()
}

/// Validates that a buffer points to accessible memory
pub fn buf_valid(ptr: *const (), mut size: uint) -> bool
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

