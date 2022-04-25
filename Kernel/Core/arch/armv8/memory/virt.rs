// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/arch/armv8/memory/virt.rs
//! Virtual memory interface
// NOTE NOTE: Page size on ARMv8 (this config) is 0x4000 (16K, 2<<14) - Keeps things interesting
use ::core::sync::atomic::{Ordering,AtomicU64};
use crate::memory::virt::ProtectionMode;
use crate::PAGE_SIZE;
use super::addresses;
use super::addresses::{USER_FRACTAL_BASE,KERNEL_FRACTAL_BASE};

const USER_SIZE: usize = 1 << 47;
const KERNEL_BASE: usize = !(USER_SIZE - 1);

pub struct AddressSpace(u64);
impl_fmt! {
	Debug(self,f) for AddressSpace { write!(f, "AddressSpace({:#x})", self.0) }
}

pub fn post_init()
{
	// No changes needed after init
}

fn prot_mode_to_attrs(prot: ProtectionMode) -> u64
{
	match prot
	{
	ProtectionMode::Unmapped => 0,
	//ProtectionMode::KernelRWX=> (0x00<<56) | (0x00<<2),
	ProtectionMode::UserRWX  => (0x00<<56) | (0x40),

	ProtectionMode::KernelRX => (0x00<<56) | (0x80),
	ProtectionMode::UserRX   => (0x00<<56) | (0xC0),
	ProtectionMode::KernelRW => (0x10<<56) | (0x00),
	ProtectionMode::UserRW   => (0x10<<56) | (0x40),
	ProtectionMode::KernelRO => (0x10<<56) | (0x80),
	ProtectionMode::UserRO   => (0x10<<56) | (0xC0),
	ProtectionMode::UserCOW  => (0x11<<56) | (0xC0),
	//_ => 0,
	}
}
fn attrs_to_prot_mode(attrs: u64) -> ProtectionMode
{
	let v = (((attrs >> 56) & 0xFF) << 8) | (attrs & 0xFC);
	match v
	{
	0x00_00 => ProtectionMode::KernelRW,	// RWX
	0x00_40 => ProtectionMode::UserRW,	// RWX
	0x00_80 => ProtectionMode::KernelRX,
	0x00_C0 => ProtectionMode::UserRX,
	0x10_00 => ProtectionMode::KernelRW,
	0x10_40 => ProtectionMode::UserRW,
	0x10_80 => ProtectionMode::KernelRO,
	0x10_C0 => ProtectionMode::UserRO,
	0x11_C0 => ProtectionMode::UserCOW,
	_ => todo!("Unknown attributes - 0x{:04x}", v),
	}
}


pub fn is_reserved<T>(addr: *const T) -> bool
{
	get_phys_raw(addr).is_some()
}
pub fn get_phys<T>(addr: *const T) -> u64
{
	get_phys_raw(addr).unwrap_or(0)
}
pub fn get_info<T>(addr: *const T) -> Option<(u64, ProtectionMode)>
{
	if let Some(paddr) = get_phys_raw(addr)
	{
		let (space,masked) = if (addr as usize) < USER_SIZE {
				(Space::User, addr as usize % USER_SIZE)
			}
			else if addr as usize >= KERNEL_BASE {
				(Space::Kernel, addr as usize % USER_SIZE)
			}
			else {
				return None;
			};
		// SAFE: Read-only
		let a = unsafe { with_entry(space, Level::Middle, masked >> (14+11), |e| {
			let v = e.load(Ordering::Relaxed);
			if v & 3 == 3 { None } else { Some(v) }
			})
			.unwrap_or_else(|| with_entry(space, Level::Bottom, masked >> 14, |e| e.load(Ordering::Relaxed)))
			};
		let prot = attrs_to_prot_mode(a & 0xFF000000_000003FC);
		Some( (paddr, prot) )
	}
	else
	{
		None
	}
}
fn get_phys_raw<T>(addr: *const T) -> Option<u64> {
	// SAFE: Queries an interface that cannot cause an exception (and won't induce memory unsafety)
	let v = unsafe {
		let ret: usize;
		::core::arch::asm!("AT S1E1R, {1}; mrs {0}, PAR_EL1", out(reg) ret, in(reg) addr, options(pure, readonly, nostack));
		ret
		};
	if v & 1 != 0 {
		None
	}
	else {
		Some( ((v & 0x0000FFFF_FFFFF000) + (addr as usize & 0xFFF)) as u64 )
	}
}

#[derive(Debug,Copy,Clone)]
enum Space
{
	User,
	Kernel,
}
#[derive(Debug,Copy,Clone)]
enum Level
{
	Root,
	Middle,
	Bottom,
}

