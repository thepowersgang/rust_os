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

fn prot_mode_to_attrs(prot: ProtectionMode) -> u64
{
	match prot
	{
	//ProtectionMode::KernelRWX=> (0x00<<56 | 0x00<<2),
	//ProtectionMode::UserRWX  => (0x00<<56 | 0x40<<2),
	ProtectionMode::KernelRX => (0x00<<56 | 0x80<<2),
	ProtectionMode::UserRX   => (0x00<<56 | 0xC0<<2),
	ProtectionMode::KernelRW => (0x20<<56 | 0x00<<2),
	ProtectionMode::UserRW   => (0x20<<56 | 0x40<<2),
	ProtectionMode::KernelRO => (0x20<<56 | 0x80<<2),
	ProtectionMode::UserRO   => (0x20<<56 | 0xC0<<2),
	_ => 0,
	}
}
fn attrs_to_prot_mode(attrs: u64) -> ProtectionMode
{
	let v = (((attrs >> 56) & 0xFF) << 8) | ((attrs >> 2) & 0xFF);
	match v
	{
	0x00_00 => ProtectionMode::KernelRW,	// RWX
	0x00_40 => ProtectionMode::UserRW,	// RWX
	0x00_80 => ProtectionMode::KernelRX,
	0x00_C0 => ProtectionMode::UserRX,
	0x20_00 => ProtectionMode::KernelRW,
	0x20_40 => ProtectionMode::UserRW,
	0x20_80 => ProtectionMode::KernelRO,
	0x20_C0 => ProtectionMode::UserRO,
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
		let a = with_entry(Level::Middle, addr as usize >> (14+11), |e| {
			let v = e.load(Ordering::Relaxed);
			if v & 3 == 3 { None } else { Some(v) }
			})
			.unwrap_or_else(|| with_entry(Level::Bottom, addr as usize >> 14, |e| e.load(Ordering::Relaxed)))
			;
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
		asm!("AT S1E1R, {1}; mrs {0}, PAR_EL1", out(reg) ret, in(reg) addr, options(pure, readonly, nostack));
		ret
		};
	if v & 1 != 0 {
		None
	}
	else {
		Some( ((v & 0x0000FFFF_FFFFF000) + (addr as usize % PAGE_SIZE)) as u64 )
	}
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
	debug_assert!(is_reserved(ptr));
	//log_trace!("with_entry({:?}, {}): ptr={:p}", level, index, ptr);
	
	// SAFE: Pointer is asserted to be valid above
	fcn( unsafe { &*ptr } )
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
		// Kernel AS doesn't need a deletion lock, as it's never pruned
		// Mutation lock also not needed (but is provided in VMM)
		
		let mask = (1 << 33)-1;
		let page = page & mask;
		//log_trace!("page = {:#x}", page);
		// 1. Ensure that top-level region is valid.
		with_entry(Level::Root, page >> 22, |e| {
			if e.load(Ordering::Relaxed) == 0 {
				::memory::phys::allocate( get_entry_addr(Level::Middle, page >> 22 << 11) as *mut () );
			}
			});
		// 2. Ensure that level2 is valid
		with_entry(Level::Middle, page >> 11, |e| {
			if e.load(Ordering::Relaxed) == 0 {
				::memory::phys::allocate( get_entry_addr(Level::Bottom, page >> 11 << 11) as *mut () );
			}
			});
		// 3. Set mapping in level3
		let val = phys | prot_mode_to_attrs(prot) | 0x403;
		with_entry(Level::Bottom, page, |e| {
			if let Err(old) = e.compare_exchange(0, val, Ordering::SeqCst, Ordering::SeqCst) {
				panic!("map() called over existing allocation: a={:p}, old={:#x}", addr, old);
			}
			});
	}
	else
	{
		// NOTE: Locking of address space not needed, as the VMM does that.
		todo!("map - user");
	}
	// Invalidate TLB for this address
	// SAFE: Safe assembly
	//unsafe {
		let MASK: usize = ((1 << 43)-1) & !3;	// 43 bits of address (after shifting by 12)
		asm!("TLBI VAE1, {}", in(reg) (addr as usize >> 12) & MASK);
	//}
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
		// SAFE: Constant value
		AddressSpace(unsafe { kernel_root[2048-2] & !0x3FFF })
	}
	pub fn new(start: usize, end: usize) -> Result<AddressSpace,()>
	{
		todo!("");
	}

	pub fn as_phys(&self) -> u64 {
		self.0
	}
}

