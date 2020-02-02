//
//
//
//! Controls a region of memory with a bump allocator, for runtime delegaion of address space
use arch::memory::addresses::{BUMP_START, BUMP_END};
use core::sync::atomic::{AtomicUsize, Ordering};

static CURPOS: AtomicUsize = AtomicUsize::new( BUMP_START );

#[derive(Debug)]
pub struct Error;

pub fn delegate(num_pages: usize) -> Result<*mut (), Error>
{
	#[cfg(not(feature="test"))]
	loop
	{
		let cur = CURPOS.load(Ordering::Acquire);
		let new = cur + num_pages * ::PAGE_SIZE;
		assert!(new >= cur);
		if new > BUMP_END {
			return Err(Error);
		}
		
		if cur == CURPOS.compare_and_swap(cur, new, Ordering::Acquire) {
			return Ok(cur as *mut _);
		}
	}
	#[cfg(feature="test")]
	unsafe
	{
		let ptr = ::std::alloc::alloc(::std::alloc::Layout::from_size_align(crate::PAGE_SIZE * num_pages, crate::PAGE_SIZE).unwrap());
		Ok(ptr as *mut ())
	}
}