fn get_entry_addr(space: Space, level: Level, index: usize) -> *const AtomicU64
{
	let base = match space
		{
		Space::User => USER_FRACTAL_BASE,
		Space::Kernel => KERNEL_FRACTAL_BASE,
		};
	let fidx = (base >> (14+11+11)) % 2048;
	let ofs = 8 * match level
		{
		Level::Root => {
			assert!(index < 2048);
			(fidx << 22) + (fidx << 11) + index
			},
		Level::Middle => {
			assert!(index < 2048*2048);
			(fidx << 22) + index
			},
		Level::Bottom => {
			assert!(index < 2048*2048*2048);
			index
			}
		};
	(base + ofs) as *const _
}

unsafe fn with_entry<F, R>(space: Space, level: Level, index: usize, fcn: F) -> R
where
	F: FnOnce(&AtomicU64)->R
{
	let ptr = get_entry_addr(space, level, index);
	debug_assert!(is_reserved(ptr));
	//log_trace!("with_entry({:?}, {}): ptr={:p}", level, index, ptr);
	
	// SAFE: Pointer is asserted to be valid above
	fcn( /*unsafe {*/ &*ptr /*}*/ )
}
unsafe fn with_leaf_entry<F, R>(addr: *const(), alloc: bool, fcn: F) -> Option<R>
where
	F: FnOnce(&AtomicU64)->R
{
/*
	let mut rv = None;
	with_leaf_entry_inner(addr, alloc, |e| rv = Some(fcn(e)));
	rv
}
fn with_leaf_entry_inner(addr: *const(), alloc: bool, fcn: dyn FnOnce(&AtomicU64)) -> Option<()>
{
// */
	const PAGE_MASK: usize = (USER_SIZE / PAGE_SIZE) - 1;
	let page = (addr as usize / PAGE_SIZE) & PAGE_MASK;
	let sign_bits = addr as usize >> 48;
	let kernel_bit = (addr as usize >> 47) & 1;
	assert!(sign_bits == [0,0xFFFF][kernel_bit], "Non-canonical address {:p} ({:#x})", addr, sign_bits);
	//log_trace!("page = {}:{:#x}", b"UK"[is_kernel as usize], page);
	// If the address is above the 48-bit user-kernel split
	let space = match kernel_bit != 0
		{
		false => Space::User,
		true => Space::Kernel,
		};
	// Kernel AS doesn't need a deletion lock, as it's never pruned
	// Mutation lock also not needed (but is provided in VMM)
	if kernel_bit == 0 {
		// TODO: Deletion lock for userland?
	}

	// 0x4000 / 8 = 0x800 = 11 bits

	// 1. Ensure that top-level region is valid.
	if with_entry(space, Level::Root, page >> 22, |e| {
		if e.load(Ordering::Relaxed) == 0 {
			if alloc {
				log_debug!("Allocate Level2 @ {:#x}", page >> 22);
				crate::memory::phys::allocate( get_entry_addr(space, Level::Middle, page >> 22 << 11) as *mut () );
				assert!(e.load(Ordering::Relaxed) != 0);
				// Clear NX (as it infects lower levels)
				e.fetch_and(!(0x10 << 56), Ordering::Relaxed);
			}
			else {
				return true;
			}
		}
		//log_debug!("with_leaf_entry: Root[{:#x}]={:#x}", page >> 22, e.load(Ordering::Relaxed));
		false
		})
	{
		return None;
	}
	// 2. Ensure that level2 is valid
	if with_entry(space, Level::Middle, page >> 11, |e| {
		if e.load(Ordering::Relaxed) == 0 {
			if alloc {
				log_debug!("Allocate Level3 @ {:#x}", page >> 11);
				crate::memory::phys::allocate( get_entry_addr(space, Level::Bottom, page >> 11 << 11) as *mut () );
				assert!(e.load(Ordering::Relaxed) != 0);
				// Clear NX (as it infects lower levels)
				e.fetch_and(!(0x10 << 56), Ordering::Relaxed);
			}
			else {
				return true;
			}
		}
		//log_debug!("with_leaf_entry: Middle[{:#x}]={:#x}", page >> 11, e.load(Ordering::Relaxed));
		false
		})
	{
		return None;
	}
	Some( with_entry(space, Level::Bottom, page, |e| fcn(e)) )
}

