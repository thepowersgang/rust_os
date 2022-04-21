// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/memorymap.rs
//! Physical memory map type
#[allow(unused_imports)]
use crate::prelude::*;

#[derive(PartialEq,Debug,Copy,Clone)]
pub enum MemoryState
{
	/// Reserved for use by the firmware
	Reserved,
	/// Used by the kernel already
	Used,
	/// Free for use
	Free,
}

/// A constant default/padding map entry
pub const MAP_PAD: MemoryMapEnt = MemoryMapEnt {
	start: 0,
	size: 0,
	state: MemoryState::Reserved,
	domain: 0
	};

/// Memory map entry
#[derive(Copy,Clone)]
pub struct MemoryMapEnt
{
	/// First address in the map
	pub start: u64,
	/// Size of the memory
	pub size: u64,
	/// Memory class
	pub state: MemoryState,
	/// NUMA domain for this memory
	pub domain: u16,
}

/// Helper to handle building a memory map (by masking out segments)
pub struct MemoryMapBuilder<'buf>
{
	slots: &'buf mut [MemoryMapEnt],
	size: usize,
}

impl MemoryMapEnt
{
	pub fn end(&self) -> u64 {
		self.limit() + 1
	}
	pub fn limit(&self) -> u64 {
		self.start + (self.size & !(crate::arch::memory::PAGE_MASK as u64)) - 1
	}
}
impl ::core::fmt::Debug for MemoryMapEnt
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "{:#x}+{:#x} {:?} Domain {}", self.start, self.size, self.state, self.domain)
	}
}

