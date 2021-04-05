// "Tifflin" Kernel
// - By John Hodge (Mutabah/thePowersGang)
//
// Core/arch/riscv64/memory.rs
//! RISC-V memory management

pub const PAGE_SIZE: usize = 0x1000;
pub type PAddr = u64;
pub type VAddr = usize;

pub mod addresses {
	pub const USER_END:       usize = 0x00000040_00000000;
	
	/// Start of the kernel heap
	pub const HEAP_START:     usize = 0xFFFFFFC0_00000000;
	/// End of the kernel heap
	pub const HEAP_END:       usize = 0xFFFFFFD0_00000000;
	
	/// Start of the kernel module load area
	pub const MODULES_BASE:   usize = HEAP_END;
	/// End of the kernel module load area
	pub const MODULES_END:    usize = 0xFFFFFFD8_00000000;
	
	/// Start of the stacks region
	pub const STACKS_BASE:    usize = MODULES_END;
	/// End of the stacks region
	pub const STACKS_END:     usize = 0xFFFFFFE0_00000000;
	
	/// Start of the hardware mapping region
	pub const HARDWARE_BASE:  usize = STACKS_END;
	/// End of the hardware mapping region
	pub const HARDWARE_END:   usize = 0xFFFFFFF0_00000000;
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
	pub const BUMP_START:	usize = 0xFFFFFFF0_00000000;
	pub const BUMP_END  :	usize = 0xFFFFFFF8_00000000;
	pub const STACK_SIZE: usize = 0x4000;
	
	#[doc(hidden)]
	pub const IDENT_START:    usize = 0xFFFFFFFF_80000000;
	#[doc(hidden)]
	pub const IDENT_END:      usize = IDENT_START + (1 << 30);	// 1GiB

	pub const TEMP_BASE: usize = 0xFFFFFFFF_FFC00000;
	pub const TEMP_END:  usize = 0xFFFFFFFF_FFE00000;	// Leave the last 2MiB free


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

pub mod virt
{
	use ::core::sync::atomic::Ordering;
	use ::core::sync::atomic::AtomicU64;
	use crate::memory::virt::ProtectionMode;

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

	#[derive(Copy,Clone,PartialEq,Debug)]
	enum PageLevel {
		Leaf4K,
		Leaf2M,
		Leaf1G,
		Leaf512G,
	}
	impl PageLevel
	{
		fn iter() -> impl Iterator<Item=Self> {
			let mut it = [PageLevel::Leaf512G, PageLevel::Leaf1G, PageLevel::Leaf2M, PageLevel::Leaf4K,].iter().copied();
			if true /*39-bit paging*/ {
				it.next();
			}
			it
		}
		fn ofs(&self) -> u32 {
			match self
			{
			PageLevel::Leaf4K => 12,
			PageLevel::Leaf2M => 21,
			PageLevel::Leaf1G => 30,
			PageLevel::Leaf512G => 39,
			}
		}
		fn mask(&self) -> u64 {
			(1 << self.ofs()) - 1
		}
	}

	fn make_pte(pa: super::PAddr, level: PageLevel, mode: ProtectionMode) -> u64
	{
		let pbits = match mode
			{
			ProtectionMode::Unmapped => return 0,
			ProtectionMode::KernelRO => 1,
			ProtectionMode::KernelRW => 3,
			ProtectionMode::KernelRX => 5,
			ProtectionMode::UserRO => 8|1,	// User
			ProtectionMode::UserRW => 8|3,
			ProtectionMode::UserRX => 8|5,
			ProtectionMode::UserCOW => (8|1) | (1 << 8-1),
			ProtectionMode::UserRWX => 8|7,
			};
		assert!(pa & level.mask() == 0, "Unaligned address {:#x} for {:?}", pa, level);
		1 | (pbits << 1) | (3 << 6) | (pa as u64) >> 2
	}
	fn invalidate_cache(va: super::VAddr)
	{
		// SAFE: This can only cause performance issues
		unsafe { asm!("SFENCE.VMA {}", in(reg) va); }
	}
	const N_PAGETABLE_ENTS: usize = 512;
	type PageTable = [AtomicU64; N_PAGETABLE_ENTS];

