// "Tifflin" Kernel
//
//
use _common::*;
use arch::memory::{PAddr};

#[derive(PartialEq,Show,Copy)]
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

#[derive(Copy)]
pub struct MemoryMapEnt
{
	pub start: PAddr,
	pub size: PAddr,
	pub state: MemoryState,
	pub domain: u16,
}

pub struct MemoryMapBuilder<'buf>
{
	slots: &'buf mut [MemoryMapEnt],
	size: usize,
}

impl ::core::fmt::Show for MemoryMapEnt
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
		for entid in range(0, self.size)
		{
			let mut sel = entid;
			for tgt in range(entid+1, self.size)
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
		for i in range(0, self.size-1)
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
		let base = base_ - ofs;
		let size = (size_ + ofs + page_mask) & !page_mask;
		
		// 1. Locate position
		let mut pos = self._find_addr(base);
		
		if pos == self.size {
			log_debug!("set_range - {:#x} past end of map, not setting", base);
			return Ok( () );
		}
		if self.slots[pos].start > base {
			log_debug!("set_range - {:#x} in a memory hole, not setting", base);
			return Ok( () );
		}
	
		if base + size <= self.slots[pos].start + self.slots[pos].size
		{
			if self.slots[pos].state != state || self.slots[pos].domain != domain
			{
				// Split (possibly) two times to create a block corresponding to the marked range
				if self.slots[pos].start != base
				{
					let leftsize = base - self.slots[pos].start;
					try!(self._split_at(pos, leftsize));
					pos += 1;
				}
				
				if self.slots[pos].size > size
				{
					try!( self._split_at(pos, size) );
				}
				self.slots[pos].state = state;
				self.slots[pos].domain = domain;
				
				//self.compact();
			}
			
		}
		else
		{
			panic!("TODO: Support marking range across multiple map entries: [{:#x}+{:#x}]",
				self.slots[pos].start, self.slots[pos].size);
		}	
	
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
			for i in range(index+1, self.size).rev()
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

