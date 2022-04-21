// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/helpers/dma_buffer.rs
///! Helper type for DMA accesses
#[allow(unused_imports)]
use crate::prelude::*;
use crate::arch::memory::PAddr;
use crate::PAGE_SIZE;
use crate::memory::virt;
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
impl<'a> !Send for DMABuffer<'a> {}
impl<'a> !Sync for DMABuffer<'a> {}

impl<'a> ::core::fmt::Debug for DMABuffer<'a> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "{:p}({:x})+{}", self.source_ptr, self.phys, self.buffer_len)
	}
}

impl<'a> DMABuffer<'a>
{
	fn check_bits(src: &[u8], bits: u8) -> bool {
		let vaddr = src.as_ptr() as usize;
		let ofs = vaddr % PAGE_SIZE;
		
		if bits as usize >= ::core::mem::size_of::<PAddr>()*8 {
			return true;
		}
		if virt::get_phys(src.as_ptr()) >> (bits as usize) != 0 {
			return false;
		}
		
		if PAGE_SIZE - ofs < src.len()
		{
			for page in src[PAGE_SIZE - ofs..].chunks(PAGE_SIZE)
			{
				let phys = virt::get_phys(page.as_ptr());
				if phys >> (bits as usize) != 0 {
					return false;
				}
			}
		}
		true
	}
	
	pub fn new_mut(src: &mut [u8], bits: u8) -> DMABuffer {
		DMABuffer::new(src, bits)
	}
	pub fn new(src: &[u8], bits: u8) -> DMABuffer {
		if Self::check_bits(src, bits) == false {
			todo!("new - Bounce because not within bit range");	
		}
		else {
			DMABuffer {
				_marker: PhantomData,
				source_ptr: src.as_ptr() as *mut _,
				buffer_len: src.len(),
				phys: virt::get_phys(src.as_ptr()),
			}
		}
	}
	
	/// Creates a new DMABuffer contigious in the specified region
	pub fn new_contig_mut(src: &mut [u8], bits: u8) -> DMABuffer {
		DMABuffer::new_contig(src, bits)
	}
	pub fn new_contig(src: &[u8], bits: u8) -> DMABuffer
	{
		let bytes = src.len();
		let phys = virt::get_phys( &src[0] );
		let end_phys = virt::get_phys( &src[src.len()-1] );
		// Check if the buffer is within the required bits
		if Self::check_bits(src, bits) == false
		{
			todo!("new_contig - Bounce because not within bit range");	
		}
		// - Quick: If the data is smaller than a page worth, and falls on a contigious pair of pages
		else if bytes <= PAGE_SIZE && phys + (bytes as PAddr)-1 == end_phys
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
	
	/// Returns an iterator over contigious physical ranges
	pub fn phys_ranges(&self) -> Ranges {
		if self.phys != virt::get_phys(self.source_ptr) {
			unimplemented!();
		}
		else {
			// TODO: Would there be a problem with different address spaces? No, not Send
			// SAFE: Borrows self, and pointer is valid (casted out in construction)
			Ranges( unsafe { ::core::slice::from_raw_parts(self.source_ptr, self.buffer_len) } )
		}
	}
	
	pub fn len(&self) -> usize {
		self.buffer_len
	}	

	//#[deprecated]
	//pub fn phys(&self) -> ::arch::memory::PAddr {
	//	self.phys
	//}
	
	pub fn update_source(&mut self) {
		if self.phys != virt::get_phys(self.source_ptr) {
			unimplemented!();
		}
	}
}

pub struct Ranges<'a>(&'a [u8]);
impl<'a> Iterator for Ranges<'a>
{
	type Item = (PAddr,usize);
	fn next(&mut self) -> Option<Self::Item> {
		if self.0.len() == 0 {
			None
		}
		else {
			let rem = PAGE_SIZE - (self.0.as_ptr() as usize) % PAGE_SIZE;
			let len = ::core::cmp::min(rem, self.0.len());
			let paddr = virt::get_phys(self.0.as_ptr());
			self.0 = &self.0[len..];
			Some( (paddr, len) )
		}
	}
}
impl<'a> DoubleEndedIterator for Ranges<'a>
{
	fn next_back(&mut self) -> Option<Self::Item> {
		if self.0.len() == 0 {
			None
		}
		else {
			let full_len = self.0.len();
			// get phys of last byte
			let lastp: *const u8 = &self.0[full_len-1];
			let min_len = (lastp as usize) % PAGE_SIZE + 1;

			let mut len = ::core::cmp::min(min_len, full_len);
			let mut paddr = virt::get_phys( &self.0[full_len - len] );

			// Merge physically contigious pages
			while len < full_len && virt::get_phys(&self.0[full_len - len - 1]) == paddr - 1 {
				if full_len - len > PAGE_SIZE {
					paddr -= PAGE_SIZE as PAddr;
					len += PAGE_SIZE;
				}
				else {
					paddr -= (full_len - len) as PAddr;
					len = full_len;
				}
			}

			self.0 = &self.0[ .. full_len - len];
			Some( (paddr, len) )
		}
	}
}
