/*
 */
///! Helper type for DMA accesses

use core::slice::SliceExt;
use arch::memory::PAddr;

/**
 * A buffer garunteed to be in a certain area of physical memory
 */
pub struct DMABuffer<'a>
{
	source_slice: &'a mut [u8],
	phys: PAddr,
}

impl<'a> DMABuffer<'a>
{
	/// Creates a new DMABuffer contigious in the specified region
	pub fn new_contig(src: &mut [u8], bits: u8) -> DMABuffer
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
				source_slice: src,
				phys: phys,
			}
		}
		else
		{
			todo!("Handle non-contig source buffer ({:#x}+{} != {:#x})", phys, bytes-1, end_phys);
			unimplemented!();
		}
	}
	
	pub fn len(&self) -> usize {
		self.source_slice.len()
	}	

	pub fn phys(&self) -> ::arch::memory::PAddr
	{
		self.phys
	}
	
	pub fn update_source(&mut self) {
		if self.phys != ::memory::virt::get_phys( &self.source_slice[0] ) {
			unimplemented!();
		}
	}
}
