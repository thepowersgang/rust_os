// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/arch/armv8/memory/virt.rs
//! Virtual memory interface
// NOTE NOTE: Page size on ARMv8 (this config) is 0x4000 (16K, 2<<14) - Keeps things interesting
use memory::virt::ProtectionMode;
use PAGE_SIZE;
use super::addresses;
use core::sync::atomic::{Ordering,AtomicU64};

const KERNEL_FRACTAL_BASE: usize = 0xFFFF_FFE0_0000_0000;
const LEVEL2_FRACTAL_OFS: usize = (2048-2)*2048*2048;	// Offset in KERNEL_FRACTAL for level2 tables
const LEVEL1_FRACTAL_OFS: usize = (2048-2)*2048*2048 + (2048-2)*2048;	// Offset in KERNEL_FRACTAL for level1 table (root)

pub struct AddressSpace(u64);

pub fn post_init()
{
	// No changes needed after init
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
		Some( ((v & 0x0000FFFF_FFFFF000) + (addr as usize & 0xFFF)) as u64 )
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

	const USER_SIZE: usize = 1 << 47;
	const PAGE_MASK: usize = (USER_SIZE / PAGE_SIZE) - 1;
	let page = (addr as usize / PAGE_SIZE) & PAGE_MASK;
	let sign_bits = addr as usize >> 48;
	let kernel_bit = (addr as usize >> 47) & 1;
	assert!(sign_bits == [0,0xFFFF][kernel_bit], "Non-canonical address {:p} ({:#x})", addr, sign_bits);
	//log_trace!("page = {}:{:#x}", b"UK"[is_kernel as usize], page);
	// If the address is above the 48-bit user-kernel split
	if kernel_bit != 0
	{
		// Kernel AS doesn't need a deletion lock, as it's never pruned
		// Mutation lock also not needed (but is provided in VMM)

		// 0x4000 / 8 = 0x800 = 11 bits

		// 1. Ensure that top-level region is valid.
		with_entry(Level::Root, page >> 22, |e| {
			if e.load(Ordering::Relaxed) == 0 {
				log_debug!("Allocate Level2 @ {:#x}", page >> 22);
				::memory::phys::allocate( get_entry_addr(Level::Middle, page >> 22 << 11) as *mut () );
				assert!(e.load(Ordering::Relaxed) != 0);
			}
			log_debug!("map: Root[{:#x}]={:#x}", page >> 22, e.load(Ordering::Relaxed));
			});
		// 2. Ensure that level2 is valid
		with_entry(Level::Middle, page >> 11, |e| {
			if e.load(Ordering::Relaxed) == 0 {
				log_debug!("Allocate Level3 @ {:#x}", page >> 11);
				::memory::phys::allocate( get_entry_addr(Level::Bottom, page >> 11 << 11) as *mut () );
				assert!(e.load(Ordering::Relaxed) != 0);
			}
			log_debug!("map: Middle[{:#x}]={:#x}", page >> 11, e.load(Ordering::Relaxed));
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
	tlbi(addr);
	{
		let readback = get_phys(addr);
		assert!(readback == phys, "{:p} readback {:#x} != set {:#x}", addr, readback, phys);
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
/// Invalidate the TLB entries associated with the specified address
fn tlbi(addr: *const ()) {
	// SAFE: TLBI can't cause unsafety
	unsafe {
		static_assert!(PAGE_SIZE == 1 << (12+2));
		const MASK: usize = ((1 << 43)-1) & !3;	// 43 bits of address (after shifting by 12), mask out bottom two bits for 14bit page size
		asm!("TLBI VAE1, {}", in(reg) (addr as usize >> 12) & MASK);
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
			static user0_root: ::Extern;
		}
		// SAFE: Just need the address
		AddressSpace(get_phys(unsafe { &user0_root as *const _ as *const () }))
	}
	pub fn new(start: usize, end: usize) -> Result<AddressSpace,()>
	{
		todo!("AddressSpace::new");
	}

	pub fn as_phys(&self) -> u64 {
		self.0
	}
}

