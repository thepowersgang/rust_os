/*
 */
///! Helper type for DMA accesses
use prelude::*;
use arch::memory::PAddr;
use core::marker::PhantomData;

/**
 * A buffer garunteed to be in a certain area of physical memory
 */
pub struct DMABuffer<'a>
{
	_marker: PhantomData<&'a mut [u8]>,
	source_ptr: *mut u8,
	buffer_len: usize,
	phys: PAddr,
}

impl<'a> DMABuffer<'a>
{
	/// Creates a new DMABuffer contigious in the specified region
	pub fn new_contig_mut(src: &mut [u8], bits: u8) -> DMABuffer {
		DMABuffer::new_contig(src, bits)
	}
	pub fn new_contig(src: &[u8], bits: u8) -> DMABuffer
	{
		use arch::memory::PAddr;
		let bytes = src.len() as PAddr;
		let phys = ::memory::virt::get_phys( &src[0] );
		let end_phys = ::memory::virt::get_phys( &src[src.len()-1] );
		// Check if the buffer is within the required bits
		if phys >> (bits as usize) != 0
		{
			todo!("new_contig - Bounce because not within bit range");	
		}
		// - Quick: If the data is smaller than a page worth, and falls on a contigious pair of pages
		else if bytes <= ::PAGE_SIZE as u64 && phys + bytes-1 == end_phys
		{
			log_debug!("phys = {:#x}, source_slice={:p}", phys, &src[0]);
			DMABuffer {
				_marker: PhantomData,
				source_ptr: src.as_ptr() as *mut _,
				buffer_len: bytes as usize,
				phys: phys,
			}
		}
		else
		{
			todo!("Handle non-contig source buffer ({:#x}+{} != {:#x})", phys, bytes-1, end_phys);
		}
	}
	
	pub fn len(&self) -> usize {
		self.buffer_len
	}	

	pub fn phys(&self) -> ::arch::memory::PAddr {
		self.phys
	}
	
	pub fn update_source(&mut self) {
		if self.phys != ::memory::virt::get_phys(self.source_ptr) {
			unimplemented!();
		}
	}
}
