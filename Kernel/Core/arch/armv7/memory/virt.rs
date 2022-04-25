//
//
//

use crate::memory::virt::ProtectionMode;
use crate::arch::memory::PAddr;
use crate::arch::memory::{PAGE_SIZE, PAGE_MASK};
use crate::arch::memory::virt::TempHandle;
use ::core::sync::atomic::{Ordering,AtomicU32};

extern "C" {
	static kernel_table0: [AtomicU32; 0x800*2];
	static kernel_exception_map: [AtomicU32; 1024];
}

const KERNEL_TEMP_BASE : usize = 0xFFC00000;
const KERNEL_TEMP_COUNT: usize = 1023/2;	// 1024 4KB entries, PAGE_SIZE is 8KB, final 4KB is left zero for the -1 deref mapping
const USER_TEMP_BASE : usize = 0x7FC0_0000;	// 4MB region at end of user space
const USER_TEMP_COUNT: usize = 0x1000 / 8 - 3;	// Top three entries are: Mapping metadata, user last table fractal, user base table
const USER_TEMP_MDATA: usize = 0x7FFF_A000;
const USER_LAST_TABLE_FRAC: usize = 0x7FFF_C000;
const USER_BASE_TABLE: usize = 0x7FFF_E000;	// 2GB - 0x800 * 4 = 0x2000

const PAGE_MASK_U32: u32 = PAGE_MASK as u32;

// TODO: Why is this -1 here?
static S_TEMP_MAP_SEMAPHORE: crate::sync::Semaphore = crate::sync::Semaphore::new(KERNEL_TEMP_COUNT as isize - 1, KERNEL_TEMP_COUNT as isize);

pub fn post_init() {
	// SAFE: Atomic
	unsafe { kernel_table0[0].store(0, Ordering::SeqCst) };

	// SAFE: Valid memory passed to init_at
	unsafe {
		crate::memory::virt::allocate(USER_TEMP_MDATA as *mut (), 1).expect("Couldn't allocate space for user temp metadat");
		pl_temp::init_at(USER_TEMP_MDATA as *mut ());
	}
	//dump_tables();
}

fn dump_tables() {
	let mut start = 0;
	// Uses get_phys_opt becuase it doesn't need to update mappings
	let mut exp = get(0);
	for page in 1 .. 0xFFFF_F {
		if let Some(ref mut v) = exp {
			*v += 0x1000;
		}
		let addr = page * 0x1000;
		let v = get( addr );
		if v != exp {
			print(start, addr-1, exp);
			start = addr;
			exp = v;
		}
	}
	print(start, !0, exp);

	fn get(addr: usize) -> Option<u32> {
		// SAFE: Correct register accesses
		unsafe {
			// TODO: Disable interrupts during this operation
			let mut res: u32;
			::core::arch::asm!("mcr p15,0, {1}, c7,c8,0; isb; mrc p15,0, {0}, c7,c4,0 ", lateout(reg) res, in(reg) addr);
			if res & 1 == 1 {
				return None;
			}
			let paddr = res & !0xFFF;
			// Try a kernel write to test writiable
			::core::arch::asm!("mcr p15,0, {1}, c7,c8,1; isb; mrc p15,0, {0}, c7,c4,0 ", lateout(reg) res, in(reg) addr);
			let is_kwrite = res & 1 == 0;
			// Try a user read to test readable
			::core::arch::asm!("mcr p15,0, {1}, c7,c8,2; isb; mrc p15,0, {0}, c7,c4,0 ", lateout(reg) res, in(reg) addr);
			let is_uread = res & 1 == 0;

			Some(paddr | 1*(is_kwrite as u32) | 2*(is_uread as u32))
		}
	}
	fn print(start: usize, end: usize, exp_val: Option<u32>) {
		match exp_val
		{
		None    => log_trace!("{:08x}-{:08x} --", start, end),
		Some(e) => {
			let (paddr, flags) = (e & !0xFFF, e & 0xFFF);
			// NOTE: Flags aren't available
			log_trace!("{:08x}-{:08x} = {:08x}-{:08x} {}", start, end, paddr as usize - (end-start+1), paddr-1, flags);
			},
		}
	}
}

fn user_root_table() -> &'static [AtomicU32; 2048]
{
	// SAFE: This memory is always mapped. (TODO: Shouldn't be Sync)
	unsafe {
		let table_ptr = USER_BASE_TABLE as *const _;
		assert!( get_phys(table_ptr) != 0 );
		&*table_ptr
	}
}
fn user_last_table() -> &'static [AtomicU32; 0x2000 / 4] {
	// SAFE: This memory is always mapped. (TODO: Shouldn't be Sync)
	unsafe {
		let table_ptr = USER_LAST_TABLE_FRAC as *const _;
		assert!( get_phys(table_ptr) != 0 );
		&*table_ptr
	}
}

