// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/heap-test.rs
//! - Simple copy of the `heap` module that wraps libstd
pub unsafe fn alloc<T>(v: T) -> *mut T {
	let p = std::alloc::alloc(std::alloc::Layout::for_value(&v)) as *mut T;
	core::ptr::write(p, v);
	p
}
pub unsafe fn alloc_raw(size: usize, align: usize) -> *mut () {
	std::alloc::alloc(std::alloc::Layout::from_size_align(size,align).unwrap()) as *mut ()
}
pub unsafe fn dealloc_raw(ptr: *mut (), size: usize, align: usize) {
	std::alloc::dealloc(ptr as *mut u8, std::alloc::Layout::from_size_align(size,align).unwrap());
}


pub struct ArrayAlloc<T>
{
	mem: Option<Box<[T]>>,
}
impl<T> ArrayAlloc<T>
{
	pub const fn empty() -> ArrayAlloc<T> {
		ArrayAlloc {
			mem: None,
			}
	}
	pub fn new(cap: usize) -> ArrayAlloc<T> {
		ArrayAlloc {
			mem: {
				let mut v = Vec::with_capacity(cap);
				// SAFE: set_len called again before drop, contents enforced by user
				unsafe { v.set_len(cap); }
				Some(v.into_boxed_slice())
				},
			}
	}
	pub unsafe fn from_raw(ptr: *mut T, count: usize) -> ArrayAlloc<T> {
		ArrayAlloc {
			mem: Some(Vec::from_raw_parts(ptr, count, count).into_boxed_slice())
			}
	}
	pub fn into_raw(self) -> *mut [T] {
		todo!("ArrayAlloc::into_raw");
	}

	pub fn expand(&mut self, _new_size: usize) -> bool {
		false
	}
	pub fn shrink(&mut self, _new_size: usize) {
	}

	pub fn count(&self) -> usize {
		self.mem.as_ref().map(|v| v.len()).unwrap_or(0)
	}
	pub fn get_base(&self) -> *const T {
		match &self.mem
		{
		None => ::std::mem::align_of::<T>() as *const _,
		Some(v) => v.as_ptr(),
		}
	}
	pub fn get_base_mut(&mut self) -> *mut T {
		match &mut self.mem
		{
		None => ::std::mem::align_of::<T>() as *mut _,
		Some(v) => v.as_mut_ptr(),
		}
	}
	pub fn get_ptr(&self, ofs: usize) -> *const T {
		assert!(ofs < self.count());
		// SAFE: Valid offset
		unsafe {
			self.get_base().offset(ofs as isize)
		}
	}
	pub fn get_ptr_mut(&mut self, ofs: usize) -> *mut T {
		assert!(ofs < self.count());
		// SAFE: Valid offset
		unsafe {
			self.get_base_mut().offset(ofs as isize)
		}
	}
}
impl<T> Drop for ArrayAlloc<T>
{
	fn drop(&mut self)
	{
		if let Some(ref mut v) = self.mem
		{
			let boxed = ::std::mem::replace(v, Box::new([]));
			let mut v = Vec::from(boxed);
			// SAFE: Setting to 0, will leak
			unsafe { v.set_len(0); }
			drop(v);
		}
	}
}
