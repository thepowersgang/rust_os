use ::core::sync::atomic::{AtomicUsize,Ordering};
use super::fdt_devices;

macro_rules! array {
	(@1024 $($e:tt)*) => { array!(@512 $($e)*, $($e)*) };
	(@512 $($e:tt)*) => { array!(@256 $($e)*, $($e)*) };
	(@256 $($e:tt)*) => { array!(@128 $($e)*, $($e)*) };
	(@128 $($e:tt)*) => { array!(@ 64 $($e)*, $($e)*) };
	(@ 64 $($e:tt)*) => { array!(@ 32 $($e)*, $($e)*) };
	(@ 32 $($e:tt)*) => { array!(@ 16 $($e)*, $($e)*) };
	(@ 16 $($e:tt)*) => { array!(@  8 $($e)*, $($e)*) };
	(@  8 $($e:tt)*) => { array!(@  4 $($e)*, $($e)*) };
	(@  4 $($e:tt)*) => { array!(@  2 $($e)*, $($e)*) };
	(@  2 $($e:tt)*) => { [ $($e)*, $($e)* ] };
}
static INTERRUPT_HANDLES: [ (AtomicUsize, AtomicUsize); 1024 ] = array!(@1024 (AtomicUsize::new(0), AtomicUsize::new(0)) );

#[derive(Default)]
pub struct IRQHandle(usize);
#[derive(Debug)]
pub struct BindError;

pub fn bind_gsi(gsi: usize, handler: fn(*const ()), info: *const ()) -> Result<IRQHandle,BindError> {
	let slot = &INTERRUPT_HANDLES[gsi];
	match slot.0.compare_exchange(0, 1, Ordering::SeqCst, Ordering::Relaxed)
	{
	Ok(_) => {
		log_debug!("bind_gsi({}) = {:p} {:p}", gsi, handler, info);
		slot.1.store(info as usize, Ordering::Relaxed);
		slot.0.store(handler as usize, Ordering::Relaxed);
		if GIC.is_init() {
			GIC.set_enable(gsi, true);
		}
		Ok( IRQHandle(gsi+1) )
		},
	Err(existing) => {
		log_warning!("bind_gsi({}) = CONFLICT {:#x}", gsi, existing);
		Err(BindError)
		},
	}
}

impl ::core::ops::Drop for IRQHandle {
	fn drop(&mut self)
	{
		if self.0 > 0
		{
			let gsi = self.0 - 1;
			let slot = &INTERRUPT_HANDLES[gsi];
			assert!( slot.0.swap(0, Ordering::SeqCst) > 1, "Unbinding IRQ handle that is already empty - gsi={}", gsi);
			if GIC.is_init() {
				GIC.set_enable(gsi, false);
			}
		}
	}
}

pub(super) fn get_intc(compat: fdt_devices::Compat, reg: fdt_devices::Reg) -> Option<&'static dyn fdt_devices::IntController>
{
	if compat.matches_any(&[ "arm,cortex-a15-gic" ])
	{
		if GIC.is_init() {
			log_error!("Two GIC instances?");
			return None;
		}

		// SAFE: Trusting the FDT
		let mut ah_iter = reg.iter_paddr().map(|r| unsafe { let (base,size) = r.expect("GIC MMIO out of PAddr range"); ::memory::virt::map_mmio(base, size).expect("GIC MMIO map failed") });
		let ah_dist = ah_iter.next().expect("GIC missing distributor range in FDT?");
		let ah_cpu  = ah_iter.next().expect("GIC missing CPU range in FDT?");

		GIC.init(ah_dist, ah_cpu);

		// Enable all interrupts
		for (i,slot) in INTERRUPT_HANDLES.iter().enumerate()
		{
			let cb = slot.0.load(Ordering::SeqCst);
			// If >1 it's already been initialised (if 1, then it's currently being initialised - so will be enabled by `bind_gsi`
			if cb > 1
			{
				GIC.set_enable(i, true);
			}
		}
		// Enable interrupts (clear masking)
		// SAFE: We're all good to start them
		unsafe { crate::arch::sync::start_interrupts(); }
		// Handle any pending ones
		self::handle();

		// Return a stub instance
		struct Instance;
		impl fdt_devices::IntController for Instance {
			fn get_gsi(&self, mut cells: fdt_devices::Cells) -> Option<u32> {
				let ty = cells.read_1()?;
				let num = cells.read_1()?;
				let flags = cells.read_1()?;
				match ty
				{
				0 => {
					if num >= 1024-32 {
						log_error!("SPI index out of range {} >= 1024-32", num);
						return None;
					}
					// "SPI" interrupts are offset by 32 entries
					let num = num + 32;
					GIC.set_mode(num as usize, match flags & 0xF
						{
						1 /*GIC_FDT_IRQ_FLAGS_EDGE_LO_HI*/ => gic::Mode::Rising,
						4 /*GIC_FDT_IRQ_FLAGS_LEVEL_HI*/ => gic::Mode::LevelHi,
						_ => {
							log_error!("TODO: Unuspported interrupt flags - {:#x}", flags);
							return None;
							}
						});
					Some( num )
					},
				1 => {
					if num >= 16 {
						log_error!("PPI index out of range {} >= 16", num);
						return None;
					}
					let num = 16 + num;
					log_notice!("TODO: Support PPI interrupts (#{})", num);
					None
					}
				_ => {
					log_error!("Support interrupt types other than `SPI` - {}", ty);
					None
					}
				}
			}
		}
		Some(&Instance)
	}
	else
	{
		None
	}
}

