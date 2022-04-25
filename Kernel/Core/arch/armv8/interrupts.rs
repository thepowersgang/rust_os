use ::core::sync::atomic::{AtomicUsize,Ordering};
use super::fdt_devices;
use super::gic;

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
impl IRQHandle {
	pub fn num(&self) -> usize {
		self.0 - 1
	}
}
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
		let mut ah_iter = reg.iter_paddr().map(|r| unsafe { let (base,size) = r.expect("GIC MMIO out of PAddr range"); crate::memory::virt::map_mmio(base, size).expect("GIC MMIO map failed") });
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

pub(super) fn handle()
{
	if GIC.is_init()
	{
		GIC.get_pending_interrupts(|idx| {
			let slot = &INTERRUPT_HANDLES[idx];
			let info = slot.1.load(Ordering::SeqCst);
			let cb = slot.0.load(Ordering::SeqCst);
			log_debug!("IRQ{}: {:#x} {:#x}", idx, cb, info);
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
