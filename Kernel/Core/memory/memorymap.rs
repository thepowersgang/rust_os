// "Tifflin" Kernel
//
//
use _common::*;
use arch::memory::{PAddr};

#[deriving(Show)]
pub enum MemoryState
{
	StateReserved,
	StateUsed,
	StateFree,
}

pub static MAP_PAD: MemoryMapEnt = MemoryMapEnt {
	start: 0,
	size: 0,
	state: StateReserved,
	domain: 0
	};

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
	size: uint,
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
	pub fn size(&self) -> uint
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
	
	pub fn set_range(&mut self, base: u64, size: u64, state: MemoryState, domain: u16)
	{
		// 1. Locate position
		let pos = self._find_addr(base);
		
		if pos == self.size {
			log_debug!("set_range - {:#x} past end of map, not setting", base);
			return ;
		}
		if self.slots[pos].start > base {
			log_debug!("set_range - {:#x} in a memory hole, not setting", base);
			return ;
		}
		
		fail!("TODO: MemoryMapBuilder.set_range(base={:#x}, size={:#x}, state={}, domain={}",
			base, size, state, domain);
	}
	
	fn _find_addr(&self, addr: u64) -> uint
	{
		for (i,slot) in self.slots.slice(0,self.size).iter().enumerate()
		{
			if slot.start + slot.size > addr {
				return i;
			}
		}
		return self.size
	}
}

// vim: ft=rust