	// ---
	// Temporary mappings
	// ---
	extern "C" {
		static boot_pt_lvl1_temp: PageTable;
	}
	static S_TEMP_MAPPING_SEM: crate::sync::Semaphore = crate::sync::Semaphore::new(N_PAGETABLE_ENTS as isize, N_PAGETABLE_ENTS as isize);
	pub unsafe fn temp_map<T>(pa: super::PAddr)  -> *mut T
	{
		S_TEMP_MAPPING_SEM.acquire();
		let pte = make_pte(pa, PageLevel::Leaf4K, ProtectionMode::KernelRW);
		for (i,slot_ent) in Iterator::enumerate( boot_pt_lvl1_temp.iter() )
		{
			match slot_ent.compare_exchange(0, pte, Ordering::SeqCst, Ordering::SeqCst)
			{
			Ok(_) => {
				let addr = super::addresses::TEMP_BASE + super::PAGE_SIZE * i;
				invalidate_cache(addr);
				//log_debug!("temp_map({:#x}) {:#x} = {:#x}", pa, addr, pte);
				return addr as *mut T;
				},
			Err(_) => {},
			}
		}
		panic!("BUG: Semaphore aquire worked, but no free slots in temporary mappings");
	}
	pub unsafe fn temp_unmap<T>(a: *mut T)
	{
		assert!(a as usize >= super::addresses::TEMP_BASE);
		let slot = (a as usize - super::addresses::TEMP_BASE) / super::PAGE_SIZE;
		assert!(slot < boot_pt_lvl1_temp.len());
		let prev = boot_pt_lvl1_temp[slot].swap(0, Ordering::SeqCst);
		assert!(prev != 0);
		S_TEMP_MAPPING_SEM.release();
	}
	/// Perform an operation with a temporary mapping
	/// UNSAFE: This allows access to artbitary memory. Users must ensure that there is no `&mut` to that memory
	pub unsafe fn with_temp_map<T: crate::lib::POD, F, R>(pa: super::PAddr, cb: F) -> R
	where
		F: FnOnce(&T) -> R
	{
		assert!(::core::mem::size_of::<T>() <= super::PAGE_SIZE);
		let p: *mut T = temp_map(pa);
		let rv = cb(&*p);
		temp_unmap(p);
		rv
	}