fn prot_mode_to_flags(mode: ProtectionMode) -> u32 {
	//AP[2] = 9, AP[1:0] = 5:4, XN=0, SmallPage=1
	match mode
	{
	ProtectionMode::Unmapped => 0x000,
	ProtectionMode::KernelRO => 0x213,
	ProtectionMode::KernelRW => 0x013,
	ProtectionMode::KernelRX => 0x012,
	ProtectionMode::UserRO => 0x233,	// 1,11,1
	ProtectionMode::UserRW => 0x033,	// 0,11,1
	ProtectionMode::UserRX => 0x232,	// 1,11,0
	ProtectionMode::UserRWX => 0x032,	// 0,11,0
	ProtectionMode::UserCOW => 0x223,	// 1,10,1 is a deprecated encoding for ReadOnly, need to find a better encoding
	}
}
fn flags_to_prot_mode(flags: u32) -> ProtectionMode {
	match flags & 0x233
	{
	0x000 => ProtectionMode::Unmapped,
	0x213 => ProtectionMode::KernelRO,
	0x013 => ProtectionMode::KernelRW,
	0x212 => ProtectionMode::KernelRX,
	0x233 => ProtectionMode::UserRO,
	0x033 => ProtectionMode::UserRW,
	0x232 => ProtectionMode::UserRX,
	0x032 => ProtectionMode::UserRWX,
	0x223 => ProtectionMode::UserCOW,
	v @ _ => todo!("Unknown mode value {:#x}", v),
	}
}

pub fn is_fixed_alloc<T>(addr: *const T, size: usize) -> bool {
	const BASE: usize = super::addresses::KERNEL_BASE;
	const ONEMEG: usize = 1024*1024;
	const LIMIT: usize = super::addresses::KERNEL_BASE + 8*ONEMEG;
	let addr = addr as usize;
	if BASE <= addr && addr < LIMIT {
		if addr + size <= LIMIT {
			true
		}
		else {
			false
		}
	}
	else {
		false
	}
}
// UNSAFE: Can cause aliasing
pub unsafe fn fixed_alloc(_p: PAddr, _count: usize) -> Option<*mut ()> {
	None
}

