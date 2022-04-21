//
//
//
//! 

pub struct PageArray<T>
{
	start: *mut T,
	//capacity: usize,
	end: *mut T,
}
unsafe impl<T: Send> Send for PageArray<T> {}
unsafe impl<T: Sync> Sync for PageArray<T> {}
impl<T> PageArray<T>
{
	pub const fn new(start: usize, end: usize) -> PageArray<T> {
		//assert_eq!( PAGE_SIZE % ::core::mem::size_of::<T>() == 0, "Creating a PageArray<{}> isn't possible - doesn't fit evenly", type_name!(T) );
		PageArray {
			start: start as *mut _,
			end: end as *mut _,
			}
	}

	pub fn capacity(&self) -> usize {
		(self.end as usize - self.start as usize) / ::core::mem::size_of::<T>()
	}

	pub fn get(&self, idx: usize) -> Option<&T> {
		if idx > self.capacity() {
			None
		}
		else {
			// SAFE: Pointer is in range, and validity is checked. Lifetime valid due to self owning area
			unsafe {
				let ptr = self.start.offset(idx as isize);
				if crate::memory::virt::is_reserved(ptr) {
					Some(&*ptr)
				}
				else {
					None
				}
			}
		}
	}

	pub fn get_alloc(&mut self, idx: usize) -> &mut T
	where
		T: Default
	{
		let per_page = crate::PAGE_SIZE / ::core::mem::size_of::<T>();
		let pgidx = idx / per_page;
		let pgofs = idx % per_page;
		let page = (self.start as usize + pgidx * crate::PAGE_SIZE) as *mut T;
		if ! crate::memory::virt::is_reserved( page )
		{
			// TODO: Handle OOM gracefully
			crate::memory::virt::allocate( page as *mut (), 1 ).expect("Failed to allocate memory for PageArray");

			// SAFE: Newly allocated, and nothing valid in it
			unsafe {
				for i in 0 .. per_page {
					let p = page.offset(i as isize);
					::core::ptr::write(p, Default::default());
				}
			}
		}
		// SAFE: Valid, and lifetimes keep it valid
		unsafe {
			&mut *page.offset(pgofs as isize)
		}
	}
}
