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
	// NOTE: Hard-coded in start.S too
	pub const STACKS_BASE:    usize = MODULES_END;
	/// End of the stacks region
	pub const STACKS_END:     usize = 0xFFFFFFE0_00000000;
	pub const STACK_SIZE: usize = 0x4000;
	pub(in crate::arch) const STACK0_BASE: usize = STACKS_BASE + STACK_SIZE;
	
	/// Start of the hardware mapping region
	pub const HARDWARE_BASE:  usize = STACKS_END;
	/// End of the hardware mapping region
	pub const HARDWARE_END:   usize = 0xFFFFFFF0_00000000;
		
	pub const BUMP_START:	usize = 0xFFFFFFF0_00000000;
	pub const BUMP_END  :	usize = 0xFFFFFFF8_00000000;

	// Physical memory reference counting base:
	//  - F-8 = 7<<32 = 28G
	//  - / 4 = (7 << 20) frames, = 7 billion = 28TB RAM
	pub const PMEMREF_BASE:   usize = BUMP_END;
	pub const PMEMREF_END:    usize = 0xFFFFFFFF_00000000;
	const MAX_FRAME_IDX: usize = (PMEMREF_END - PMEMREF_BASE) / 4;	// 32-bit integer each
	pub const PMEMBM_BASE:	  usize = PMEMREF_END;
	pub const PMEMBM_END:     usize = PMEMBM_BASE + MAX_FRAME_IDX / 8;	// 8 bits per byte in bitmap
	static_assert!(PMEMBM_END <= IDENT_START);
	
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

	pub struct AddressSpace(u64);
	impl_fmt! {
		Debug(self,f) for AddressSpace { write!(f, "AddressSpace({:#x})", self.0) }
	}
	impl AddressSpace
	{
		pub fn pid0() -> AddressSpace {
			// SAFE: Just getting the address of a static
			AddressSpace( crate::memory::virt::get_phys(unsafe { extern "C" { static boot_pt_lvl3_0: crate::Extern; } &boot_pt_lvl3_0 }) )
		}
		pub fn new(clone_start: usize, clone_end: usize) -> Result<AddressSpace,crate::memory::virt::MapError>
		{
			use crate::memory::virt::MapError;

			struct NewTable(crate::arch::memory::virt::TempHandle<u64>, /** Level for the contained items */PageLevel);
			impl NewTable {
				fn new(level: PageLevel) -> Result<NewTable,MapError> {
					match crate::memory::phys::allocate_bare()
					{
					Err(crate::memory::phys::Error) => Err( MapError::OutOfMemory ),
					Ok(temp_handle) => Ok( NewTable( temp_handle.into(), level ) ),
					}
				}
				fn into_frame(self) -> super::PAddr {
					let rv = self.0.phys_addr();
					::core::mem::forget(self);
					rv
				}
			}
			impl ::core::ops::Drop for NewTable {
				fn drop(&mut self) {
					// TODO: This method needs to recursively free paging structures held by it.
					todo!("NewTable::drop");
				}
			}
			impl ::core::ops::Deref for NewTable { type Target = [u64]; fn deref(&self) -> &[u64] { &self.0 } }
			impl ::core::ops::DerefMut for NewTable { fn deref_mut(&mut self) -> &mut [u64] { &mut self.0 } }

			/// Clone an entire table, returns the new PTE
			fn opt_clone_table(table_level: PageLevel, base_addr: usize, clone_start: usize, clone_end: usize, prev_table_pte: u64) -> Result<u64, MapError>
			{
				// Zero = unmapped, return without logging
				if prev_table_pte == 0
				{
					return Ok(0);
				}
				log_trace!("opt_clone_table({:?} @ {:#x}, {:#x}--{:#x} from {:#x})", table_level, base_addr, clone_start, clone_end, prev_table_pte);
				// Check range
				if clone_end <= base_addr
				{
					Ok(0)
				}
				else if clone_start >= base_addr + (1 << table_level.ofs())
				{
					Ok(0)
				}
				// If the bottom 4 bits are 1, it's a sub-table
				// - Recurse into it
				else
				{
					let ent = Pte(prev_table_pte);
					if prev_table_pte & 0xF == 1
					{
						let item_level = match table_level.down()
							{
							Some(v) => v,
							None => panic!("opt_clone_table({:?} @ {:#x} from {:#x}) - Reached 4K but still recursing", table_level, base_addr, prev_table_pte),
							};
						let mut table = NewTable::new(item_level)?;
						// SAFE: Valid and unaliased physical memory
						unsafe {
							with_temp_map(ent.phys_base(), |ptr: &PageTable|->Result<(),MapError> {
								for i in 0 .. 512
								{
									table[i] = opt_clone_table(item_level, base_addr + (i << item_level.ofs()), clone_start, clone_end, ptr[i].load(Ordering::Relaxed))?;
								}
								Ok( () )
								})?;
						}
						Ok( (table.into_frame() >> 12 << 10) + 1 )
					}
					// Otherwise, it's data
					else
					{
						let perms = ent.get_perms();
						let frame = match perms
							{
							ProtectionMode::UserRX | ProtectionMode::UserCOW => {
								let addr = ent.phys_base();
								crate::memory::phys::ref_frame( addr );
								addr
								},
							ProtectionMode::UserRWX | ProtectionMode::UserRW => {
								// SAFE: We've just determined that this page is mapped in, so we won't crash. Any race is the user's fault (and shouldn't impact the kernel)
								let src = unsafe { ::core::slice::from_raw_parts(base_addr as *const u8, super::PAGE_SIZE) };
								let mut newpg = crate::memory::virt::alloc_free()?;
								for (d,s) in Iterator::zip( newpg.iter_mut(), src.iter() ) {
									*d = *s;
								}
								newpg.into_frame().into_addr()
								},
							v @ _ => todo!("opt_clone_page - Mode {:?}", v),
							};
						Ok( make_pte(frame, table_level, perms) )
					}
				}
			}

			// TODO: This could suffer from the dining philosophers problem
			// Could run out of temporary slots if multiple processes try to fork at once

			// Create a new root table
			let root_granuality = PageLevel::Leaf1G;
			let mut table = NewTable::new(root_granuality)?;
			// SAFE: Valid physical address, never gets `&mut`
			unsafe { with_temp_map(get_root_table_phys(), |ptr: &PageTable|->Result<(),MapError> {
				// Recursively copy the user clone region
				for i in 0 .. 256
				{
					table[i] = opt_clone_table(root_granuality, i << root_granuality.ofs(), clone_start, clone_end, ptr[i].load(Ordering::Relaxed))?;
				}
				// Shallow copy all kernel top-level entries
				for i in 256 .. 512
				{
					// SAFE: Atomic operations
					table[i] = ptr[i].load(Ordering::Relaxed);
				}
				Ok( () )
				})? };

			Ok(AddressSpace(table.into_frame()))
		}
		pub(in crate::arch::riscv64) fn as_phys(&self) -> u64 {
			self.0
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
		fn down(&self) -> Option<PageLevel> {
			match self
			{
			PageLevel::Leaf4K => None,
			PageLevel::Leaf2M => Some(PageLevel::Leaf4K),
			PageLevel::Leaf1G => Some(PageLevel::Leaf2M),
			PageLevel::Leaf512G => Some(PageLevel::Leaf1G),
			}
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
		unsafe { ::core::arch::asm!("SFENCE.VMA {}", in(reg) va); }
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
		//log_debug!("temp_unmap({:p})", a);
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
		//log_trace!("with_temp_map::<T={},F={}>({:#x})", type_name!(T), type_name!(F), pa);
		let p: *mut T = temp_map(pa);
		let rv = cb(&*p);
		temp_unmap(p);
		rv
	}

	// ---
	// Mapping lookups
	// ---
	struct Pte(u64);
	impl Pte {
		fn is_valid(&self) -> bool { self.0 & 1 != 0 }
		fn ppn(&self) -> u64 { (self.0 & ((1 << 54)-1) ) >> 10 }
		fn phys_base(&self) -> u64 { self.ppn() << 12 }
		fn permissions(&self) -> u64 { (self.0 >> 1) & 0xF }

		fn is_read(&self) -> bool { self.permissions() & 1 != 0 }
		fn is_write(&self) -> bool { self.permissions() & 2 != 0 }
		fn is_exec(&self) -> bool { self.permissions() & 4 != 0 }
		fn is_user(&self) -> bool { self.permissions() & 8 != 0 }
		fn is_cow(&self) -> bool { (self.0 >> 8) & 1 != 0 }

		fn get_perms(&self) -> ProtectionMode {
			match self.permissions()
			{
			0x1 => ProtectionMode::KernelRO,
			0x3 => ProtectionMode::KernelRW,
			0x5 => ProtectionMode::KernelRX,
			//0x7 => ProtectionMode::KernelRWX,
			0b1001 if self.is_cow() => ProtectionMode::UserCOW,
			0b1001 => ProtectionMode::UserRO,
			0b1011 => ProtectionMode::UserRW,
			0b1101 => ProtectionMode::UserRX,
			0b1111 => ProtectionMode::UserRWX,
			v => todo!("get_perms(): {:#x}", v),
			}
		}
	}
	struct PageWalkRes {
		pte: Pte,
		level: PageLevel,
	}
	impl_fmt!{
		Debug(self, f) for PageWalkRes {
			write!(f, "PageWalkRes({:?} {:#x} )", self.level, self.pte.0)
		}
	}
	fn get_root_table_phys() -> u64 {
		// SAFE: asm reading a register
		unsafe {
			let v: u64;
			::core::arch::asm!("csrr {}, satp", out(reg) v, options(nomem, pure));
			let ppn = v & ((1 << 60)-1);
			ppn << 12
		}
	}
	fn page_walk(addr: usize, max_level: PageLevel) -> Option<PageWalkRes> {
		let max_addr_size = PageLevel::iter().next().unwrap().ofs() + 9;
		let top_bits = addr >> (max_addr_size - 1);
		if top_bits != [0,!0 >> (max_addr_size-1)][top_bits & 1] {
			log_error!("page_walk({:#x}): Non-canonical - addr[{}:]={:#x}", addr, max_addr_size-1, top_bits);
			return None;
		}

		let mut table_base = get_root_table_phys();
		for lvl in PageLevel::iter()
		{
			let vpn = (addr >> lvl.ofs()) & (512-1);
			// SAFE: Address should be valid and non-conflicte, accessing atomically
			let pte = unsafe { with_temp_map(table_base, |ptr: &PageTable| {
				ptr[vpn].load(Ordering::Relaxed)
				}) };
			let rv = PageWalkRes { pte: Pte(pte), level: lvl };
			// Unmapped (V=0)
			// Leaf node (has permissions bits set)
			// Maximum level reached
			if !rv.pte.is_valid() || rv.pte.permissions() != 0 || lvl == max_level {
				return Some(rv);
			}
			table_base = rv.pte.phys_base();
		}
		panic!("page_walk({:#x}): {:#x} - Unexpected nested", addr, table_base);
	}
	#[inline]
	pub fn get_phys<T>(p: *const T) -> crate::memory::PAddr {
		match page_walk(p as usize, PageLevel::Leaf4K)
		{
		Some(r) if r.pte.is_valid() => r.pte.phys_base() + (p as usize as u64 & r.level.mask()),
		_ => 0,
		}
	}
	#[inline]
	pub fn is_reserved<T>(p: *const T) -> bool {
		// TODO: Poke the PF handler for this HART, read the memory, then check the flag
		page_walk(p as usize, PageLevel::Leaf4K).map(|v| v.pte.is_valid()).unwrap_or(false)
	}
	pub fn get_info<T>(_p: *const T) -> Option<(crate::memory::PAddr,crate::memory::virt::ProtectionMode)> {
		todo!("get_info");
	}

	const FIXED_START: usize = 0xFFFFFFFF_80000000;
	const FIXED_END  : usize = 0xFFFFFFFF_C0000000;
	pub fn is_fixed_alloc(addr: *const (), size: usize) -> bool {
		// If this is within 0x...F_80000000 to 0x...F_BFFFFFFF
		FIXED_START <= addr as usize && addr as usize + size <= FIXED_END
	}
	pub unsafe fn fixed_alloc(_p: crate::memory::PAddr, _count: usize) -> Option<*mut ()> {
		// Check if it's within 1GB of the start of RAM
		None
	}

	pub fn can_map_without_alloc(a: *mut ()) -> bool {
		// Do a page walk to a maximum level
		page_walk(a as usize, PageLevel::Leaf2M).map(|v| v.pte.is_valid()).unwrap_or(false)
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
		if top_bits != [0,!0 >> (max_addr_size-1)][top_bits & 1] {
			log_error!("with_pte_inner({:#x}): Non-canonical - addr[{}:]={:#x}", addr, max_addr_size-1, top_bits);
			return ;
		}

		// SAFE: asm reading a register
		let mut table_base = get_root_table_phys();
		for lvl in PageLevel::iter()
		{
			if lvl == target_level {
				break;
			}
			let vpn = (addr >> lvl.ofs()) & (512-1);
			//log_trace!("{:#x} {:?} table_base={:#x} vpn={}", addr, lvl, table_base, vpn);
			// SAFE: Address should be valid and non-conflicted, accessing atomically
			let pte = unsafe { with_temp_map(table_base, |ptr: &PageTable| {
				let rv = ptr[vpn].load(Ordering::Relaxed);
				if rv & 1 == 0 && allocate {
					assert!(rv == 0, "Unexpected populated but not valid non-leaf PTE: {:#x}", rv);
					let p = crate::memory::phys::allocate_bare().expect("TODO: Handle allocation errors in `virt::map`");
					let new_rv = (p.phys_addr() >> 2) | 1;
					log_debug!("{:#x} {:?} Allocate PT pte={:#x}", addr, lvl, new_rv);
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
			//log_trace!("{:#x} {:?} pte={:#x}", addr, lvl, pte);
			let rv = Pte(pte);
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

	pub unsafe fn map(a: *mut (), p: crate::memory::PAddr, mode: crate::memory::virt::ProtectionMode) {
		//log_trace!("map({:p}, {:#x}, mode={:?})", a, p, mode);
		let new_pte = make_pte(p, PageLevel::Leaf4K, mode);
		with_pte(a as usize, PageLevel::Leaf4K, /*allocate*/true, |pte| {
			match pte.compare_exchange(0, new_pte, Ordering::SeqCst, Ordering::SeqCst)
			{
			Ok(_) => { },	// Cool
			Err(existing) => {
				panic!("TODO: Handle mapping collision in `virt::map` - {:#x}", existing);
				}
			}
			});
	}
	pub unsafe fn reprotect(a: *mut (), mode: crate::memory::virt::ProtectionMode) {
		with_pte(a as usize, PageLevel::Leaf4K, /*allocate*/true, |pte| {
			let old_pte = pte.load(Ordering::SeqCst);
			let new_pte = make_pte(Pte(old_pte).phys_base(), PageLevel::Leaf4K, mode);
			match pte.compare_exchange(old_pte, new_pte,  Ordering::SeqCst, Ordering::SeqCst)
			{
			Ok(_) => { },	// Cool
			Err(existing) => {
				panic!("TODO: Handle mapping collision in `virt::reprotect` - {:#x} != {:#x}", existing, old_pte);
				}
			}
			});
	}
	pub unsafe fn unmap(a: *mut ()) -> Option<crate::memory::PAddr> {
		let mut rv = None;
		with_pte(a as usize, PageLevel::Leaf4K, /*allocate*/false, |pte| {
			let v = pte.swap(0, Ordering::SeqCst);
			if v != 0
			{
				rv = Some(Pte(v).phys_base());
			}
			});
		rv
	}


	/// Handle a page (memory access) fault
	///
	/// Returns `true` if the fault was resolved (e.g. CoW copied, or paged-out memory returned)
	pub fn page_fault(a: usize, is_write: bool) -> bool
	{
		let PageWalkRes { pte: r, level } = match page_walk(a, PageLevel::Leaf4K)
			{
			Some(v) => v,
			None => return false,
			};

		if r.is_valid()
		{
			// Check for CoW
			if r.is_cow() {
				assert!(r.is_user(), "COW mapping for non-user");
				assert!(r.is_read(), "COW mapping not readable?");
				assert!(!r.is_exec(), "COW mapping executable?");
				assert!(!r.is_write(), "COW mapping writeable?");
				assert!(is_write, "COW mapping access failure not a write");
				assert!(level == PageLevel::Leaf4K, "COW mapping not 4K?");

				// 1. Lock (relevant) address space
				// SAFE: Changes to address space are transparent
				crate::memory::virt::with_lock(a, || unsafe {
					let frame = r.phys_base();
					let pgaddr = a & !(super::PAGE_SIZE - 1);
					// 2. Get the PMM to provide us with a unique copy of that frame (can return the same addr)
					// - This borrow is valid, as the page is read-only (for now)
					let newframe = crate::memory::phys::make_unique( frame, &*(pgaddr as *const [u8; super::PAGE_SIZE]) );
					// 3. Remap to this page as UserRW (because COW is user-only atm)
					let new_pte = make_pte(newframe, PageLevel::Leaf4K, ProtectionMode::UserRW);
					with_pte(a, PageLevel::Leaf4K, /*allocate*/false, |pte| {
						match pte.compare_exchange(r.0, new_pte, Ordering::SeqCst, Ordering::SeqCst)
						{
						Ok(_) => {},
						Err(other_pte) => todo!("Contented CoW clone? - {:#x} != {:#x}", other_pte, r.0),
						}
						});
					invalidate_cache(a);
					});
				return true;
			}
			false
		}
		else
		{
			// Memory is not valid, might be paged out?
			if r.0 != 0 {
				todo!("Handle invalid but non-zero mappings: {:#x}", r.0);
			}
			false
		}
	}
}