#[derive(Copy,Clone,Debug)]
enum PageEntryRegion {
	NonGlobal,
	Global,
}
impl PageEntryRegion {
	fn get_section_ent(&self, idx: usize) -> &AtomicU32 {
		match self
		{
		&PageEntryRegion::NonGlobal => {
			assert!(idx < 2048);
			&user_root_table()[idx]
			},
		&PageEntryRegion::Global => {
			assert!(idx < 4096);
			// SAFE: Atomic pointer
			unsafe { &kernel_table0[idx] }
			},
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
		mapping: TempHandle<AtomicU32>,
		idx: usize,
		ofs: usize
		},
}
impl PageEntry
{
	//fn alloc(addr: *const (), level: usize) -> Result<PageEntry, ()> {
	//	todo!("PageEntry::alloc({:p}, level={})", addr, level);
	//}
	/// Obtain a page entry for the specified address
	fn get(addr: *const ()) -> PageEntry {
		use super::addresses::KERNEL_BASE;
		let (rgn, p_idx) = if (addr as usize) < KERNEL_BASE {
				(PageEntryRegion::NonGlobal, (addr as usize) >> 12)
			}
			else {
				(PageEntryRegion::Global, (addr as usize) >> 12)
			};

		// SAFE: Aliasing in this case is benign
		let sect_ent = rgn.get_section_ent(p_idx >> 8).load(Ordering::SeqCst);
		if sect_ent & 0b11 == 0b01 {
			PageEntry::Page {
				// SAFE: Alias is beign, as accesses are atomic
				mapping: unsafe { TempHandle::new( sect_ent & !PAGE_MASK_U32 ) },
				idx: p_idx,
				ofs: (addr as usize) & PAGE_MASK,
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
		match self
		{
		&PageEntry::Section { rgn, idx, .. } => (rgn.get_section_ent(idx).load(Ordering::SeqCst) & 3 != 0),
		&PageEntry::Page { ref mapping, idx, .. } => (mapping[idx & 0x3FF].load(Ordering::SeqCst) & 3 != 0),
		}
	}

	fn phys_addr(&self) -> PAddr {
		match self
		{
		&PageEntry::Section { rgn, idx, ofs } => (rgn.get_section_ent(idx).load(Ordering::SeqCst) & !PAGE_MASK_U32) + ofs as u32,
		&PageEntry::Page { ref mapping, idx ,ofs } => (mapping[idx & 0x3FF].load(Ordering::SeqCst) & !PAGE_MASK_U32) + ofs as u32,
		}
	}
	fn mode(&self) -> ProtectionMode {
		match self
		{
		&PageEntry::Section { rgn, idx, .. } =>
			match rgn.get_section_ent(idx).load(Ordering::SeqCst) & PAGE_MASK_U32
			{
			0x000 => ProtectionMode::Unmapped,
			0x402 => ProtectionMode::KernelRW,
			v @ _ if v & 3 == 1 => unreachable!(),
			v @ _ => todo!("Unknown mode value in section {:?} {} - {:#x}", rgn, idx, v),
			},
		&PageEntry::Page { ref mapping, idx, .. } => flags_to_prot_mode( mapping[idx & 0x3FF].load(Ordering::SeqCst) & PAGE_MASK_U32 ),
		}
	}
	//fn reset(&mut self) -> Option<(PAddr, ProtectionMode)> {
	//}
	
	unsafe fn set(&mut self, addr: PAddr, mode: ProtectionMode) -> Option<(PAddr, ProtectionMode)> {
		match self
		{
		&mut PageEntry::Section { .. } => todo!("Calling PageEntry::set on Section"),
		&mut PageEntry::Page { ref mapping, idx, .. } => {
			let flags = prot_mode_to_flags(mode);
			let old = mapping[idx & 0x3FF].swap( (addr as u32) | flags, Ordering::SeqCst );
			log_debug!("set(): {:#x} -> {:#x}", old, (addr as u32) | flags);
			if old & PAGE_MASK_U32 == 0 {
				None
			}
			else {
				Some( ((old & !PAGE_MASK_U32) as PAddr, flags_to_prot_mode(old & PAGE_MASK_U32)) )
			}
			},
		}
	}
}
impl_fmt! {
	Debug(self, f) for PageEntry {
		if self.is_reserved() {
			write!(f, "{:#x}-{:?}", self.phys_addr(), self.mode())
		}
		else {
			write!(f, "Unmapped()")
		}
	}
}


#[allow(dead_code)]
/// Process-local temporary mappings
mod pl_temp
{
	use super::{USER_TEMP_BASE,USER_TEMP_COUNT};
	use crate::PAGE_SIZE;
	use core::sync::atomic::Ordering;

	struct MetaDataInner {
		counts: [u32; USER_TEMP_COUNT],
	}
	impl MetaDataInner {
		fn mappings(&self) -> &[super::AtomicU32] {
			&super::user_last_table()[1024..]
		}
	}
	struct MetaData {
		free: crate::sync::Semaphore,
		lock: crate::sync::Mutex<MetaDataInner>,
	}
	impl MetaData {
		// UNSAFE: User must not leak this pointer across process boundaries
		unsafe fn get() -> &'static MetaData {
			let addr = super::USER_TEMP_MDATA as *mut MetaData;
			assert!( super::get_phys(addr) != 0 );
			&*addr
		}
	}

	pub unsafe fn init_at(addr: *mut ()) {
		let addr = addr as *mut MetaData;
		*addr = MetaData {
			free: crate::sync::Semaphore::new(USER_TEMP_COUNT as isize, USER_TEMP_COUNT as isize),
			lock: crate::sync::Mutex::new(MetaDataInner {
				counts: [0; USER_TEMP_COUNT],
				}),
			};
	}

	// UNSAFE: Can cause aliasing
	pub unsafe fn map_temp(paddr: crate::memory::PAddr) -> ProcTempMapping {
		let val = paddr as u32 | 0x13;

		// SAFE: Only used locally
		let md = /*unsafe*/ { MetaData::get() };
		md.free.acquire();
		let mut lh = md.lock.lock();

		// 1. Semaphore
		// 2. Search through entries search for one with the same paddr
		for i in 0 .. USER_TEMP_COUNT
		{
			let v = lh.mappings()[i*2].load(Ordering::Relaxed);
			
			if v == val {
				lh.counts[i] += 1;
				return ProcTempMapping( (USER_TEMP_BASE + i*PAGE_SIZE) as *const _ );
			}
			//else if v == 0 {
			//	first_free = Some(i);
			//}
			//else lh.counts[i] == 0 {
			//	// TODO: Want to avoid churning a slot
			//	first_zero = Some(i);
			//}
		}
		for i in 0 .. USER_TEMP_COUNT
		{
			if let Ok(_) = lh.mappings()[i*2].compare_exchange(0, val, Ordering::Relaxed, Ordering::Relaxed) {
				lh.mappings()[i*2+1].store(val + 0x1000, Ordering::Relaxed);
				lh.counts[i] += 1;

				let addr = USER_TEMP_BASE + i*PAGE_SIZE;
				super::tlbimva( addr as *mut () );
				return ProcTempMapping( addr as *const _ );
			}
		}
		for i in 0 .. USER_TEMP_COUNT
		{
			if lh.counts[i] == 0 {
				lh.mappings()[i*2  ].store(val         , Ordering::Relaxed);
				lh.mappings()[i*2+1].store(val + 0x1000, Ordering::Relaxed);
				lh.counts[i] += 1;

				let addr = USER_TEMP_BASE + i*PAGE_SIZE;
				super::tlbimva( addr as *mut () );
				return ProcTempMapping( (USER_TEMP_BASE + i*PAGE_SIZE) as *const _ );
			}
		}
		panic!("Out of temp mappings but semaphore said there were some");
	}
	pub struct ProcTempMapping(*const [u8; PAGE_SIZE]);
	impl ProcTempMapping {
		pub fn get_slice<T: crate::lib::POD>(&self) -> &[T] {
			// SAFE: POD and alignment is always < 1 page
			unsafe {
				::core::slice::from_raw_parts( self.0 as *const T, PAGE_SIZE / ::core::mem::size_of::<T>() )
			}
		}
	}
	impl Drop for ProcTempMapping {
		fn drop(&mut self) {
			let i = (self.0 as usize - USER_TEMP_BASE) / PAGE_SIZE;
			// SAFE: Doesn't leak
			let md = unsafe { MetaData::get() };
			md.lock.lock().counts[i] -= 1;
			md.free.release();
		}
	}
}

/// Reference to a page table
enum TableRef {
	/// Statically allocated table (e.g. kernel root)
	Static(&'static [AtomicU32; 0x800]),
	/// 
	Dynamic(TempHandle<AtomicU32>),
	/////Process-local temporary
	//DynamicPL(pl_temp::ProcTempMapping),
}
impl ::core::ops::Deref for TableRef {
	type Target = [AtomicU32];
	fn deref(&self) -> &Self::Target {
		match self
		{
		&TableRef::Static(ref v) => &v[..],
		&TableRef::Dynamic(ref v) => &v[..],
		//&TableRef::DynamicPL(ref v) => v.get_slice(),
		}
	}
}

/// Returns the physical address of the table controlling `vaddr`. If `alloc` is true, a new table will be allocated if needed.
/// 
/// Return value is a reference to the able, and the index of the address within the table
fn get_table_addr<T>(vaddr: *const T, alloc: bool) -> Option< (TableRef, usize) > {
	let addr = vaddr as usize & !PAGE_MASK;
	let page = addr >> 12;	// NOTE: 12 as each entry in the table services 4KB
	let (ttbr_ofs, tab_idx) = (page >> 11, page & 0x7FF);
	const ENTS_PER_ALLOC: usize = PAGE_SIZE / 0x400;	// Each entry in the top-level table points to a 1KB second-level table
	
	let ent_r = if ttbr_ofs*ENTS_PER_ALLOC < 0x800 {
			if ttbr_ofs * ENTS_PER_ALLOC >= 0x800 - 8 {
				return Some( (TableRef::Static(user_last_table()), tab_idx) );
			}
			&user_root_table()[ttbr_ofs*ENTS_PER_ALLOC .. ][..ENTS_PER_ALLOC]
		}
		else {
			// Kernel
			// SAFE: Atomic static
			unsafe { &kernel_table0[ ttbr_ofs*ENTS_PER_ALLOC .. ][..ENTS_PER_ALLOC] }
		};
	
	let ent_v = ent_r[0].load(Ordering::SeqCst);
	match ent_v & PAGE_MASK_U32
	{
	0 => if alloc {
			log_debug!("New table for {:#x}", ttbr_ofs << (11+12));
			let mut handle: TempHandle<u32> = match crate::memory::phys::allocate_bare()
				{
				Ok(v) => v.into(),
				Err(e) => todo!("get_table_addr - alloc failed {:?}", e)
				};
			for v in handle.iter_mut() { *v = 0; }
			//crate::memory::virt::with_temp(|frame: &[AtomicU32]| for v in frame.iter { v.store(0) });

			let frame = handle.phys_addr();
			match ent_r[0].compare_exchange(0, frame + 0x1, Ordering::Acquire, Ordering::Relaxed)
			{
			Err(ent_v) => {
				// SAFE: Frame is owned by this function, and is umapped just after this
				unsafe { crate::memory::phys::deref_frame(frame); }
				drop(handle);
				// SAFE: Address is correct, and immutable
				let ret_handle = unsafe { TempHandle::new(ent_v & !PAGE_MASK_U32) };
				Some( (TableRef::Dynamic( ret_handle ), tab_idx) )
				},
			Ok(_/*0*/) => {
				for i in 1 .. ENTS_PER_ALLOC as u32 {
					assert!( ent_r[i as usize].load(Ordering::Relaxed) == 0 );
					ent_r[i as usize].store(frame + i*0x400 + 0x1, Ordering::SeqCst);
				}
				Some( (TableRef::Dynamic(handle.into()), tab_idx) )
				},
			}
		}
		else {
			None
		},
	// SAFE: Address is correct, and immutable
	1 => Some( (TableRef::Dynamic( unsafe { TempHandle::new(ent_v & !PAGE_MASK_U32) } ), tab_idx) ),
	//// SAFE: Address is correct, and immutable
	//1 => Some( (TableRef::DynamicPL( unsafe { pl_temp::map_temp(ent_v & !PAGE_MASK_U32) } ), tab_idx) ),
	0x402 => panic!("Called get_table_addr on large mapping @ {:p} (idx={:#x})", vaddr, ttbr_ofs*ENTS_PER_ALLOC),
	v @ _ => todo!("get_table_addr - Other flags bits {:#x} (for addr {:p})", v, vaddr),
	}
}


/// Temporarily map a frame into memory
/// UNSAFE: User to ensure that the passed address doesn't alias
pub unsafe fn temp_map<T>(phys: crate::arch::memory::PAddr) -> *mut T {
	//log_trace!("temp_map<{}>({:#x})", type_name!(T), phys);
	assert!(phys as u32 % PAGE_SIZE as u32 == 0);
	let val = (phys as u32) + 0x13;	
	
	S_TEMP_MAP_SEMAPHORE.acquire();
	for i in 0 .. KERNEL_TEMP_COUNT {
		let ents = &kernel_exception_map[i*2 ..][.. 2];
		if let Ok(_) = ents[0].compare_exchange(0, val, Ordering::Acquire, Ordering::Relaxed) {
			let addr = (KERNEL_TEMP_BASE + i * crate::PAGE_SIZE) as *mut _;
			//log_trace!("- Addr = {:p}", addr);
			// - Set the next node and check that it's zero
			assert!( ents[1].swap(val+0x1000, Ordering::Acquire) == 0 );
			tlbimva( addr as *mut () );
			return addr;
		}
	}
	panic!("No free temp mappings");
}
/// UNSAFE: Can cause use-after-free if address is used after call
pub unsafe fn temp_unmap<T>(addr: *mut T)
{
	//log_trace!("temp_unmap<{}>({:p})", type_name!(T), addr);
	assert!(addr as usize >= KERNEL_TEMP_BASE);
	let i = (addr as usize - KERNEL_TEMP_BASE) / crate::PAGE_SIZE;
	assert!(i < KERNEL_TEMP_COUNT);
	// - Clear in reverse order to avoid racing between the +1 release and +0's acquire
	kernel_exception_map[i*2+1].store(0, Ordering::Release);
	kernel_exception_map[i*2+0].store(0, Ordering::Release);
	S_TEMP_MAP_SEMAPHORE.release();
}

pub fn is_reserved<T>(addr: *const T) -> bool {
	get_phys_opt(addr).is_some()
	//PageEntry::get(addr as *const ()).is_reserved()
}
pub fn get_phys<T: ?Sized>(addr: *const T) -> crate::arch::memory::PAddr {
	get_phys_opt(addr as *const ()).unwrap_or(0)
}
fn get_phys_opt<T>(addr: *const T) -> Option<crate::arch::memory::PAddr> {
	let res: u32;
	// SAFE: Correct register accesses
	unsafe {
		// TODO: Disable interrupts during this operation
		::core::arch::asm!("mcr p15,0, {1}, c7,c8,0; isb; mrc p15,0, {0}, c7,c4,0 ", lateout(reg) res, in(reg) addr);
	};

	match res & 3 {
	1 | 3 => None,
	0 => Some( ((res as usize & !PAGE_MASK) | (addr as usize & PAGE_MASK)) as u32 ),
	2 => {
		todo!("Unexpected supersection at {:p}, res={:#x}", addr, res);
		//let pa_base: u64 = (res & !0xFFFFFF) as u64 | ((res as u64 & 0xFF0000) << (32-16));
		//Some( pa_base as u32 | (addr as usize as u32 & 0xFFFFFF) )
		},
	_ => unreachable!(),
	}
}

pub fn get_info<T>(addr: *const T) -> Option<(crate::arch::memory::PAddr, crate::memory::virt::ProtectionMode)> {
	let pe = PageEntry::get(addr as *const ());
	if pe.is_reserved() {
		Some( (pe.phys_addr(), pe.mode()) )
	}
	else {
		None
	}
}

/// TLB Invalidate by Modified Virtual Address
fn tlbimva(a: *mut ()) {
	// SAFE: TLB invalidation is not the unsafe part :)
	unsafe {
		// Note: since PAGE_SIZE is 0x2000, needs to be called twice
		::core::arch::asm!("mcr p15,0, {0}, c8,c7,1 ; dsb ; isb", in(reg) ((a as usize & !PAGE_MASK) | 1 ), options(nostack));
		::core::arch::asm!("mcr p15,0, {0}, c8,c7,1 ; dsb ; isb", in(reg) ((a as usize & !PAGE_MASK) | 0x1000 | 1 ), options(nostack));
	}
}
///// Data Cache Clean by Modified Virtual Address (to PoC)
//fn dccmvac(a: *mut ()) {
//}

pub fn can_map_without_alloc(a: *mut ()) -> bool {
	get_table_addr(a, false).is_some()
}

pub unsafe fn map(a: *mut (), p: PAddr, mode: ProtectionMode) {
	//log_debug!("map({:p} = {:#x}, {:?})", a, p, mode);
	return map_int(a,p,mode);
	
	// "Safe" helper to constrain interior unsafety
	fn map_int(a: *mut (), p: PAddr, mode: ProtectionMode) {
		// 1. Map the relevant table in the temp area
		let (mh, idx) = get_table_addr(a, true).unwrap();
		assert!(mode != ProtectionMode::Unmapped, "Invalid pass of ProtectionMode::Unmapped to map");
		assert!( idx % 2 == 0 );	// two entries per 8KB page

		// 2. Insert
		let mode_flags = prot_mode_to_flags(mode);
		if let Err(old) = mh[idx+0].compare_exchange(0, p + mode_flags, Ordering::SeqCst, Ordering::Relaxed) {
			panic!("map() called over existing allocation: a={:p}, old={:#x}", a, old);
		}
		mh[idx+1].swap(p + 0x1000 + mode_flags, Ordering::SeqCst);
		tlbimva(a);
	}
}
pub unsafe fn reprotect(a: *mut (), mode: ProtectionMode) {
	//log_debug!("reprotect({:p}, {:?})", a, mode);
	return reprotect_int(a, mode);

	fn reprotect_int(a: *mut (), mode: ProtectionMode) {
		// 1. Map the relevant table in the temp area
		let (mh, idx) = get_table_addr(a, false).expect("Calling reprotect() on unmapped location");
		assert!(mode != ProtectionMode::Unmapped, "Invalid pass of ProtectionMode::Unmapped to reprotect");
		assert!( idx % 2 == 0 );	// two entries per 8KB page

		// 2. Update
		let mode_flags = prot_mode_to_flags(mode);
		let v = mh[idx].load(Ordering::SeqCst);
		assert!(v != 0, "reprotect() called on an unmapped location: a={:p}", a);
		let p = v & !PAGE_MASK_U32;
		if let Err(old) = mh[idx+0].compare_exchange(v, p | mode_flags, Ordering::SeqCst, Ordering::Relaxed) {
			panic!("reprotect() called in a racy manner: a={:p} old({:#x}) != v({:#x})", a, old, v);
		}
		mh[idx+1].swap( (p + 0x1000) | mode_flags, Ordering::SeqCst);
		tlbimva( a );
	}
}
pub unsafe fn unmap(a: *mut ()) -> Option<PAddr> {
	log_debug!("unmap({:p})", a);
	return unmap_int(a);

	fn unmap_int(a: *mut()) -> Option<PAddr> {
		// 1. Map the relevant table in the temp area
		let (mh, idx) = get_table_addr(a, false).expect("Calling unmap() on unmapped location");
		assert!( idx % 2 == 0 );	// two entries per 8KB page

		let old = mh[idx+0].swap(0, Ordering::SeqCst);
		mh[idx+1].store(0, Ordering::SeqCst);
		tlbimva(a);
		if old & 3 == 0 {
			None
		}
		else {
			Some( old & !PAGE_MASK_U32 )
		}
	}
}

#[derive(Debug)]
pub struct AddressSpace(u32);
impl AddressSpace
{
	pub fn pid0() -> AddressSpace {
		extern "C" {
			static kernel_table0: crate::Extern;
		}
		// SAFE: Static.
		AddressSpace( get_phys( unsafe { &kernel_table0 } ) )
	}
	pub fn new(clone_start: usize, clone_end: usize) -> Result<AddressSpace,crate::memory::virt::MapError> {
		assert!( clone_start % crate::PAGE_SIZE == 0 );
		assert!( clone_end % crate::PAGE_SIZE == 0 );
		// 1. Allocate a new root-level table for the user code
		let mut new_root: TempHandle<u32> = crate::memory::phys::allocate_bare()?.into();
		// 2. Allocate the final table (for fractal)
		let mut last_tab: TempHandle<u32> = crate::memory::phys::allocate_bare()?.into();
		last_tab[2044] = (last_tab.phys_addr() as u32 + 0x0000) | 0x13;
		last_tab[2045] = (last_tab.phys_addr() as u32 + 0x1000) | 0x13;
		last_tab[2046] = (new_root.phys_addr() as u32 + 0x0000) | 0x13;
		last_tab[2047] = (new_root.phys_addr() as u32 + 0x1000) | 0x13;
		for i in 0 .. 8 {
			new_root[2048-8 + i as usize] = (last_tab.phys_addr() as u32 + i * 0x400) | 0x1;
		}

		if clone_start >> 20 < 2040 {
			todo!("Allocate extra tables for AddressSpace::new");
		}
		let start_pidx = clone_start / crate::PAGE_SIZE;
		let end_pidx = clone_end / crate::PAGE_SIZE;
		
		for page in start_pidx .. end_pidx
		{
			let ofs = page*2 - 2040*256;
			let dst_slots = &mut last_tab[ofs ..][.. 2];
			let src_slot_0_val = user_last_table()[ofs].load(Ordering::Relaxed);
			
			let mode_flags = src_slot_0_val & PAGE_MASK_U32;
			match flags_to_prot_mode(mode_flags)
			{
			ProtectionMode::Unmapped => {},
			ProtectionMode::UserCOW => todo!("Clone when COW"),
			ProtectionMode::UserRW | ProtectionMode::UserRO | ProtectionMode::UserRX => {
				let src_ptr = (page * crate::PAGE_SIZE) as *const u8;
				// SAFE: Memory is valid (TODO: What if this changes? Shouldn't cause errors, just inconsistent user data)
				let src = unsafe { ::core::slice::from_raw_parts(src_ptr, crate::PAGE_SIZE) };
				let mut data = crate::memory::phys::allocate_bare()?;
				data.copy_from_slice(src);
				dst_slots[0] = (data.phys_addr() as u32 + 0x0000) | mode_flags;
				dst_slots[1] = (data.phys_addr() as u32 + 0x1000) | mode_flags;
				log_trace!("- Clone @{:p} = {:#x}", src_ptr, data.phys_addr());
				},
			mode @ _ => {
				log_warning!("TODO: Other protection modes: {:?}", mode)
				},
			}
		}

		Ok( AddressSpace( new_root.phys_addr() ) )
	}

