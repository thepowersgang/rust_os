//
//
//

use memory::virt::ProtectionMode;
use arch::memory::PAddr;
use arch::memory::{PAGE_SIZE, PAGE_MASK};
use arch::memory::virt::TempHandle;
use core::sync::atomic::Ordering;

const KERNEL_TEMP_BASE : usize = 0xFFC00000;
const KERNEL_TEMP_COUNT: usize = 1023/2;	// 1024 4KB entries, PAGE_SIZE is 8KB, final 4KB is left zero for the -1 deref mapping
const USER_BASE_TABLE: usize = 0x7FFF_E000;	// 2GB - 0x800 * 4 = 0x2000
const USER_TEMP_TABLE: usize = 0x7FFF_D000;	// Previous page to that
const USER_TEMP_BASE: usize = 0x7FC0_0000;	// 1 page worth of temp mappings (top three are used for base table/temp table)

const PAGE_MASK_U32: u32 = PAGE_MASK as u32;

// TODO: Why is this -1 here?
static S_TEMP_MAP_SEMAPHORE: ::sync::Semaphore = ::sync::Semaphore::new(KERNEL_TEMP_COUNT as isize - 1, KERNEL_TEMP_COUNT as isize);

pub fn post_init() {
	kernel_table0[0].store(0, Ordering::SeqCst);

	//dump_tables();
}

fn dump_tables() {
	let mut start = 0;
	let mut exp = get_phys_opt(0 as *const u8);
	for page in 1 .. 0xFFFF_F {
		let addr = page * 0x1000;
		let v = get_phys_opt( addr as *const u8 );
		if v != exp {
			print(start, addr-1, exp);
			start = addr;
			exp = v;
		}
		if let Some(ref mut v) = exp {
			*v += 0x1000;
		}
	}
	print(start, !0, exp);

	fn print(start: usize, end: usize, exp_val: Option<u32>) {
		match exp_val
		{
		None    => log_trace!("{:08x}-{:08x} --", start, end),
		Some(e) => {
			let (paddr, flags) = (e & !0xFFF, e & 0xFFF);
			log_trace!("{:08x}-{:08x} = {:08x}-{:08x} {:03x}", start, end, paddr as usize - (end-start+1), paddr-1, flags);
			},
		}
	}
}

fn prot_mode_to_flags(mode: ProtectionMode) -> u32 {
	//AP[2] = 9, AP[1:0] = 5:4, XN=0, SmallPage=1
	match mode
	{
	ProtectionMode::Unmapped => 0x000,
	ProtectionMode::KernelRO => 0x213,
	ProtectionMode::KernelRW => 0x013,
	ProtectionMode::KernelRX => 0x052,
	ProtectionMode::UserRO => 0x233,	// 1,11,1
	ProtectionMode::UserRW => 0x033,	// 0,11,1
	ProtectionMode::UserRX => 0x232,	// 1,11,0
	ProtectionMode::UserRWX => 0x032,	// 0,11,0
	ProtectionMode::UserCOW => 0x223,	// 1,10,1 is a deprecated encoding for ReadOnly, need to find a better encoding
	}
}
fn flags_to_prot_mode(flags: u32) -> ProtectionMode {
	match flags
	{
	0x000 => ProtectionMode::Unmapped,
	0x212 => ProtectionMode::KernelRO,
	0x012 => ProtectionMode::KernelRW,
	0x053 => ProtectionMode::KernelRX,
	0x232 => ProtectionMode::UserRO,
	0x032 => ProtectionMode::UserRW,
	0x233 => ProtectionMode::UserRX,
	0x033 => ProtectionMode::UserRWX,
	0x223 => ProtectionMode::UserCOW,
	v @ _ => todo!("Unknown mode value {:#x}", v),
	}
}

/// Atomic 32-bit integer, used for table entries
type AtomicU32 = ::sync::atomic::AtomicValue<u32>;