pub fn can_map_without_alloc(addr: *mut ()) -> bool
{
	false
}
pub unsafe fn map(addr: *const (), phys: u64, prot: ProtectionMode)
{
	log_debug!("map({:p} = {:#x}, {:?})", addr, phys, prot);

	// 3. Set mapping in level3
	let val = phys | prot_mode_to_attrs(prot) | 0x403;
	with_leaf_entry(addr, true, |e| {
		if let Err(old) = e.compare_exchange(0, val, Ordering::SeqCst, Ordering::SeqCst) {
			panic!("map() called over existing allocation: a={:p}, old={:#x}", addr, old);
		}
		});
	// Invalidate TLB for this address
	tlbi(addr);
	{
		let readback = get_phys(addr);
		assert!(readback == phys, "{:p} readback {:#x} != set {:#x}", addr, readback, phys);
	}
}
pub unsafe fn reprotect(addr: *const (), prot: ProtectionMode)
{
	with_leaf_entry(addr, false, |e| {
		let v = e.load(Ordering::SeqCst);
		let new_val = (v & 0x00FFFFFF_FFFFF000) | (prot_mode_to_attrs(prot) | 0x403);
		if let Err(_) = e.compare_exchange(v, new_val, Ordering::SeqCst, Ordering::SeqCst) {
			panic!("Race in reprotect");
		}
		log_debug!("reprotect({:p}) {:p} {:#x} -> {:#x}", addr, e, v, new_val);
		});
	tlbi(addr);
}
pub unsafe fn unmap(addr: *const ()) -> Option<u64>
{
	with_leaf_entry(addr, false, |e| {
		e.swap(0, Ordering::SeqCst) & 0x00FFFFFF_FFFFF000
		})
}
/// Invalidate the TLB entries associated with the specified address
fn tlbi(addr: *const ()) {
	// SAFE: TLBI can't cause unsafety
	unsafe {
		static_assert!(PAGE_SIZE == 1 << (12+2));
		const MASK: usize = ((1 << 43)-1) & !3;	// 43 bits of address (after shifting by 12), mask out bottom two bits for 14bit page size
		::core::arch::asm!("TLBI VAE1, {}", in(reg) (addr as usize >> 12) & MASK);
	}
}


pub unsafe fn fixed_alloc(phys: u64, count: usize) -> Option<*mut ()>
{
	None
}
pub fn is_fixed_alloc(addr: *const (), count: usize) -> bool
{
	if addresses::IDENT_START <= addr as usize && addr as usize + count * PAGE_SIZE <= (addresses::IDENT_START + addresses::IDENT_SIZE) {
		return true;
	}
	false
}


static S_TEMP_MAP_SEM: crate::sync::Semaphore = crate::sync::Semaphore::new(2048, 2048);
extern "C" {
	static kernel_temp_mappings: [AtomicU64; 2048];
}
pub unsafe fn temp_map<T>(phys: u64) -> *mut T
{
	let v = phys | prot_mode_to_attrs(ProtectionMode::KernelRW) | 0x403;
	S_TEMP_MAP_SEM.acquire();
	for i in 0 .. 2048
	{
		if let Ok(_) = kernel_temp_mappings[i].compare_exchange(0, v, Ordering::SeqCst, Ordering::Relaxed)
		{
			let addr = addresses::TEMP_BASE + i * PAGE_SIZE;
			tlbi(addr as *const _);
			return addr as *mut _;
		}
	}
	panic!("temp_map: Semaphore returned ok but no mappings");
}
pub unsafe fn temp_unmap<T>(addr: *mut T)
{
	assert!(addresses::TEMP_BASE <= addr as usize);
	let i = (addr as usize - addresses::TEMP_BASE) / PAGE_SIZE;
	assert!(i < 2048);
	let v = kernel_temp_mappings[i].swap(0, Ordering::SeqCst);
	assert!(v != 0);
	tlbi(addr as *const ());
	S_TEMP_MAP_SEM.release();
}


