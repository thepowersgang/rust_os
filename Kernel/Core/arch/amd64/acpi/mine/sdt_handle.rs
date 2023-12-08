
use super::SDT;
use super::super::SDTHeader;

/// A handle to a SDT
pub struct SDTHandle<T:'static>
{
	maphandle: crate::memory::virt::AllocHandle,
	ofs: usize,
	_type: ::core::marker::PhantomData<T>,
}

impl<T: crate::lib::POD> SDTHandle<T>
{
	/// Map an SDT into memory, given a physical address
	pub unsafe fn new(physaddr: u64) -> SDTHandle<T>
	{
		//log_trace!("new(physaddr={:#x})", physaddr);
		let ofs = (physaddr & (crate::PAGE_SIZE - 1) as u64) as usize;
		
		// Obtain length (and validate)
		// TODO: Support the SDT header spanning across two pages
		assert!(crate::PAGE_SIZE - ofs >= ::core::mem::size_of::<SDTHeader>());
		// Map the header into memory temporarily (maybe)
		let mut handle = match crate::memory::virt::map_hw_ro(physaddr - ofs as u64, 1, "ACPI") {
			Ok(v) => v,
			Err(_) => panic!("Oops, temp mapping SDT failed"),
			};
		let (length,) = {
			let hdr = handle.as_ref::<SDTHeader>(ofs);
			
			// Get the length
			(hdr.length as usize,)
			};
		
		// Map the resultant memory
		let npages = (ofs + length + crate::PAGE_SIZE - 1) / crate::PAGE_SIZE;
		log_trace!("npages = {}, ofs = {}, length = {}", npages, ofs, length);
		if npages != 1
		{
			handle = match crate::memory::virt::map_hw_ro(physaddr - ofs as u64, npages, "ACPI") {
				Ok(x) => x,
				Err(_) => panic!("Map fail")
				};
		}
		SDTHandle {
			maphandle: handle,
			ofs,
			_type: ::core::marker::PhantomData,
			}
	}
	
	pub fn make_static(self) -> &'static SDT<T>
	{
		self.maphandle.make_static::<SDT<T>>(self.ofs)
	}
}

impl<T: crate::lib::POD> ::core::ops::Deref for SDTHandle<T>
{
	type Target = SDT<T>;
	fn deref<'s>(&'s self) -> &'s SDT<T> {
		self.maphandle.as_ref(self.ofs)
	}
}
