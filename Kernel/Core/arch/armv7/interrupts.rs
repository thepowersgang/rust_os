//
//
//
use crate::lib::Vec;
use crate::sync::Spinlock;
use crate::lib::LazyStatic;
use super::fdt_devices;
use super::gic;

pub type BindError = ();

#[derive(Debug)]
pub struct IRQHandle(u32);
impl Default for IRQHandle {
	fn default() -> IRQHandle { IRQHandle(!0) }
}

struct Binding {
	handler: fn ( *const() ),
	info: *const (),
}
unsafe impl Send for Binding {}

static S_IRQS: LazyStatic<Vec< Spinlock<Option<Binding>> >> = lazystatic_init!();

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
		for (i,slot) in S_IRQS.iter().enumerate()
		{
			if let Some(_) = *slot.lock()
			{
				GIC.set_enable(i, true);
			}
		}
		// Enable interrupts (clear masking)
		// SAFE: We're all good to start them
		unsafe { crate::arch::sync::start_interrupts(); }
		GIC.trigger_sgi_self(0);
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
pub(super) fn init()
{
	S_IRQS.prep(|| Vec::from_fn(1024, |_| Default::default()));
}

static GIC: gic::GicInstance = gic::GicInstance::new_uninit();

#[linkage="external"]
#[no_mangle]
pub extern "C" fn interrupt_handler(regs: &[u32; 4+13+2])
{
	log_debug!("interrupt_handler(PC={:#x}, SPSR={:#x})", regs[4+13], regs[4+14]);
	handle();
}
fn handle()
{
	log_debug!("handle()");
	if !GIC.is_init() {
		log_debug!("IRQ with GIC not initialised?");
		return ;
	}
	GIC.get_pending_interrupts(|irq| {
		if irq >= S_IRQS.len() {
			// ... No idea!
			log_error!("IRQ {} raised, but out of range", irq);
		}
		else {
			//return ;
			match S_IRQS[irq].try_lock_cpu()
			{
			None => {
				// Lock is already held by this CPU, just drop the IRQ
				log_trace!("IRQ{} not handled due to lock collision", irq);
				},
			Some(v) =>
				match *v
				{
				None => log_warning!("IRQ{} fired but not bound", irq),
				Some(ref v) => (v.handler)( v.info ),
				},
			}
		}
	});
}

pub fn bind_gsi(gsi: usize, handler: fn(*const()), info: *const ()) -> Result<IRQHandle,()> {

	if gsi >= S_IRQS.len() {
		log_error!("bind_gsi({}, ...) - Out of range (S_IRQS.len() = {})", gsi, S_IRQS.len());
		Err( () )
	}
	else {
		let mut lh = S_IRQS[gsi].lock();
		if lh.is_some() {
			Err( () )
		}
		else {
			*lh = Some(Binding {
				handler: handler,
				info: info,
				});
			if GIC.is_init() {
				GIC.set_enable(gsi, true);
			}
			Ok( IRQHandle(gsi as u32) )
		}
	}
}

