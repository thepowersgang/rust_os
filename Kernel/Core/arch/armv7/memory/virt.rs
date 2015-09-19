//
//
//

use memory::virt::ProtectionMode;
use arch::memory::PAddr;

const KERNEL_TEMP_BASE : usize = 0xFF800000;
const KERNEL_TEMP_LIMIT: usize = 0xFFFF0000;
const KERNEL_BASE_TABLE: usize = 0xFFFF8000;
const KERNEL_TEMP_TABLE: usize = 0xFFFF9000;

pub fn is_fixed_alloc<T>(addr: *const T, size: usize) -> bool {
	const BASE : usize = super::addresses::KERNEL_BASE;
	const LIMIT: usize = super::addresses::KERNEL_BASE + 4*1024*1024;
	let addr = addr as usize;
	if addr < BASE {
		false
	}
	else if addr >= LIMIT {
		false
	}
	else if addr + size > LIMIT {
		false
	}
	else {
		true
	}
}
// UNSAFE: Can cause aliasing
pub unsafe fn fixed_alloc(p: PAddr, count: usize) -> Option<*mut ()> {
	None
}

#[derive(Copy,Clone)]
enum PageEntryRegion {
	NonGlobal,
	Global,
}
impl PageEntryRegion {
	unsafe fn get_page_ent(&self, idx: usize) -> &mut u32 {
		assert!(idx < (1 << 20));
		match self
		{
		&PageEntryRegion::NonGlobal => todo!("PageEntryRegion::get_page_ent - non-global"),
		&PageEntryRegion::Global => todo!("PageEntryRegion::get_page_ent - global"),
		}
	}
	unsafe fn get_section_ent(&self, idx: usize) -> &mut u32 {
		assert!(idx < 4096);
		match self
		{
		&PageEntryRegion::NonGlobal => todo!("PageEntryRegion::get_section_ent - non-global"),
		&PageEntryRegion::Global => &mut *((KERNEL_BASE_TABLE + idx * 4) as *mut u32),
		}
	}
}
enum PageEntry {
	Section {
		rgn: PageEntryRegion,
		idx: usize,
		ofs: usize
		},
	Page {
		rgn: PageEntryRegion,
		idx: usize,
		ofs: usize
		},
}
impl PageEntry
{
	fn alloc(addr: *const (), level: usize) -> Result<PageEntry, ()> {
		todo!("PageEntry::alloc");
	}
	fn get(addr: *const ()) -> PageEntry {
		use super::addresses::KERNEL_BASE;
		let (rgn, p_idx) = if (addr as usize) < KERNEL_BASE {
				(PageEntryRegion::NonGlobal, (addr as usize - KERNEL_BASE) >> 12)
			}
			else {
				(PageEntryRegion::Global, (addr as usize) >> 12)
			};

		// SAFE: Aliasing in this case is benign
		if unsafe { *rgn.get_section_ent(p_idx >> 8) } & 0b11 == 0b01 {
			PageEntry::Page {
				rgn: rgn,
				idx: p_idx,
				ofs: (addr as usize) & 0xFFF,
				}
		}
		else {
			PageEntry::Section {
				rgn: rgn,
				idx: p_idx >> 8,
				ofs: (addr as usize) & 0xFF_FFF,
				}
		}
	}


	fn is_reserved(&self) -> bool {
		// TODO: Need to _ensure_ that the page table is not removed during manipulation
		// SAFE: Aliasing is benign, and page table should be mapped (see above TODO)
		unsafe {
			match self
			{
			&PageEntry::Section { rgn, idx, .. } => (*rgn.get_section_ent(idx) & 3 != 0),
			&PageEntry::Page { rgn, idx, .. } => (*rgn.get_page_ent(idx) & 3 != 0),
			}
		}
	}

	fn phys_addr(&self) -> ::arch::memory::PAddr {
		todo!("PageEntry::phys_addr");
	}
}

pub fn is_reserved<T>(addr: *const T) -> bool {
	PageEntry::get(addr as *const ()).is_reserved()
}
pub fn get_phys<T>(addr: *const T) -> ::arch::memory::PAddr {
	PageEntry::get(addr as *const ()).phys_addr()
}

pub fn get_info<T>(addr: *const T) -> Option<(u32, ::memory::virt::ProtectionMode)> {
	todo!("get_info")
}

pub unsafe fn map(a: *mut (), p: PAddr, mode: ProtectionMode) {
}
pub unsafe fn reprotect(a: *mut (), mode: ProtectionMode) {
}
pub unsafe fn unmap(a: *mut ()) -> Option<PAddr> {
	todo!("unmap")
}

#[derive(Debug)]
pub struct AddressSpace(u32);
impl AddressSpace
{
	pub fn pid0() -> AddressSpace {
		extern "C" {
			static kernel_table0: [u32; 4096];
		}
		AddressSpace( &kernel_table0 as *const _ as usize as u32 )
	}
	pub fn new(clone_start: usize, clone_end: usize) -> Result<AddressSpace,::memory::virt::MapError> {
		todo!("AddressSpace::new({:#x} -- {:#x})", clone_start, clone_end);
	}

	pub fn get_ttbr0(&self) -> u32 { self.0 }
}

