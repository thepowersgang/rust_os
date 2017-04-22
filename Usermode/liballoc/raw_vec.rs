/// Wrapper around ArrayAlloc

pub struct RawVec<T>(::array::ArrayAlloc<T>);
impl<T> RawVec<T> {
	pub fn new() -> RawVec<T> {
		RawVec( ::array::ArrayAlloc::new(0) )
	}
	pub fn with_capacity(cap: usize) -> RawVec<T> {
		RawVec( ::array::ArrayAlloc::new(cap) )
	}
	pub fn with_capacity_zeroed(cap: usize) -> RawVec<T> {
		let rv = RawVec( ::array::ArrayAlloc::new(cap) );
		// SAFE: Access in bounds, shouldn't be read without upper unsafe ensuring safety of zero value
		unsafe {
			::core::ptr::write_bytes(rv.ptr(), 0, rv.cap() * ::core::mem::size_of::<T>());
		}
		rv
	}
	pub unsafe fn from_raw_parts(base: *mut T, size: usize) -> RawVec<T> {
		RawVec( ::array::ArrayAlloc::from_raw_parts(base, size) )
	}
	pub fn cap(&self) -> usize {
		self.0.count()
	}
	pub fn ptr(&self) -> *mut T {
		self.0.get_base() as *mut T
	}
	pub fn shrink_to_fit(&mut self, used: usize) {
		self.0.resize(used);
	}
	pub fn reserve(&mut self, cur_used: usize, extra: usize) {
		let newcap = cur_used + extra;
		if newcap < self.cap() {
			
		}
		else {
			self.0.resize(newcap);
		}
	}
	pub fn reserve_exact(&mut self, cur_used: usize, extra: usize) {
		let newcap = cur_used + extra;
		if newcap < self.cap() {
			
		}
		else {
			self.0.resize(newcap);
		}
	}
	pub fn double(&mut self) {
		//kernel_log!("RawVec::<{}>::double()", type_name!(T));
		if self.cap() == 0 {
			self.0.resize(1);
		}
		else {
			let newcap = self.cap() * 2;
			self.0.resize(newcap);
		}
	}
	pub fn into_box(self) -> ::boxed::Box<[T]> {
		todo!("into_box");
	}
}

