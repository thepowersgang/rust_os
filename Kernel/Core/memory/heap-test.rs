
pub unsafe fn alloc<T>(_v: T) -> *mut T {
	todo!("heap::alloc");
}
pub unsafe fn alloc_raw(_size: usize, _align: usize) -> *mut () {
	todo!("heap::alloc_raw");
}
pub unsafe fn dealloc_raw(_ptr: *mut (), _size: usize, _align: usize) {
	todo!("heap::dealloc_raw");
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

	pub fn expand(&mut self, new_size: usize) -> bool {
		false
	}
	pub fn shrink(&mut self, new_size: usize) {
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
