// "Tifflin" Kernel
// - By John Hodge (Mutabah/thePowersGang)
//
// Core/arch/riscv64/memory.rs
//! RISC-V memory management

pub const PAGE_SIZE: usize = 0x1000;
pub type PAddr = u64;
pub type VAddr = usize;

pub mod addresses {
	pub const USER_END:       usize = 0x00008000_00000000;
	
	/// Start of the kernel heap
	pub const HEAP_START:     usize = 0xFFFF8000_00000000;
	/// End of the kernel heap
	pub const HEAP_END:       usize = 0xFFFF9000_00000000;
	
	/// Start of the kernel module load area
	pub const MODULES_BASE:   usize = HEAP_END;
	/// End of the kernel module load area
	pub const MODULES_END:    usize = 0xFFFFA000_00000000;
	
	/// Start of the stacks region
	pub const STACKS_BASE:    usize = MODULES_END;
	/// End of the stacks region
	pub const STACKS_END:     usize = 0xFFFFB000_00000000;
	
	/// Start of the hardware mapping region
	pub const HARDWARE_BASE:  usize = STACKS_END;
	/// End of the hardware mapping region
	pub const HARDWARE_END:   usize = 0xFFFF_C000_00000000;
//
//	// Physical memory reference counting base:
//	//  - D-C = 1<<(32+12) = (1 << 44)
//	//  - / 4 = (1 << 42) frames, = 4 trillion = 16PB RAM
//	pub const PMEMREF_BASE:   usize = HARDWARE_END;
//	pub const PMEMREF_END:    usize = 0xFFFF_D000_00000000;
//	const MAX_FRAME_IDX: usize = (PMEMREF_END - PMEMREF_BASE) / 4;	// 32-bit integer each
//	pub const PMEMBM_BASE:	  usize = PMEMREF_END;
//	pub const PMEMBM_END:     usize = PMEMBM_BASE + MAX_FRAME_IDX / 8;	// 8 bits per byte in bitmap
//		
	pub const BUMP_START:	usize = 0xFFFF_E000_00000000;
	pub const BUMP_END  :	usize = 0xFFFF_F000_00000000;
	pub const STACK_SIZE: usize = 0x4000;
	
//	#[doc(hidden)]
//	pub const IDENT_START:    usize = 0xFFFFFFFF_80000000;	// PML4[511] (plus some)
//	#[doc(hidden)]
//	pub const IDENT_END:      usize = IDENT_START + 0x400000;	// 4MiB
//	
//	/// 
//	pub const TEMP_BASE: usize = IDENT_END;
//	pub const TEMP_END:  usize = 0xFFFFFFFF_FFFF0000;	// Leave the last 16 pages free
	pub fn is_global(addr: usize) -> bool
	{
		if addr < USER_END {
			false
		}
		//else if addr < HEAP_START {
		//	panic!("Calling is_global on non-canonical address {:#x}", addr)
		//}
		// TODO: Kernel-side per-process data
		else {
			true
		}
	}
}

pub mod virt {
	pub struct AddressSpace;
	impl AddressSpace
	{
		pub fn pid0() -> AddressSpace {
			AddressSpace
		}
		pub fn new(_cstart: usize, _cend: usize) -> Result<AddressSpace,()> {
			return Ok(AddressSpace);
		}
	}

	pub fn post_init() {
	}

	pub unsafe fn temp_map<T>(_pa: super::PAddr)  -> *mut T {
		::core::ptr::null_mut()
	}
	pub unsafe fn temp_unmap<T>(_a: *mut T) {
	}

	enum PageLevel {
		Leaf4K,
		Leaf2M,
		Leaf1G,
		Leaf512G,
	}
	struct PageWalkRes {
		pte: u64,
		level: PageLevel,
	}
	impl PageWalkRes {
		fn is_valid(&self) -> bool { self.pte & 1 != 0 }
		fn ppn(&self) -> u64 { self.pte >> 10 }
		fn phys_base(&self) -> u64 { self.ppn() << 12 }
		fn phys_mask(&self) -> u64 {
			match self.level
			{
			PageLevel::Leaf4K => (1 << 12) - 1,
			PageLevel::Leaf2M => (1 << 21) - 1,
			PageLevel::Leaf1G => (1 << 30) - 1,
			PageLevel::Leaf512G => (1 << 39) - 1,
			}
		}
	}
	fn page_walk(addr: usize) -> PageWalkRes {
		todo!("page_walk({:#x})", addr);
	}
	pub fn get_phys<T>(p: *const T) -> ::memory::PAddr {
		let r = page_walk(p as usize);
		if r.is_valid() {
			r.phys_base() + (p as usize as u64 & r.phys_mask())
		}
		else {
			0
		}
	}
	pub fn is_reserved<T>(p: *const T) -> bool {
		page_walk(p as usize).is_valid()
	}
	pub fn get_info<T>(_p: *const T) -> Option<(::memory::PAddr,::memory::virt::ProtectionMode)> {
		todo!("get_info");
	}

	const FIXED_START: usize = 0xFFFFFFFF_80000000;
	const FIXED_END  : usize = 0xFFFFFFFF_C0000000;
	pub fn is_fixed_alloc(addr: *const (), size: usize) -> bool {
		// If this is within 0x...F_80000000 to 0x...F_BFFFFFFF
		FIXED_START <= addr as usize && addr as usize + size <= FIXED_END
	}
	pub unsafe fn fixed_alloc(_p: ::memory::PAddr, _count: usize) -> Option<*mut ()> {
		// Check if it's within 1GB of the start of RAM
		None
	}

	pub fn can_map_without_alloc(_a: *mut ()) -> bool {
		false
	}

	pub unsafe fn map(_a: *mut (), _p: ::memory::PAddr, _mode: ::memory::virt::ProtectionMode) {
		todo!("map");
	}
	pub unsafe fn reprotect(_a: *mut (), _mode: ::memory::virt::ProtectionMode) {
		todo!("reprotect");
	}
	pub unsafe fn unmap(_a: *mut ()) -> Option<::memory::PAddr> {
		todo!("unwrap");
	}
}
pub mod phys {
	pub fn ref_frame(_frame_idx: u64) {
	}
	pub fn deref_frame(_frame_idx: u64) -> u32 {
		1
	}
	pub fn get_multiref_count(_frame_idx: u64) -> u32 {
		0
	}

	pub fn mark_free(_frame_idx: u64) -> bool {
		false
	}
	pub fn mark_used(_frame_idx: u64) {
	}
}