use self::gic::GicInstance;
static GIC: GicInstance = GicInstance::new_uninit();

fn handle()
{
	if GIC.is_init()
	{
		GIC.get_pending_interrupts(|idx| {
			let slot = &INTERRUPT_HANDLES[idx];
			log_debug!("IRQ{}", idx);
			let info = slot.1.load(Ordering::SeqCst);
			let cb = slot.0.load(Ordering::SeqCst);
			if cb > 1 {
				// SAFE: Correct type, pointer set by `bind_gsi` above
				let cb: fn(*const ()) = unsafe { ::core::mem::transmute(cb) };
				cb(info as *const ());
			}
			else if cb == 1 {
			}
			else {
			}
			});
	}
}


mod gic {
	use ::core::sync::atomic::{Ordering,AtomicUsize,AtomicU32};
	pub struct GicInstance {
		dist: AtomicUsize,
		cpu: AtomicUsize,
	}
	impl GicInstance
	{
		pub const fn new_uninit() -> GicInstance {
			GicInstance { dist: AtomicUsize::new(0), cpu: AtomicUsize::new(0) }
		}
		pub fn init(&self, ah_dist: crate::memory::virt::MmioHandle, ah_cpu: crate::memory::virt::MmioHandle) {
			// TODO: Check the allocation size
			// SAFE: Access is currently unique
			let ptr_dist: *mut () = unsafe { ah_dist.as_int_mut(0) };
			// SAFE: Access is currently unique
			let ptr_cpu : *mut () = unsafe { ah_cpu .as_int_mut(0) };
			log_debug!("GIC Dist @ {:p} = {:#x}  CPU @ {:p} = {:#x}",
				ptr_dist, ::memory::virt::get_phys(ptr_dist),
				ptr_cpu , ::memory::virt::get_phys(ptr_cpu ),
				);
			match self.dist.compare_exchange(0, ptr_dist as usize, Ordering::SeqCst, Ordering::Relaxed)
			{
			Ok(_) => {
				self.cpu.store(ptr_cpu as usize, Ordering::SeqCst);

				self.reg_cpu(GICC_CTLR).store(0, Ordering::Relaxed);	// Disable interface
				self.reg_cpu(GICC_PMR).store(0xFF, Ordering::Relaxed);	// Enable all priorities
				self.reg_cpu(GICC_CTLR).store(1, Ordering::Relaxed);	// Enable interface
				self.reg_dist(GICD_CTLR).store(1, Ordering::Relaxed);	// Enable distributor

				::core::mem::forget(ah_dist);
				::core::mem::forget(ah_cpu);
				},
			Err(other) => {
				log_error!("Multiple GICs registered? - {:#x} and {:#x}", other, ptr_dist as usize);
				}
			}
		}
		pub fn is_init(&self)->bool {
			self.cpu.load(Ordering::SeqCst) != 0
		}

		/// Loop running claim+complete until zero is returned
		pub fn get_pending_interrupts(&self, mut cb: impl FnMut(usize)) {
			loop
			{
				let v = self.reg_cpu(GICC_IAR).load(Ordering::Relaxed);
				log_trace!("v = {}", v);
				if v == 1023 {
					break;
				}
				cb(v as usize);
				self.reg_cpu(GICC_EOIR).store(v, Ordering::Relaxed);
			}
		}

		pub fn set_enable(&self, idx: usize, enable: bool) {
			if enable {
				self.reg_dist(GICD_ISENABLERn(idx/32)).store(1 << (idx%32), Ordering::Relaxed);
			}
			else {
				self.reg_dist(GICD_ICENABLERn(idx/32)).store(1 << (idx%32), Ordering::Relaxed);
			}
		}
		pub fn set_mode(&self, idx: usize, mode: Mode) {
			let reg = self.reg_dist(GICD_ICFGRn(idx / 16));
			let ofs = (idx % 16) * 2;
			match mode
			{
			Mode::LevelHi => { reg.fetch_and(!(2 << ofs), Ordering::Relaxed); }
			Mode::Rising  => { reg.fetch_or(2 << ofs, Ordering::Relaxed); }
			}
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

		// NOTE: None of these can cause memory unsafety
		fn reg_cpu(&self, reg: CpuRegister) -> &AtomicU32
		{
			// SAFE: No register can cause memory unsafety
			unsafe { &*self.get_ref_cpu(reg as usize) }
		}
		fn reg_dist(&self, reg: DistRegister) -> &AtomicU32
		{
			// SAFE: No register can cause memory unsafety
			unsafe { &*self.get_ref_dist(reg.0) }
		}
	}

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
	const GICD_CTLR: DistRegister = DistRegister(0x00);
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
}
