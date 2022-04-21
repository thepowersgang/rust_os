/// Access to user-controlled memory
///

pub fn read<T: crate::lib::POD>(addr: usize) -> Result<T, ()> {
	assert!( ::core::mem::size_of::<T>() < crate::PAGE_SIZE );
	let ptr = addr as *const T;
	if ! crate::memory::buf_valid(ptr as *const (), ::core::mem::size_of::<T>()) {
		Err( () )
	}
	else if addr % ::core::mem::align_of::<T>() != 0 {
		Err( () )
	}
	else {
		// TODO: XXX Handle potential for user to alter the AS during this
		// SAFE: (Assuming single-thread) Alignment and validity checked
		unsafe {
			Ok( ::core::ptr::read(addr as *const T) )
		}
	}
}