	// ---
	// Mapping lookups
	// ---
	struct PageWalkRes {
		pte: u64,
		level: PageLevel,
	}
	impl PageWalkRes {
		fn is_valid(&self) -> bool { self.pte & 1 != 0 }
		fn ppn(&self) -> u64 { (self.pte & ((1 << 54)-1) ) >> 10 }
		fn phys_base(&self) -> u64 { self.ppn() << 12 }
		fn phys_mask(&self) -> u64 { self.level.mask() }
		fn permissions(&self) -> u64 { (self.pte >> 1) & 0xF }
	}
	fn page_walk(addr: usize, max_level: PageLevel) -> Option<PageWalkRes> {
		let max_addr_size = PageLevel::iter().next().unwrap().ofs() + 9;
		let top_bits = addr >> (max_addr_size - 1);
		if top_bits & (top_bits + 1) != 0 {
			log_error!("page_walk({:#x}): Non-canonical - addr[{}:]={:#x}", addr, max_addr_size-1, top_bits);
			return None;
		}

		// SAFE: asm reading a register
		let mut table_base = unsafe {
			let v: u64;
			asm!("csrr {}, satp", out(reg) v, options(nomem, pure));
			let ppn = v & ((1 << 60)-1);
			ppn << 12
			};
		for lvl in PageLevel::iter()
		{
			let vpn = (addr >> lvl.ofs()) & (512-1);
			// SAFE: Address should be valid and non-conflicte, accessing atomically
			let pte = unsafe { with_temp_map(table_base, |ptr: &PageTable| {
				ptr[vpn].load(Ordering::Relaxed)
				}) };
			let rv = PageWalkRes { pte, level: lvl };
			// Unmapped (V=0)
			// Leaf node (has permissions bits set)
			// Maximum level reached
			if !rv.is_valid() || rv.permissions() != 0 || lvl == max_level {
				return Some(rv);
			}
			table_base = rv.phys_base();
		}
		panic!("page_walk({:#x}): {:#x} - Unexpected nested", addr, table_base);
	}
	#[inline]
	pub fn get_phys<T>(p: *const T) -> ::memory::PAddr {
		match page_walk(p as usize, PageLevel::Leaf4K)
		{
		Some(r) if r.is_valid() => r.phys_base() + (p as usize as u64 & r.phys_mask()),
		_ => 0,
		}
	}
	#[inline]
	pub fn is_reserved<T>(p: *const T) -> bool {
		page_walk(p as usize, PageLevel::Leaf4K).map(|v| v.is_valid()).unwrap_or(false)
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

	pub fn can_map_without_alloc(a: *mut ()) -> bool {
		// Do a page walk to a maximum level
		page_walk(a as usize, PageLevel::Leaf2M).map(|v| v.is_valid()).unwrap_or(false)
	}

	//enum PageWalkError {
	//	NonCanonical,
	//	UnexpectedLargePage,
	//	AllocationError,
	//}
	fn with_pte_inner(addr: usize, target_level: PageLevel, allocate: bool, cb: &mut dyn FnMut(&AtomicU64))
	{
		// Do a page walk to a maximum level, allocating as we go
		let max_addr_size = PageLevel::iter().next().unwrap().ofs() + 9;
		let top_bits = addr >> (max_addr_size - 1);
		if top_bits & (top_bits + 1) != 0 {
			log_error!("with_pte_inner({:#x}): Non-canonical - addr[{}:]={:#x}", addr, max_addr_size-1, top_bits);
			return ;
		}

		// SAFE: asm reading a register
		let mut table_base = unsafe {
			let v: u64;
			asm!("csrr {}, satp", out(reg) v, options(nomem, pure));
			let ppn = v & ((1 << 60)-1);
			ppn << 12
			};
		for lvl in PageLevel::iter()
		{
			if lvl == target_level {
				break;
			}
			let vpn = (addr >> lvl.ofs()) & (512-1);
			// SAFE: Address should be valid and non-conflicted, accessing atomically
			let pte = unsafe { with_temp_map(table_base, |ptr: &PageTable| {
				let rv = ptr[vpn].load(Ordering::Relaxed);
				if rv & 1 == 0 && allocate {
					assert!(rv == 0, "Unexpected populated but not valid non-leaf PTE: {:#x}", rv);
					let p = crate::memory::phys::allocate_bare().expect("TODO: Handle allocation errors in `virt::map`");
					let new_rv = p.phys_addr() | 1;
					match ptr[vpn].compare_exchange(0, new_rv, Ordering::SeqCst, Ordering::SeqCst)
					{
					Ok(_) => { new_rv },	// Cool
					Err(raced_rv) => {
						// SAFE: Frame is unreferenced
						crate::memory::phys::deref_frame(p.phys_addr());
						raced_rv
						},
					}
				}
				else {
					rv
				}
				}) };
			let rv = PageWalkRes { pte, level: lvl };
			// Unmapped (V=0) - should be impossible
			if !rv.is_valid() {
				assert!(!allocate);
				return ;
			}
			// Leaf node (has permissions bits set)
			if rv.permissions() != 0 {
				panic!("TODO: Handle large-page collision in `virt::with_pte_inner`");
			}

			table_base = rv.phys_base();
		}
		let vpn = (addr >> target_level.ofs()) & (512-1);
		// SAFE: Valid address, atomic operations used
		unsafe {
			with_temp_map(table_base, |ptr: &PageTable| {
				(*cb)(&ptr[vpn])
				});
		}
	}
	fn with_pte<F,R>(addr: usize, level: PageLevel, allocate: bool, cb: F) -> Option<R>
	where
		F: FnOnce(&AtomicU64)->R
	{
		let mut cb = Some(cb);
		let mut rv = None;
		with_pte_inner(addr, level, allocate, &mut |pte| rv = Some( (cb.take().unwrap())(pte) ));
		rv
	}

	pub unsafe fn map(a: *mut (), p: ::memory::PAddr, mode: ::memory::virt::ProtectionMode) {
		let new_pte = make_pte(p, PageLevel::Leaf4K, mode);
		with_pte(a as usize, PageLevel::Leaf4K, /*allocate*/true, |pte| {
			match pte.compare_exchange(0, new_pte, Ordering::SeqCst, Ordering::SeqCst)
			{
			Ok(_) => { },	// Cool
			Err(existing) => {
				panic!("TODO: Handle mapping collision in `virt::map`");
				}
			}
			});
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

