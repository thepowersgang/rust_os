// "Tifflin" Kernel
// - By John Hodge (Mutabah/thePowersGang)
//
// Core/arch/armv7/gic.rs
//! ARM GIC (Generic Interrupt Controller) driver
// REFERENCE ARM IHI 0048
// NOTE: This is shared by both armv7 and armv8
use ::core::sync::atomic::{Ordering,AtomicUsize};
use crate::lib::hwreg;

/// Lazy initialised GIC instance (suitable for storage in static memory)
pub struct GicInstance {
	dist: AtomicUsize,
	cpu: AtomicUsize,
}
impl GicInstance
{
	/// Create a new zero-initialised instance
	pub const fn new_uninit() -> GicInstance {
		GicInstance { dist: AtomicUsize::new(0), cpu: AtomicUsize::new(0) }
	}
	/// Initialise an uninitailised instance
	/// - `ah_dist` is an allocation handle to the Distrubutor register block (usually early in the memory map)
	/// - `ah_cpu` is an allocation handle to the per-CPU reigster block
	pub fn init(&self, ah_dist: crate::memory::virt::MmioHandle, ah_cpu: crate::memory::virt::MmioHandle)
	{
		// TODO: Check the allocation size
		// SAFE: Access is currently unique
		let ptr_dist: *mut () = unsafe { ah_dist.as_int_mut(0) };
		// SAFE: Access is currently unique
		let ptr_cpu : *mut () = unsafe { ah_cpu .as_int_mut(0) };
		log_debug!("GIC Dist @ {:p} = {:#x}  CPU @ {:p} = {:#x}",
			ptr_dist, crate::memory::virt::get_phys(ptr_dist),
			ptr_cpu , crate::memory::virt::get_phys(ptr_cpu ),
			);
		match self.dist.compare_exchange(0, ptr_dist as usize, Ordering::SeqCst, Ordering::Relaxed)
		{
		Ok(_) => {
			self.cpu.store(ptr_cpu as usize, Ordering::SeqCst);

			self.reg_cpu(GICC_CTLR).store(0x00);	// Disable interface
			self.reg_cpu(GICC_PMR ).store(0xFF);	// Enable all priorities
			self.reg_cpu(GICC_CTLR).store(0x01);	// Enable interface
			self.reg_dist(GICD_CTLR).store(1);	// Enable distributor

			::core::mem::forget(ah_dist);
			::core::mem::forget(ah_cpu);
			},
		Err(other) => {
			log_error!("Multiple GICs registered? - {:#x} and {:#x}", other, ptr_dist as usize);
			}
		}
	}
	pub fn is_init(&self)->bool {
		// Check the CPU as it's initialised second (avoids races)
		self.cpu.load(Ordering::SeqCst) != 0
	}

	/// Loop running claim+complete until zero is returned
	pub fn get_pending_interrupts(&self, mut cb: impl FnMut(usize)) {
		loop
		{
			let v = self.reg_cpu(GICC_IAR).load();
			if v == 1023 {
				break;
			}
			cb(v as usize);
			self.reg_cpu(GICC_EOIR).store(v);
		}
	}

	/// Enable/disable the specified interrupt
	pub fn set_enable(&self, idx: usize, enable: bool) {
		if enable {
			self.reg_dist(GICD_ISENABLERn(idx/32)).store(1 << (idx%32));
		}
		else {
			self.reg_dist(GICD_ICENABLERn(idx/32)).store(1 << (idx%32));
		}
	}
	/// Modify the mode (level or edge) of the specified interrupt
	pub fn set_mode(&self, idx: usize, mode: Mode) {
		let reg = self.reg_dist(GICD_ICFGRn(idx / 16));
		let ofs = (idx % 16) * 2;
		match mode
		{
		Mode::LevelHi => { reg.fetch_and(!(2 << ofs)); }
		Mode::Rising  => { reg.fetch_or(   2 << ofs ); }
		}
	}
	/// Send a Software Generated Interrupt to this core
	pub fn trigger_sgi_self(&self, id: u8) {
		self.reg_dist(GICD_SGIR).store(0b10 << 24 | ((id & 0xF) as u32));
	}
	/// Send a Software Generated Interrupt to all other cores
	pub fn trigger_sgi_others(&self, id: u8) {
		self.reg_dist(GICD_SGIR).store(0b01 << 24 | ((id & 0xF) as u32));
	}

	fn get_ref_dist<T: crate::lib::POD>(&self, ofs: usize) -> *const T {
		assert!(ofs + ::core::mem::size_of::<T>() < 0x1_0000);
		let base = self.dist.load(Ordering::SeqCst);
		assert!(base != 0, "Using unintialised GicInstance");
		// SAFE: Once non-zero, this pointer is always valid. Range checked above
		(base + ofs) as *const T
	}
	fn get_ref_cpu<T: crate::lib::POD>(&self, ofs: usize) -> *const T {
		assert!(ofs + ::core::mem::size_of::<T>() < 0x1_0000);
		let base = self.cpu.load(Ordering::SeqCst);
		assert!(base != 0, "Using unintialised GicInstance");
		(base + ofs) as *const T
	}

	fn reg_cpu(&self, reg: CpuRegister) -> hwreg::SafeRW<u32>
	{
		// SAFE: No register can cause memory unsafety
		unsafe { hwreg::Reg::from_ptr(self.get_ref_cpu(reg as usize)) }
	}
	fn reg_dist(&self, reg: DistRegister) -> hwreg::SafeRW<u32>
	{
		// SAFE: No register can cause memory unsafety
		unsafe { hwreg::Reg::from_ptr(self.get_ref_dist(reg.0)) }
	}
}

/// Interrupt trigger mode (level or edge)
pub enum Mode {
	LevelHi,
	Rising,
}

#[repr(usize)]
#[allow(non_camel_case_types)]
enum CpuRegister
{
	GICC_CTLR = 0x00,
	GICC_PMR  = 0x04,
	GICC_BPR  = 0x08,
	GICC_IAR  = 0x0C,
	GICC_EOIR = 0x10,
}
use self::CpuRegister::*;
struct DistRegister(usize);
const GICD_CTLR: DistRegister = DistRegister(0x__0);
const GICD_SGIR: DistRegister = DistRegister(0xF00);
//const GICD_TYPER: DistRegister = DistRegister(0x04);
//const GICD_ISPENDR0: DistRegister = DistRegister(0x200);
#[allow(non_snake_case)]
fn GICD_ISENABLERn(n: usize)->DistRegister {
	assert!(n < 32);
	DistRegister(0x100 + n*4)
}
#[allow(non_snake_case)]
fn GICD_ICENABLERn(n: usize)->DistRegister {
	assert!(n < 32);
	DistRegister(0x180 + n*4)
}
#[allow(non_snake_case)]
fn GICD_ICPENDRn(n: usize)->DistRegister {
	assert!(n < 32);
	DistRegister(0x280 + n*4)
}
#[allow(non_snake_case)]
fn GICD_ITARGETSRn(n: usize)->DistRegister {
	assert!(n < 32);
	DistRegister(0x800 + n)
}
#[allow(non_snake_case)]
fn GICD_ICFGRn(n: usize)->DistRegister {
	assert!(n < 0x100/4);
	DistRegister(0xC00 + n*4)
}

