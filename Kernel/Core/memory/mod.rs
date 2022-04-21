//
//
//
use crate::arch::memory::{VAddr};
use core::option::Option::{self,None,Some};

pub use self::memorymap::{MAP_PAD, MemoryMapEnt, MemoryMapBuilder};
pub use self::memorymap::MemoryState;

pub mod phys;
mod phys_track;

pub mod virt;
#[cfg_attr(feature="test", path="heap-test.rs")]
pub mod heap;
// TODO: Merge user and freeze
pub mod user;

pub mod freeze;

pub mod helpers;

pub mod memorymap;

pub mod bump_region;
pub mod page_cache;
pub mod page_array;

pub use crate::arch::memory::PAddr;

/*
#[derive(Copy,Clone,Debug)]
pub struct PAddr(::arch::memory::PAddrRaw);
impl PAddr {
	pub fn as_inner(self) -> ::arch::memory::PAddrRaw {
		self.0
	}
	pub fn to_u32(&self) -> Option<u32> {
		self.0.try_into()
	}
}
impl ::core::ops::Add<usize> for PAddr
{
	type Output = PAddr;
	fn add(self, v: usize) -> PAddr {
		assert!( !0 - self.0 > v as usize, "Overflow adding {} to physical address {:#x}", v, self.0 );
		PAddr( self.0 + v as u32 )
	}
}
impl ::core::ops::Sub<PAddr> for PAddr {
	type Output = isize;
	fn sub(self, v: PAddr) -> isize {
		// TODO: Check that the difference doesn't overflow
		(self.0 - v.0) as isize
	}
}
*/

/// Validate that a C string points to valid memory, and return a 'a slice to it
/// UNSAFE: Lifetime is inferred
pub unsafe fn c_string_as_byte_slice<'a>(c_str: *const i8) -> Option<&'a [u8]>
{
	// 1. Check first page
	if ! crate::arch::memory::virt::is_reserved(c_str) {
		return None;
	}
	
	let mut ptr = c_str;
	while *ptr != 0
	{
		ptr = ptr.offset(1);
		if ptr as usize % crate::PAGE_SIZE == 0
		{
			if ! crate::arch::memory::virt::is_reserved(ptr) {
				return None;
			}
		}
	}
	
	Some( ::core::slice::from_raw_parts(c_str as *const u8, ptr as usize - c_str as usize) )
}
/// Validate a C string (legacy)
//#[deprecated="Use ::memory::c_string_as_byte_slice instead"]
pub fn c_string_valid(c_str: *const i8) -> bool
{
	// SAFE: Pointer is valid for lifetime of input pointer (barring odd input behavior)
	unsafe { c_string_as_byte_slice(c_str).is_some() }
}

// UNSAFE: Lifetime is inferred, and memory must point to a valid T instance
pub unsafe fn buf_to_slice<'a, T>(ptr: *const T, size: usize) -> Option<&'a [T]> {
	if size > 0 && ptr as usize % ::core::mem::align_of::<T>() != 0 {
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
	
	if size > 0 && ptr as usize % ::core::mem::align_of::<T>() != 0 {
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
	if size == 0 {
		if addr == 0 {
			// HACK: Strictly speaking, NULL would be valid according to this method
			//  but checking it here makes life easier in the slice methods
			return false;
		}
		else {
			return true;
		}
	}
	else if ! crate::arch::memory::virt::is_reserved(ptr) {
		return false;
	}
	let rem_ofs = crate::PAGE_SIZE - addr % crate::PAGE_SIZE;
	
	if size > rem_ofs
	{
		addr += rem_ofs;
		size -= rem_ofs;
		while size != 0
		{
			if ! crate::arch::memory::virt::is_reserved(addr as *const ()) {
				return false;
			}
			if size > crate::PAGE_SIZE {
				size -= crate::PAGE_SIZE;
				addr += crate::PAGE_SIZE;
			}
			else {
				break;
			}
		}
	}
	
	true
}


// vim: ft=rust

