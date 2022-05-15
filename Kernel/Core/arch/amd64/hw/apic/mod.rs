// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/hw/apic/mod.rs
//! x86 APIC (Advanced Programmable Interrupt Controller) Driver.
// mod.rs -- Core API / init
use crate::prelude::*;

module_define!{APIC, [ACPI], init}

mod raw;
mod init;

pub type IRQHandler = fn(info: *const ());

#[derive(Default)]
pub struct IRQHandle
{
	num: usize,
	isr_handle: crate::arch::amd64::interrupts::ISRHandle,
}

#[derive(Debug,Copy,Clone)]
pub enum IrqError
{
	BadIndex,
	BindFail(crate::arch::amd64::interrupts::BindISRError),
}

//#[link_section="processor_local"]
//static s_lapic_lock: ::sync::Mutex<()> = mutex_init!( () );
#[allow(non_upper_case_globals)]
static s_lapic: crate::lib::LazyStatic<raw::LAPIC> = lazystatic_init!();
#[allow(non_upper_case_globals)]
static s_ioapics: crate::lib::LazyStatic<Vec<raw::IOAPIC>> = lazystatic_init!();

fn init()
{
	let madt = match crate::arch::amd64::acpi::find::<init::ACPI_MADT>("APIC", 0)
		{
		None => {
			log_warning!("No MADT ('APIC') table in ACPI");
			return ;
			},
		Some(v) => v,
		};
	
	madt.data().dump(madt.data_len());
	
	// Handle legacy (8259) PIC
	if (madt.data().flags & 1) != 0 {
		log_notice!("Legacy PIC present, disabling");
		// Disable legacy PIC by masking all interrupts off
		// SAFE: Only code to access the PIC
		unsafe {
			crate::arch::x86_io::outb(0xA1, 0xFF);	// Disable slave
			crate::arch::x86_io::outb(0x21, 0xFF);	// Disable master
		}
	}
	
	// Find the LAPIC address
	let mut lapic_addr = madt.data().local_controller_addr as u64;
	for ent in madt.data().records(madt.data_len()).filter_map(
		|r| match r { init::MADTDevRecord::DevLAPICAddr(x) => Some(x.address), _ => None }
		)
	{
		lapic_addr = ent;
	}
	
	// Create instances of the IOAPIC "driver" for all present controllers
	let ioapics: Vec<_> = madt.data().records(madt.data_len()).filter_map(
			|r| match r {
				init::MADTDevRecord::DevIOAPIC(a) => Some(raw::IOAPIC::new(a.address as u64, a.interrupt_base as usize)),
				_ => None
				}
			).collect();
	
	// Create APIC and IOAPIC instances
	// SAFE: Called in a single-threaded context
	unsafe {
		s_lapic.prep(|| raw::LAPIC::new(lapic_addr));
		s_lapic.ls_unsafe_mut().global_init();

		s_ioapics.prep(|| ioapics);
		};
	s_lapic.init();
	
	// Enable interupts
	// TODO: Does S_IRQS_ENABLED ever get read?
	crate::arch::amd64::threads::S_IRQS_ENABLED.store(true, ::core::sync::atomic::Ordering::Relaxed);
	// SAFE: Just STI, nothing to worry about
	unsafe { ::core::arch::asm!("sti"); }
}

fn get_ioapic(interrupt: usize) -> Option<(&'static raw::IOAPIC, usize)>
{
	match s_ioapics.iter().find( |a| a.contains(interrupt) )
	{
	None => None,
	Some(x) => {
		let ofs = interrupt - x.first();
		Some( (x, ofs) )
		},
	}
}
fn get_lapic() -> &'static raw::LAPIC
{
	&*s_lapic
}

///// Registers a message-signalled interrupt handler.
//pub fn register_msi(callback: fn (*const()), info: *const ()) -> Result<(uint,::arch::interrupts::ISRHandle),()>
//{
//	// 1. Find a spare ISR slot on a processor
//	let lapic_id = 0;
//	let isrnum = 33u;
//	// 2. Bind
//	let h = try!(::arch::interrupts::bind_isr(lapic_id, isrnum as u8, callback, info, cleanup_nop));
//	Ok( (isrnum, h) )
//}

/// Local + IO APIC interrupt handler
//#[req_safe(irq)]
extern "C" fn lapic_irq_handler(isr: usize, info: *const(), gsi: usize)
{
	//log_trace!("lapic_irq_handler: (isr={},info={:?},gsi={})", isr, info, gsi);
	let (ioapic,ofs) = match get_ioapic(gsi) {
		Some(x) => x,
		None => {
			log_error!("lapic_irq_handler - GSI does not correspond to an IOAPIC ({})", gsi);
			return ()
			}
		};
	
	match ioapic.get_callback(ofs)
	{
	Some(cb) => cb(info),
	None => log_notice!("No bound callback for GSI{}", gsi),
	}
	ioapic.eoi( ofs );
	get_lapic().eoi(isr);
}

/// Registers an interrupt
pub fn register_irq(global_num: usize, callback: IRQHandler, info: *const() ) -> Result<IRQHandle,IrqError>
{
	// Locate the relevant apic
	let (ioapic,ofs) = match get_ioapic(global_num) {
		Some(x) => x,
		None => return Err( IrqError::BadIndex ),
		};
	
	// Bind ISR
	// TODO: Pick a suitable processor, and maybe have separate IDTs (and hence separate ISR lists)
	let lapic_id = 0u32;
	let isr_handle = match crate::arch::amd64::interrupts::bind_free_isr(lapic_irq_handler, info, global_num)
		{
		Ok(v) => v,
		Err(e) => return Err(IrqError::BindFail(e)),
		};

	// Enable the relevant IRQ on the LAPIC and IOAPIC
	// - Uses edge triggering so the handler can signal to the downstream
	// - Works (at least with qemu) even if the source is level-triggered
	let mode = raw::TriggerMode::EdgeHi;
	//let mode = raw::TriggerMode::LevelHi;
	ioapic.set_irq(ofs, isr_handle.idx() as u8, lapic_id, mode, callback);
	
	Ok( IRQHandle {
		num: global_num,
		isr_handle,
		} )
}

//impl ::core::fmt::Display for IrqError
//{
//	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
//	{
//		match *self
//		{
//		IrqError::BadIndex => write!(f, "Bad IRQ Number"),
//		IrqError::BindFail(e) => write!(f, "Failed to bind: {}", e),
//		}
//	}
//}

impl IRQHandle
{
	pub fn num(&self) -> u32 { self.num as u32 }
}
impl ::core::fmt::Debug for IRQHandle
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(),::core::fmt::Error>
	{
		let (ioapic,ofs) = get_ioapic(self.num).unwrap();
		write!(f, "IRQHandle{{#{}, LAPIC={:?}, Reg={:#x}}}",
			self.num,
			get_lapic().get_vec_status(self.isr_handle.idx() as u8),
			ioapic.get_irq_reg(ofs)
			)
	}
}

impl ::core::ops::Drop for IRQHandle
{
	fn drop(&mut self)
	{
		let (ioapic,ofs) = get_ioapic(self.num).unwrap();
		ioapic.disable_irq(ofs);
	}
}

// vim: ft=rust

