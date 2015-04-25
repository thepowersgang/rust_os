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
		// 1. A single page within the required number of bits
		if phys % (::PAGE_SIZE as PAddr) + bytes < (::PAGE_SIZE as PAddr) && phys >> (bits as usize) == 0
		{
			DMABuffer {
				source_slice: src,
				phys: phys,
			}
		}
		else
		{
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
}