	pub fn get_ttbr0(&self) -> u32 { self.0 }
}


// --------------------------------------------------------------------
//
// --------------------------------------------------------------------
#[repr(C)]
pub struct AbortRegs
{
	sp: u32,
	lr: u32,
	gprs: [u32; 13],	// R0-R12
	_unused: u32,	// Padding (actually R0)
	ret_pc: u32,	// SRSFD/RFEFD state
	spsr: u32,
}
#[no_mangle]
pub fn data_abort_handler(pc: u32, reg_state: &AbortRegs, dfar: u32, dfsr: u32) {

	let pg_base = (dfar as usize) & !PAGE_MASK;
	let mut ent = PageEntry::get(pg_base as *const ());
	if ent.mode() == ProtectionMode::UserCOW {
		// 1. Lock (relevant) address space
		// SAFE: Changes to address space are transparent
		crate::memory::virt::with_lock(dfar as usize, || unsafe {
			let frame = ent.phys_addr() & !PAGE_MASK as u32;
			// 2. Get the PMM to provide us with a unique copy of that frame (can return the same addr)
			let newframe = crate::memory::phys::make_unique( frame, &*(pg_base as *const [u8; PAGE_SIZE]) );
			// 3. Remap to this page as UserRW (because COW is user-only atm)
			ent.set(newframe, ProtectionMode::UserRW);
			let pg_base2 = pg_base + 0x1_000;
			PageEntry::get(pg_base2 as *const ()).set(newframe + 0x1_000, ProtectionMode::UserRW);
			tlbimva(pg_base  as *mut _);
			log_debug!("- COW frame copied");
			});
		return ;
	}

	log_warning!("Data abort by {:#x} address {:#x} status {:#x} ({}), LR={:#x}", pc, dfar, dfsr, fsr_name(dfsr), reg_state.lr);
	//log_debug!("Registers:");
	//log_debug!("R 0 {:08x}  R 1 {:08x}  R 2 {:08x}  R 3 {:08x}  R 4 {:08x}  R 5 {:08x}}  R 6 {:08x}", reg_state.gprs[0]);
	dump_tables();
	
	
	if pc < 0x8000_0000 {
		log_error!("User fault - Infinite spin (TODO: Handle)");
		loop {}
	}
	else {
		let rs = crate::arch::imp::aeabi_unwind::UnwindState::from_regs([
			reg_state.gprs[0], reg_state.gprs[1], reg_state.gprs[ 2], reg_state.gprs[3],
			reg_state.gprs[4], reg_state.gprs[5], reg_state.gprs[ 6], reg_state.gprs[7],
			reg_state.gprs[8], reg_state.gprs[9], reg_state.gprs[10], reg_state.gprs[11],
			reg_state.gprs[12], reg_state.sp, reg_state.lr, reg_state.ret_pc,
			]);
		crate::arch::imp::print_backtrace_unwindstate(rs, pc as usize);
		panic!("Kernel data abort by {:#x} at {:#x}", pc, dfar);
	}
}
fn fsr_name(ifsr: u32) -> &'static str {
	match ifsr & 0x40F
	{
	0x001 => "Alignment fault",
	0x004 => "Instruction cache maintainence",
	0x00C => "Sync Ext abort walk lvl1",
	0x00E => "Sync Ext abort walk lvl2",
	0x40C => "Sync Ext pairity walk lvl1",
	0x40E => "Sync Ext pairity walk lvl2",
	0x005 => "Translation fault lvl1",
	0x007 => "Translation fault lvl2",
	0x003 => "Access flag fault lvl1",
	0x006 => "Access flag fault lvl2",
	0x009 => "Domain fault lvl1",
	0x00B => "Domain fault lvl2",
	0x00D => "Permissions fault lvl1",
	0x00F => "Permissions fault lvl2",
	0x002 => "Debug event",
	0x008 => "Synchronous external abort",
	_ => "undefined",
	}
}
#[no_mangle]
pub fn prefetch_abort_handler(pc: u32, reg_state: &AbortRegs, ifsr: u32) {
	log_warning!("Prefetch abort at {:#x} status {:#x} ({}) - LR={:#x}, SP={:#x}", pc, ifsr, fsr_name(ifsr), reg_state.lr, reg_state.sp);
	dump_tables();
	let ent = PageEntry::get(pc as usize as *const ());
	//log_debug!("- {:?}", ent);
	{
		let mode = ent.mode();
		log_debug!("- {:#x} {:?}", ent.phys_addr(), mode);
	}
	//log_debug!("Registers:");
	//log_debug!("R 0 {:08x}  R 1 {:08x}  R 2 {:08x}  R 3 {:08x}  R 4 {:08x}  R 5 {:08x}}  R 6 {:08x}", reg_state.gprs[0]);
	
	let rs = crate::arch::imp::aeabi_unwind::UnwindState::from_regs([
		reg_state.gprs[ 0], reg_state.gprs[1], reg_state.gprs[ 2], reg_state.gprs[ 3],
		reg_state.gprs[ 4], reg_state.gprs[5], reg_state.gprs[ 6], reg_state.gprs[ 7],
		reg_state.gprs[ 8], reg_state.gprs[9], reg_state.gprs[10], reg_state.gprs[11],
		reg_state.gprs[12], reg_state.sp, reg_state.lr, reg_state.ret_pc,
		]);
	crate::arch::imp::print_backtrace_unwindstate(rs, pc as usize);
	loop {}
}
#[no_mangle]
pub fn ud_abort_handler(pc: u32, reg_state: &AbortRegs) {
	log_warning!("Undefined instruction abort at {:#x} - LR={:#x}", pc, reg_state.lr);
		
	let rs = crate::arch::imp::aeabi_unwind::UnwindState::from_regs([
		reg_state.gprs[ 0], reg_state.gprs[1], reg_state.gprs[ 2], reg_state.gprs[ 3],
		reg_state.gprs[ 4], reg_state.gprs[5], reg_state.gprs[ 6], reg_state.gprs[ 7],
		reg_state.gprs[ 8], reg_state.gprs[9], reg_state.gprs[10], reg_state.gprs[11],
		reg_state.gprs[12], reg_state.sp, reg_state.lr, reg_state.ret_pc,
		]);
	crate::arch::imp::print_backtrace_unwindstate(rs, pc as usize);
	loop {}
}
