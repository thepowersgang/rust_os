//
//
//
//!
use memory::virt::ProtectionMode;
use PAGE_SIZE;
use core::sync::atomic::{Ordering,AtomicU64};

const KERNEL_FRACTAL_BASE: usize = 0xFFFF_FFE0_0000_0000;
const LEVEL2_FRACTAL_OFS: usize = (2048-2)*2048*2048;	// Offset in KERNEL_FRACTAL for level2 tables
const LEVEL1_FRACTAL_OFS: usize = (2048-2)*2048*2048 + (2048-2)*2048;	// Offset in KERNEL_FRACTAL for level1 table (root)

pub struct AddressSpace(u64);

pub fn post_init()
{
}


pub fn is_reserved<T>(addr: *const T) -> bool
{
	false
}
pub fn get_phys<T>(addr: *const T) -> u64
{
	0
}
pub fn get_info<T>(addr: *const T) -> Option<(u64, ProtectionMode)>
{
	None
}

#[derive(Debug,Copy,Clone)]
enum Level
{
	Root,
	Middle,
	Bottom,
}

fn get_entry_addr(level: Level, index: usize) -> *const AtomicU64
{
	(match level
	{
	Level::Root => {
		assert!(index < 2048);
		KERNEL_FRACTAL_BASE + (LEVEL1_FRACTAL_OFS + index) * 8
		},
	Level::Middle => {
		assert!(index < 2048*2048);
		KERNEL_FRACTAL_BASE + (LEVEL2_FRACTAL_OFS + index) * 8
		},
	Level::Bottom => {
		assert!(index < 2048*2048*2048);
		KERNEL_FRACTAL_BASE + index * 8
		}
	}) as *const _
}

fn with_entry<F, R>(level: Level, index: usize, fcn: F) -> R
where
	F: FnOnce(&AtomicU64)->R
{
	let ptr = get_entry_addr(level, index);
	debug_assert!(get_info(ptr).is_some());
	log_trace!("with_entry({:?}, {}): ptr={:p}", level, index, ptr);
	
	// SAFE: Pointer is asserted to be valid above
	fcn( unsafe { &*ptr } )
}

fn prot_mode_to_flags(prot: ProtectionMode) -> u64 {
	match prot
	{
	_ => 0,
	}
}

pub fn can_map_without_alloc(addr: *mut ()) -> bool
{
	false
}
pub unsafe fn map(addr: *const (), phys: u64, prot: ProtectionMode)
{
	log_debug!("map({:p} = {:#x}, {:?})", addr, phys, prot);

	let page = addr as usize / PAGE_SIZE;
	if page >> (48-14) > 0
	{
		// Kernel AS doesn't need locking, as it's never pruned
		let mask = (1 << 33)-1;
		let page = page & mask;
		log_trace!("page = {:#x}", page);
		// 1. Ensure that top-level region is valid.
		with_entry(Level::Root, page >> 22, |e| {
			if e.load(Ordering::Relaxed) == 0 {
				::memory::virt::allocate( get_entry_addr(Level::Middle, page >> 22 << 11) as *mut (), 1 );
			}
			});
		// 2. Ensure that level2 is valid
		with_entry(Level::Middle, page >> 11, |e| {
			if e.load(Ordering::Relaxed) == 0 {
				::memory::virt::allocate( get_entry_addr(Level::Bottom, page >> 11 << 11) as *mut (), 1 );
			}
			});
		// 3. Set mapping in level3
		let val = phys | prot_mode_to_flags(prot) | 0x403;
		with_entry(Level::Bottom, page, |e| {
			if let Err(old) = e.compare_exchange(0, val, Ordering::SeqCst, Ordering::SeqCst) {
				panic!("map() called over existing allocation: a={:p}, old={:#x}", addr, old);
			}
			});
	}
	else
	{
		// TODO: Lock address space
		todo!("map - user");
	}
}
pub unsafe fn reprotect(addr: *const (), prot: ProtectionMode)
{
	todo!("reprotect");
}
pub unsafe fn unmap(addr: *const ()) -> Option<u64>
{
	None
}


pub unsafe fn fixed_alloc(phys: u64, count: usize) -> Option<*mut ()>
{
	None
}
pub fn is_fixed_alloc(addr: *const (), count: usize) -> bool
{
	false
}


pub unsafe fn temp_map<T>(phys: u64) -> *mut T
{
	todo!("");
}
pub unsafe fn temp_unmap<T>(addr: *mut T)
{
	todo!("");
}


impl AddressSpace
{
	pub fn pid0() -> AddressSpace
	{
		extern "C" {
			static kernel_root: [u64; 2048];
		}
		AddressSpace(kernel_root[2048-2] & !0x3FFF)
	}
	pub fn new(start: usize, end: usize) -> Result<AddressSpace,()>
	{
		todo!("");
	}

	pub fn as_phys(&self) -> u64 {
		self.0
	}
}