impl<'buf> MemoryMapBuilder<'buf>
{
	/// Construct a new builder using the provided backing memory
	pub fn new<'a>(buf: &'a mut [MemoryMapEnt]) -> MemoryMapBuilder<'a>
	{
		MemoryMapBuilder {
			slots: buf,
			size: 0,
		}
	}
	/// Currently populated size of the map
	pub fn size(&self) -> usize
	{
		self.size
	}
	
	/// Add a new (base) entry to the map
	pub fn append(&mut self, base: u64, size: u64, state: MemoryState, domain: u16)
	{
		log_debug!("append(base={:#x}, size={:#x}, state={:?}, domain={})",
			base, size, state, domain);
		self.slots[self.size] = MemoryMapEnt {
			start: base,
			size: size,
			state: state,
			domain: domain,
			};
		self.size += 1;
	}
	
	/// Sort all entries
	pub fn sort(&mut self)
	{
		self.slots[..self.size].sort_unstable_by_key(|v| v.start);
	}
	/// Validate the memory map's consistency (no overlapping entries)
	pub fn validate(&self) -> bool
	{
		let mut ret = true;
		for i in 0 .. self.size-1
		{
			if self.slots[i].start + self.slots[i].size > self.slots[i+1].start
			{
				log_error!("Map slot #{} overlaps with next ({:#x} + {:#x} > {:#x}",
					i, self.slots[i].start, self.slots[i].size, self.slots[i+1].start);
				ret = false;
			}
		}
		ret
	}
	/// Compact the map (combine abutting entries)
	pub fn compact(&mut self)
	{
		self.sort();
		let mut it = self.slots[..self.size].iter_mut();
		if let Some(mut cur) = it.next()
		{
			for next in it
			{
				let mut advance = true;
				// If the current overlaps with or is abuts next entry 
				if cur.start + cur.size >= next.start
				{
					let overlap = (cur.start + cur.size) - next.start;
					if cur.state == next.state && cur.domain == next.domain {
						// Potentially overlapping, and same - merge
						if next.size >= overlap {
							cur.size += next.size - overlap;
						}
						next.size = 0;
						advance = false;
					}
					else if overlap > 0 {
						// Overlapping and different!
						// - Print and error
					}
					else {
						// Adjacent and different
					}
				}
				if advance
				{
					cur = next;
				}
			}
		}
		//log_trace!("compact: {:?}", &self.slots[..self.size]);

		// Remove all zero-sized entries
		let mut w_i = 1;
		for r_i in 1 .. self.size
		{
			if self.slots[r_i].size == 0
			{
				// Ignore
			}
			else
			{
				if r_i > w_i
				{
					self.slots[w_i] = self.slots[r_i];
				}
				w_i += 1;
			}
		}
		self.size = w_i;
		//log_trace!("compact: {:?}", &self.slots[..self.size]);
	}
	
	/// Update the state of a specified range of memory
	/// NOTE: This rounds all actions to page boundaries
	pub fn set_range(&mut self, base_: u64, size_: u64, state: MemoryState, domain: u16) -> Result<(),()>
	{
		log_debug!("set_range(base={:#x}, size={:#x}, state={:?}, domain={})",
			base_, size_, state, domain);
		
		let page_mask = crate::PAGE_SIZE as u64 - 1;
		let ofs = base_ & page_mask;
		let mut base = base_ - ofs;
		let mut size = (size_ + ofs + page_mask) & !page_mask;
		
		// 1. Locate position
		let mut pos = self._find_addr(base);
		
		if pos == self.size {
			log_debug!("set_range - {:#x} past end of map, not setting", base);
			return Ok( () );
		}
		if self.slots[pos].start > base {
			// TODO: Handle range overlapping end of a hole
			log_debug!("set_range - {:#x} in a memory hole, not setting", base);
			return Ok( () );
		}
		
		let first = pos;
		while pos < self.size && size > 0 {
			let mut slot = self.slots[pos];
			log_debug!("{}: ({:#x}+{:#x}): slot={:?}", pos, base, size, slot);
			if slot.start > base {
				// If this slot starts after the base, then consume enough to set `base` to the start
				let seg_size = ::core::cmp::min(size, slot.start - base);
				base += seg_size;
				size -= seg_size;
			}
			else {
				let slot_ofs = base - slot.start;
				let seg_size = ::core::cmp::min(size, slot.size - slot_ofs);
				if slot.state != state {
					// Altered region starts within block
					// - Split that block, advance `pos`, and then do other checks
					if slot.start < base {
						let leftsize = base - slot.start;
						self._split_at(pos, leftsize)?;
						pos += 1;
						slot = self.slots[pos];
					}

					// Altered region ends within block
					// - Split the block
					if slot.size > size {
						self._split_at(pos, size)?;
						base += size;
						size = 0;
					}
					// Altered region encompasses block
					else {
						base += seg_size;
						size -= seg_size;
					}
					// Update the state of the entire block (known to be equal to the current range)
					self.slots[pos].state = state;
					self.slots[pos].domain = domain;
				}
				else {
					// Already same state, no change
					base += seg_size;
					size -= seg_size;
				}
				pos += 1;
			}
		}
		// Print all of the updated slots
		for pos in first..pos
		{
			log_debug!("={}: slot={:?}", pos, self.slots[pos]);
		}

		// Merge ajoining regions
		self.compact();
	
		Ok( () )
	}
	
	/// Locate the slot for a given address
	fn _find_addr(&self, addr: u64) -> usize
	{
		for (i,slot) in self.slots[0 .. self.size].iter().enumerate()
		{
			if slot.start + slot.size > addr {
				return i;
			}
		}
		return self.size
	}
	
	/// Split the given slot into two equivalent slots
	fn _split_at(&mut self, index: usize, left_size: u64) -> Result<(),()>
	{
		if self.size >= self.slots.len()
		{
			Err( () )
		}
		else
		{
			assert!(self.slots[index].size > left_size);
			for i in (index+1 .. self.size).rev()
			{
				self.slots[i+1] = self.slots[i];
			}
			self.slots[index+1].start = self.slots[index].start + left_size;
			self.slots[index+1].size = self.slots[index].size - left_size;
			self.slots[index+1].state = self.slots[index].state;
			self.slots[index+1].domain = self.slots[index].domain;
			self.slots[index].size = left_size;
			self.size += 1;
			
			Ok( () )
		}
	}
}

// vim: ft=rust