pub fn is_fixed_alloc<T>(addr: *const T, size: usize) -> bool {
	const BASE: usize = super::addresses::KERNEL_BASE;
	const ONEMEG: usize = 1024*1024;
	const LIMIT: usize = super::addresses::KERNEL_BASE + 4*ONEMEG;
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
		// SAFE: Atomic and valid
		&PageEntryRegion::NonGlobal => unsafe {
			assert!(idx < 2048);
			let table = &*(USER_BASE_TABLE as *const [AtomicU32; 0x800]);
			assert!( get_phys(table) != 0 );
			&table[idx]
			},
		&PageEntryRegion::Global => {
			assert!(idx < 4096);
			&kernel_table0[idx]
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

extern "C" {
	static kernel_table0: [AtomicU32; 0x800*2];
	static kernel_exception_map: [AtomicU32; 1024];
}

/// Returns the physical address of the table controlling `vaddr`. If `alloc` is true, a new table will be allocated if needed.
fn get_table_addr<T>(vaddr: *const T, alloc: bool) -> Option< (::arch::memory::PAddr, usize) > {
	let addr = vaddr as usize & !PAGE_MASK;
	let page = addr >> 12;	// NOTE: 12 as each entry in the table services 4KB
	let (ttbr_ofs, tab_idx) = (page >> 11, page & 0x7FF);
	const ENTS_PER_ALLOC: usize = PAGE_SIZE / 0x400;	// Each entry in the top-level table points to a 1KB second-level table
	const USER_BASE_TABLE_PTR: *const [AtomicU32; 0x800] = USER_BASE_TABLE as *const [AtomicU32; 0x800];
	
	let ent_r = if ttbr_ofs < 0x800/ENTS_PER_ALLOC {
			// SAFE: This memory should always be mapped
			unsafe { & (*USER_BASE_TABLE_PTR)[ttbr_ofs*ENTS_PER_ALLOC .. ][..ENTS_PER_ALLOC] }
		}
		else {
			// Kernel
			&kernel_table0[ ttbr_ofs*ENTS_PER_ALLOC .. ][..ENTS_PER_ALLOC]
		};
	
	let ent_v = ent_r[0].load(Ordering::SeqCst);
	match ent_v & PAGE_MASK_U32
	{
	0 => if alloc {
			let handle: TempHandle<AtomicU32> = match ::memory::phys::allocate_bare()
				{
				Ok(v) => v.into(),
				Err(e) => todo!("get_table_addr - alloc failed")
				};
			//::memory::virt::with_temp(|frame: &[AtomicU32]| for v in frame.iter { v.store(0) });
			for v in handle.iter() {
				v.store(0, Ordering::SeqCst);
			}
			let frame = handle.phys_addr();
			let ent_v = ent_r[0].compare_and_swap(0, frame + 0x1, Ordering::Acquire);
			if ent_v != 0 {
				::memory::phys::deref_frame(frame);
				Some( (ent_v & !PAGE_MASK_U32, tab_idx) )
			}
			else {
				for i in 1 .. ENTS_PER_ALLOC as u32 {
					assert!( ent_r[i as usize].load(Ordering::Relaxed) == 0 );
					ent_r[i as usize].store(frame + i*0x400 + 0x1, Ordering::SeqCst);
				}
				Some( (frame & !PAGE_MASK_U32, tab_idx) )
			}
		}
		else {
			None
		},
	1 => Some( (ent_v & !PAGE_MASK_U32, tab_idx) ),
	0x402 => panic!("Called get_table_addr on large mapping @ {:p} (idx={:#x})", vaddr, ttbr_ofs*ENTS_PER_ALLOC),
	v @ _ => todo!("get_table_addr - Other flags bits {:#x} (for addr {:p})", v, vaddr),
	}
}


/// Temporarily map a frame into memory
/// UNSAFE: User to ensure that the passed address doesn't alias
pub unsafe fn temp_map<T>(phys: ::arch::memory::PAddr) -> *mut T {
	log_trace!("TempHandle<{}>::new({:#x})", type_name!(T), phys);
	assert!(phys as u32 % PAGE_SIZE as u32 == 0);
	let val = (phys as u32) + 0x13;	
	
	S_TEMP_MAP_SEMAPHORE.acquire();
	for i in 0 .. KERNEL_TEMP_COUNT {
		let ents = &kernel_exception_map[i*2 ..][.. 2];
		if ents[0].compare_and_swap(0, val, Ordering::Acquire) == 0 {
			// - Set the next node and check that it's zero
			assert!( ents[1].swap(val+0x1000, Ordering::Acquire) == 0 );
			let addr = (KERNEL_TEMP_BASE + i * ::PAGE_SIZE) as *mut _;
			tlbimva(addr as *mut ());
			//log_trace!("- Addr = {:p}", addr);
			return addr;
		}
	}
	panic!("No free temp mappings");
}
// UNSAFE: Can cause use-after-free if address is invalid
pub unsafe fn temp_unmap<T>(addr: *mut T)
{
	assert!(addr as usize >= KERNEL_TEMP_BASE);
	let i = (addr as usize - KERNEL_TEMP_BASE) / ::PAGE_SIZE;
	assert!(i < KERNEL_TEMP_COUNT);
	kernel_exception_map[i+0].store(0, Ordering::Release);
	kernel_exception_map[i+1].store(0, Ordering::Release);
	S_TEMP_MAP_SEMAPHORE.release();
}

pub fn is_reserved<T>(addr: *const T) -> bool {
	get_phys_opt(addr).is_some()
	//PageEntry::get(addr as *const ()).is_reserved()
}
pub fn get_phys<T>(addr: *const T) -> ::arch::memory::PAddr {
	get_phys_opt(addr).unwrap_or(0)
	//PageEntry::get(addr as *const ()).phys_addr()
}
fn get_phys_opt<T>(addr: *const T) -> Option<::arch::memory::PAddr> {
	let res: u32;
	// SAFE: Correct register accesses
	unsafe {
		// TODO: Disable interrupts during this operation
		asm!("
			mcr p15,0, $1, c7,c8,0;
			isb;
			mrc p15,0, $0, c7,c4,0
			"
			: "=r" (res) : "r" (addr)
			);
	};

	match res & 3 {
	1 | 3 => None,
	0 => Some( ((res as usize & !PAGE_MASK) | (addr as usize & PAGE_MASK)) as u32 ),
	2 => {
		todo!("Unexpected supersection at {:p}, res={:#x}", addr, res);
		let pa_base: u64 = (res & !0xFFFFFF) as u64 | ((res as u64 & 0xFF0000) << (32-16));
		Some( pa_base as u32 | (addr as usize as u32 & 0xFFFFFF) )
		},
	_ => unreachable!(),
	}
}

pub fn get_info<T>(addr: *const T) -> Option<(::arch::memory::PAddr, ::memory::virt::ProtectionMode)> {
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
		asm!("mcr p15,0, $0, c8,c7,1 ; dsb ; isb" : : "r" ( (a as usize & !PAGE_MASK) | 1 ) : "memory" : "volatile")
	}
}
///// Data Cache Clean by Modified Virtual Address (to PoC)
//fn dccmvac(a: *mut ()) {
//}

pub fn can_map_without_alloc(a: *mut ()) -> bool {
	get_table_addr(a, false).is_some()
}

pub unsafe fn map(a: *mut (), p: PAddr, mode: ProtectionMode) {
	log_debug!("map({:p} = {:#x}, {:?})", a, p, mode);
	return map_int(a,p,mode);
	
	// "Safe" helper to constrain interior unsafety
	fn map_int(a: *mut (), p: PAddr, mode: ProtectionMode) {
		// 1. Map the relevant table in the temp area
		let (tab_phys, idx) = get_table_addr(a, true).unwrap();
		//log_debug!("map_int({:p}, {:#x}, {:?}) - tab_phys={:#x},idx={}", a, p, mode, tab_phys, idx);
		// SAFE: Address space is valid during manipulation, and alias is benign
		let mh: TempHandle<AtomicU32> = unsafe {  TempHandle::new( tab_phys ) };
		assert!(mode != ProtectionMode::Unmapped, "Invalid pass of ProtectionMode::Unmapped to map");
		// 2. Insert
		let mode_flags = prot_mode_to_flags(mode);
		let old = mh[idx+0].compare_and_swap(0, p + mode_flags, Ordering::SeqCst);
		assert!(old == 0, "map() called over existing allocation: a={:p}, old={:#x}", a, old);
		mh[idx+1].swap(p + 0x1000 + mode_flags, Ordering::SeqCst);
		tlbimva(a);
		tlbimva( (a as usize + 0x1000) as *mut () );
	}
}
pub unsafe fn reprotect(a: *mut (), mode: ProtectionMode) {
	return reprotect_int(a, mode);

	fn reprotect_int(a: *mut (), mode: ProtectionMode) {
		// 1. Map the relevant table in the temp area
		let (tab_phys, idx) = get_table_addr(a, true).unwrap();
		// SAFE: Address space is valid during manipulation, and alias is benign
		let mh: TempHandle<AtomicU32> = unsafe { TempHandle::new( tab_phys ) };
		assert!(mode != ProtectionMode::Unmapped, "Invalid pass of ProtectionMode::Unmapped to map");
		// 2. Insert
		let mode_flags = prot_mode_to_flags(mode);
		let v = mh[idx].load(Ordering::Acquire);
		assert!(v != 0, "reprotect() called on an unmapped location: a={:p}", a);
		let p = v & !PAGE_MASK_U32;
		//log_debug!("reprotect(): a={:p} mh={:p} idx={}, new={:#x}", a, &mh[0], idx, (v & !PAGE_MASK_U32) + mode_flags);
		let old = mh[idx].compare_and_swap(v, p + mode_flags, Ordering::Release);
		assert!(old == v, "reprotect() called in a racy manner: a={:p} old({:#x}) != v({:#x})", a, old, v);
		mh[idx+1].swap(p + 0x1000 + mode_flags, Ordering::SeqCst);
		tlbimva(a);
		tlbimva( (a as usize + 0x1000) as *mut () );
	}
}
pub unsafe fn unmap(a: *mut ()) -> Option<PAddr> {
	log_debug!("unmap({:p})", a);
	return unmap_int(a);

	fn unmap_int(a: *mut()) -> Option<PAddr> {
		// 1. Map the relevant table in the temp area
		let (tab_phys, idx) = get_table_addr(a, true).unwrap();
		// SAFE: Address space is valid during manipulation, and alias is benign
		let mh: TempHandle<AtomicU32> = unsafe { TempHandle::new( tab_phys ) };
		let old = mh[idx+0].swap(0, Ordering::SeqCst);
		mh[idx+1].swap(0, Ordering::SeqCst);
		tlbimva(a);
		tlbimva( (a as usize + 0x1000) as *mut () );
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
			static kernel_table0: ::Void;
			static kernel_phys_start: u32;
		}
		let tab0_addr = kernel_phys_start + (&kernel_table0 as *const _ as usize as u32 - 0x80000000);
		AddressSpace( tab0_addr )
	}
	pub fn new(clone_start: usize, clone_end: usize) -> Result<AddressSpace,::memory::virt::MapError> {
		// 1. Allocate a new root-level table for the user code (requires two pages aligned, or just use 1GB for user)
		todo!("AddressSpace::new({:#x} -- {:#x})", clone_start, clone_end);
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

	log_warning!("Data abort by {:#x} address {:#x} status {:#x} ({})", pc, dfar, dfsr, fsr_name(dfsr));
	dump_tables();
	//log_debug!("Registers:");
	//log_debug!("R 0 {:08x}  R 1 {:08x}  R 2 {:08x}  R 3 {:08x}  R 4 {:08x}  R 5 {:08x}}  R 6 {:08x}", reg_state.gprs[0]);
	
	let mut ent = PageEntry::get(dfar as usize as *const ());
	if ent.mode() == ProtectionMode::UserCOW {
		// 1. Lock (relevant) address space
		// SAFE: Changes to address space are transparent
		::memory::virt::with_lock(dfar as usize, || unsafe {
			let frame = ent.phys_addr();
			// 2. Get the PMM to provide us with a unique copy of that frame (can return the same addr)
			let newframe = ::memory::phys::make_unique( frame, &*(((dfar as usize) & !PAGE_MASK) as *const [u8; PAGE_SIZE]) );
			// 3. Remap to this page as UserRW (because COW is user-only atm)
			ent.set(newframe, ProtectionMode::UserRW);
			log_debug!("- COW frame copied");
			});

		return ;
	}
	
	if pc < 0x8000_0000 {
		loop {}
	}
	else {
		let rs = ::arch::imp::aeabi_unwind::UnwindState::from_regs([
			reg_state.gprs[0], reg_state.gprs[1], reg_state.gprs[ 2], reg_state.gprs[3],
			reg_state.gprs[4], reg_state.gprs[5], reg_state.gprs[ 6], reg_state.gprs[7],
			reg_state.gprs[8], reg_state.gprs[9], reg_state.gprs[10], reg_state.gprs[11],
			reg_state.gprs[12], reg_state.sp, reg_state.lr, reg_state.ret_pc,
			]);
		::arch::imp::print_backtrace_unwindstate(rs, pc as usize);
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
	_ => "undefined",
	}
}
#[no_mangle]
pub fn prefetch_abort_handler(pc: u32, reg_state: &AbortRegs, ifsr: u32) {
	log_warning!("Prefetch abort at {:#x} status {:#x} ({}) - LR={:#x}", pc, ifsr, fsr_name(ifsr), reg_state.lr);
	//log_debug!("Registers:");
	//log_debug!("R 0 {:08x}  R 1 {:08x}  R 2 {:08x}  R 3 {:08x}  R 4 {:08x}  R 5 {:08x}}  R 6 {:08x}", reg_state.gprs[0]);
	
	let rs = ::arch::imp::aeabi_unwind::UnwindState::from_regs([
		reg_state.gprs[0], reg_state.gprs[1], reg_state.gprs[ 2], reg_state.gprs[3],
		reg_state.gprs[4], reg_state.gprs[5], reg_state.gprs[ 6], reg_state.gprs[7],
		reg_state.gprs[8], reg_state.gprs[9], reg_state.gprs[10], reg_state.gprs[11],
		reg_state.gprs[12], reg_state.sp, reg_state.lr, reg_state.ret_pc,
		]);
	::arch::imp::print_backtrace_unwindstate(rs, pc as usize);
	loop {}
}