impl AddressSpace
{
	pub fn pid0() -> AddressSpace
	{
		extern "C" {
			static user0_root: crate::Extern;
		}
		// SAFE: Just need the address
		AddressSpace(get_phys(unsafe { &user0_root as *const _ as *const () }))
	}
	pub fn new(clone_start: usize, clone_end: usize) -> Result<AddressSpace,crate::memory::virt::MapError>
	{
		use crate::memory::virt::MapError;

		// TODO: Make this more common between architectures (a generic bound for `Level`)?
		struct NewTable(crate::arch::memory::virt::TempHandle<u64>, /** Level for the contained items */Level);
		impl NewTable {
			fn new(level: Level) -> Result<NewTable,MapError> {
				match crate::memory::phys::allocate_bare()
				{
				Err(crate::memory::phys::Error) => Err( MapError::OutOfMemory ),
				Ok(temp_handle) => Ok( NewTable( temp_handle.into(), level ) ),
				}
			}
			fn phys(&self) -> super::PAddr {
				self.0.phys_addr()
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

		fn opt_clone_table_ent(table_level: Level, base_addr: usize, clone_start: usize, clone_end: usize, prev_table_pte: u64) -> Result<u64, MapError>
		{
			let (next,size) = match table_level
				{
				Level::Root   => (Some(Level::Middle), 1 << (11+11+14)),
				Level::Middle => (Some(Level::Bottom), 1 << (11+14)),
				Level::Bottom => (None, 1 << 14),
				};
			if prev_table_pte == 0
			{
				Ok(0)
			}
			else if clone_end <= base_addr || base_addr + size <= clone_start
			{
				Ok(0)
			}
			else if next.is_none() || prev_table_pte & 3 == 1 {
				assert!(size == PAGE_SIZE, "TODO: Large blocks in clone region");
				const PADDR_MASK: u64 = 0x0000FFFF_FFFFF000;
				const ATTR_MASK: u64 = !PADDR_MASK;
				let paddr = prev_table_pte & PADDR_MASK;
				let paddr = match attrs_to_prot_mode(prev_table_pte)
					{
					ProtectionMode::UserRX | ProtectionMode::UserCOW => {
						crate::memory::phys::ref_frame( paddr );
						paddr
						},
					ProtectionMode::UserRWX | ProtectionMode::UserRW => {
						// SAFE: We've just determined that this page is mapped in, so we won't crash. Any race is the user's fault (and shouldn't impact the kernel)
						let src = unsafe { ::core::slice::from_raw_parts(base_addr as *const u8, PAGE_SIZE) };
						let mut newpg = crate::memory::virt::alloc_free()?;
						newpg.copy_from_slice(src);
						newpg.into_frame().into_addr()
						}
					v @ _ => todo!("opt_clone_table_ent - Mode {:?}", v),
					};
				let rv = paddr | (prev_table_pte & ATTR_MASK);
				//log_debug!("opt_clone_table_ent: {:#x} {:#x} -> {:#x}", base_addr, prev_table_pte, rv);
				Ok( rv )
			}
			else
			{
				let next = next.unwrap();	// None checked above
				let slot_base = (base_addr / size) << 11;
				let mut table = NewTable::new(next)?;
				for i in 0 .. 2048
				{
					let slot = slot_base + i;
					let a = slot * (size >> 11);
					// SAFE: Read-only access
					unsafe {
						table[i] = with_entry(Space::User, next, slot, |e| {
							opt_clone_table_ent(next, a, clone_start, clone_end, e.load(Ordering::Relaxed))
							})?;
					}
				}
				let rv = table.into_frame() | 0x403;
				//log_debug!("opt_clone_table_ent: > {:?} {:#x} {:#x} -> {:#x}", table_level, base_addr, prev_table_pte, rv);
				Ok( rv )
			}
		}
		let mut table = NewTable::new(Level::Root)?;
		for i in 0 .. (2048-1)
		{
			let a = i << (11+11+14);
			// SAFE: Entry is available, only read
			unsafe { with_entry(Space::User, Level::Root, i, |e|->Result<(),MapError> {
				table[i] = opt_clone_table_ent(Level::Root, a, clone_start, clone_end, e.load(Ordering::Relaxed))?;
				Ok(())
				})? };
		}
		table[2047] = table.phys() | 0x403;
		Ok(AddressSpace(table.into_frame()))
	}

	pub fn as_phys(&self) -> u64 {
		self.0
	}
}

pub fn data_abort(_esr: u64, far: usize) -> bool
{
	if let Some( (phys, ProtectionMode::UserCOW) ) = get_info(far as *const ())
	{
		// Ensure lock is held before manipulation
		// SAFE: Correct PTE manipulation
		crate::memory::virt::with_lock(far, || unsafe {
			with_leaf_entry(far as *const (), false, |e| {
				let v = e.load(Ordering::SeqCst);
				if attrs_to_prot_mode(v) == ProtectionMode::UserCOW
				{
					let frame = phys & !(PAGE_SIZE as u64 - 1);
					let pgaddr = far & !(PAGE_SIZE - 1);
					// 2. Get the PMM to provide us with a unique copy of that frame (can return the same addr)
					// - This borrow is valid, as the page is read-only (for now)
					let newframe = crate::memory::phys::make_unique( frame, &*(pgaddr as *const [u8; PAGE_SIZE]) );
					let new_val = newframe | prot_mode_to_attrs(ProtectionMode::UserRW) | 0x403;
					
					if let Err(_) = e.compare_exchange(v, new_val, Ordering::SeqCst, Ordering::Relaxed)
					{
						todo!("data_abort: Contention for COW");
					}
				}
				});
			});
		return true;
	}
	false
}

