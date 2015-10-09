// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/memorymap.rs
//! Physical memory map type
#[allow(unused_imports)]
use prelude::*;

#[derive(PartialEq,Debug,Copy,Clone)]
pub enum MemoryState
{
	Reserved,
	Used,
	Free,
}

pub const MAP_PAD: MemoryMapEnt = MemoryMapEnt {
	start: 0,
	size: 0,
	state: MemoryState::Reserved,
	domain: 0
	};

#[derive(Copy,Clone)]
pub struct MemoryMapEnt
{
	pub start: u64,
	pub size: u64,
	pub state: MemoryState,
	pub domain: u16,
}

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
		self.start + (self.size & !0xFFF) - 1
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
	pub fn new<'a>(buf: &'a mut [MemoryMapEnt]) -> MemoryMapBuilder<'a>
	{
		MemoryMapBuilder {
			slots: buf,
			size: 0,
		}
	}
	pub fn size(&self) -> usize
	{
		self.size
	}
	
	pub fn append(&mut self, base: u64, size: u64, state: MemoryState, domain: u16)
	{
		self.slots[self.size] = MemoryMapEnt {
			start: base,
			size: size,
			state: state,
			domain: domain,
			};
		self.size += 1;
	}
	
	pub fn sort(&mut self)
	{
		for entid in (0 .. self.size)
		{
			let mut sel = entid;
			for tgt in (entid+1 .. self.size)
			{
				if self.slots[sel].start > self.slots[tgt].start {
					sel = tgt;
				}
			}
			if sel != entid
			{
				// Swap
				let a = self.slots[sel];
				self.slots[sel] = self.slots[entid];
				self.slots[entid] = a;
			}
		}
	}
	pub fn validate(&self) -> bool
	{
		let mut ret = true;
		for i in (0 .. self.size-1)
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
	
	pub fn set_range(&mut self, base_: u64, size_: u64, state: MemoryState, domain: u16) -> Result<(),()>
	{
		log_debug!("set_range(base={:#x}, size={:#x}, state={:?}, domain={})",
			base_, size_, state, domain);
		
		let page_mask = ::PAGE_SIZE as u64 - 1;
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
		
		while pos < self.size && size > 0 {
			let slot = self.slots[pos];
			log_debug!("{}: ({:#x}+{:#x}): slot={:?}", pos, base, size, slot);
			if slot.start > base {
				let seg_size = ::core::cmp::min(size, slot.start - base);
				base += seg_size;
				size -= seg_size;
			}
			else {
				let seg_size = ::core::cmp::min(size, slot.size - (base - slot.start));
				if slot.state != state {
					// Altered region starts within block
					if slot.start < base {
						let leftsize = base - slot.start;
						try!(self._split_at(pos, leftsize));
					}
					// Altered region ends within block
					else if slot.size > size {
						try!( self._split_at(pos, size) );
						base += size;
						size = 0;
					}
					// Altered region encompasses block
					else {
						base += seg_size;
						size -= seg_size;
					}
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
		// TODO: Merge ajoining regions
		//self.compact();
	
		Ok( () )
	}
	
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

