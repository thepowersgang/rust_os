//
//
//

use memory::virt::ProtectionMode;
use arch::memory::PAddr;

const KERNEL_BASE_TABLE: usize = 0xFFFF8000;

pub fn is_fixed_alloc<T>(_addr: *const T, _size: usize) -> bool {
	//const BASE : usize = super::addresses::KERNEL_BASE;
	//const ONEMEG: usize =1024*1024
	//const LIMIT: usize = super::addresses::KERNEL_BASE + 4*ONEMEG;
	false
}
// UNSAFE: Can cause aliasing
pub unsafe fn fixed_alloc(_p: PAddr, _count: usize) -> Option<*mut ()> {
	None
}

#[derive(Copy,Clone)]
enum PageEntryRegion {
	NonGlobal,
	Global,
}
impl PageEntryRegion {
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
		mapping: TempHandle,
		idx: usize,
		ofs: usize
		},
}
impl PageEntry
{
	fn alloc(addr: *const (), level: usize) -> Result<PageEntry, ()> {
		todo!("PageEntry::alloc({:p}, level={})", addr, level);
	}
	/// Obtain a page entry for the specified address
	fn get(addr: *const ()) -> PageEntry {
		use super::addresses::KERNEL_BASE;
		let (rgn, p_idx) = if (addr as usize) < KERNEL_BASE {
				(PageEntryRegion::NonGlobal, (addr as usize - KERNEL_BASE) >> 12)
			}
			else {
				(PageEntryRegion::Global, (addr as usize) >> 12)
			};

		// SAFE: Aliasing in this case is benign
		let sect_ent = unsafe { *rgn.get_section_ent(p_idx >> 8) };
		if sect_ent & 0b11 == 0b01 {
			PageEntry::Page {
				// SAFE: ... won't be mutated (yet), but need to ensure safety TODO
				mapping: unsafe { TempHandle::new( sect_ent & !0xFFF ) },
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
			&PageEntry::Page { ref mapping, idx, .. } => (mapping[idx & 0x3FF] & 3 != 0),
			}
		}
	}

	fn phys_addr(&self) -> ::arch::memory::PAddr {
		// SAFE: Aliasing is benign, and page table should be mapped (see above TODO)
		unsafe {
			match self
			{
			&PageEntry::Section { rgn, idx, ofs } => (*rgn.get_section_ent(idx) & !0xFFF) + ofs as u32,
			&PageEntry::Page { ref mapping, idx ,ofs } => (mapping[idx & 0x3FF] & !0xFFF) + ofs as u32,
			}
		}
	}
}

#[repr(C)]
struct AtomicU32(::core::cell::UnsafeCell<u32>);
impl AtomicU32 {
	pub fn cxchg(&self, val: u32, new: u32) -> u32 {
		// SAFE: Atomic
		unsafe { ::core::intrinsics::atomic_cxchg_relaxed(self.0.get(), val, new) }
	}
	pub fn store(&self, val: u32) {
		// SAFE: Atomic
		unsafe { ::core::intrinsics::atomic_store_relaxed(self.0.get(), val) }
	}
	pub fn load(&self) -> u32 {
		// SAFE: Atomic
		unsafe { ::core::intrinsics::atomic_load_relaxed(self.0.get()) }
	}
}
extern "C" {
	static kernel_table0: [AtomicU32; 0x800*2];
	static kernel_exception_map: [AtomicU32; 1024];
}

fn get_table_addr<T>(a: *const T, alloc: bool) -> Option< (::arch::memory::PAddr, usize) > {
	let addr = a as usize;
	let page = addr >> 12;
	let (ttbr_ofs, tab_idx) = (page >> 8, page & 0xFF);
	let ent_r = if ttbr_ofs < 0x800 {
			todo!("get_table_addr - User");
		}
		else {
			// Kernel
			&kernel_table0[ ttbr_ofs ]
		};
	
	//let ent_v = ent_r.load();
	let ent_v = ent_r.load();
	match ent_v & 0xFFF
	{
	0 => if alloc {
			let frame = ::memory::phys::allocate_bare().expect("TODO get_table_addr");
			let ent_v = ent_r.cxchg(0, frame + 0x1);
			if ent_v != 0 {
				::memory::phys::deref_frame(frame);
				Some( (ent_v & !0xFFF, tab_idx) )
			}
			else {
				Some( (frame & !0xFFF, tab_idx) )
			}
		}
		else {
			None
		},
	1 => Some( (ent_v & !0xFFF, tab_idx) ),
	v @ _ => todo!("get_table_addr - Other flags bits {:#x}", v),
	}
}
//static S_TEMP_MAP_SEMAPHORE: Semaphore = Semaphore::new();
const KERNEL_TEMP_BASE : usize = 0xFFC00000;

struct TempHandle(*mut [u32; 1024]);
impl TempHandle
{
	/// UNSAFE: User must ensure that address is valid, and that no aliasing occurs
	unsafe fn new(phys: ::arch::memory::PAddr) -> TempHandle {
		let val = (phys as u32) + 0x13;	

		//S_TEMP_MAP_SEMAPHORE.take();
		// #1023 is reserved for -1 mapping
		for i in 0 .. 1023 {
			if kernel_exception_map[i].cxchg(0, val) == 0 {
				return TempHandle( (KERNEL_TEMP_BASE + i * 0x1000) as *mut _ );
			}
		}
		panic!("No free temp mappings");
	}
}
impl ::core::ops::Deref for TempHandle {
	type Target = [u32];
	fn deref(&self) -> &[u32] {
		// SAFE: We should have unique access
		unsafe { &*self.0 }
	}
}
impl ::core::ops::DerefMut for TempHandle {
	fn deref_mut(&mut self) -> &mut [u32] {
		// SAFE: We should have unique access
		unsafe { &mut *self.0 }
	}
}
impl ::core::ops::Drop for TempHandle {
	fn drop(&mut self) {
		let i = (self.0 as usize - KERNEL_TEMP_BASE) / 0x1000;
		kernel_exception_map[i].store(0);
		//S_TEMP_MAP_SEMAPHORE.add();
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
	// 1. Map the relevant table in the temp area
	let (tab_phys, idx) = get_table_addr(a, true).unwrap();
	// TODO: Ensure nothing else is manipulating this segment of AS
	let mut mh = TempHandle::new( tab_phys );
	// 2. Insert
	let mode_flags = match mode
		{
		ProtectionMode::Unmapped => panic!("Invalid pass of Unmapped to map"),
		ProtectionMode::KernelRO => 0x212,
		ProtectionMode::KernelRW => 0x012,
		ProtectionMode::KernelRX => 0x053,
		ProtectionMode::UserRO => 0x232,
		ProtectionMode::UserRW => 0x032,
		ProtectionMode::UserRX => 0x233,
		ProtectionMode::UserRWX => 0x033,
		ProtectionMode::UserCOW => 0x223,	// 1,10 is a deprecated encoding for RO, need to find a better encoding
		};
	mh[idx] = p + mode_flags;
}
pub unsafe fn reprotect(a: *mut (), mode: ProtectionMode) {
	todo!("reprotect({:p}, {:?}", a, mode)
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
			static kernel_table0_ofs: ::Void;
			static kernel_data_start: u32;
		}
		let tab0_addr = kernel_data_start + (&kernel_table0_ofs as *const _ as usize as u32);
		AddressSpace( tab0_addr )
	}
	pub fn new(clone_start: usize, clone_end: usize) -> Result<AddressSpace,::memory::virt::MapError> {
		todo!("AddressSpace::new({:#x} -- {:#x})", clone_start, clone_end);
	}

	pub fn get_ttbr0(&self) -> u32 { self.0 }
}

