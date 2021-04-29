//
//
//
use lib::Vec;
use sync::Spinlock;
use lib::LazyStatic;

pub type BindError = ();

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

pub fn init() {
	// TODO: Interrogate the FDT to discover the IRQ controller layout
	if let Some(fdt) = ::arch::imp::boot::get_fdt()
	{
		for p in fdt.get_props_cb(|idx,leaf,name| match (idx,leaf)
			{
			(0,false) => name == "",
			(1,false) => name == "intc" || name.starts_with("intc@"),
			(2,true) => name == "reg",
			_ => false,
			})
		{
			log_debug!("INTC {:x?}", p);
		}
	}

	S_IRQS.prep(|| Vec::from_fn(32, |_| Default::default()));
}

#[linkage="external"]
#[no_mangle]
pub extern "C" fn interrupt_handler()
{
	let irq = get_active_interrupt();
	if irq as usize >= S_IRQS.len() {
		// ... No idea!
	}
	else {
		match S_IRQS[irq as usize].try_lock_cpu()
		{
		None => {
			// Lock is already held by this CPU, just drop the IRQ
			},
		Some(v) =>
			match *v
			{
			None => {},
			Some(ref v) => (v.handler)( v.info ),
					// TODO: Acknowledge the IRQ on the GIC?
			},
		}
	}
}

fn get_active_interrupt() -> u32
{
	todo!("get_active_interrupt - Query the active interrupt from the GIC");
}

pub fn bind_gsi(gsi: usize, handler: fn(*const()), info: *const ()) -> Result<IRQHandle,()> {

	if gsi >= S_IRQS.len() {
		Err( () )
	}
	else {
		let mut lh = S_IRQS[gsi].lock();
		if lh.is_some() {
			Err( () )
		}
		else {
			// TODO: Enable this interrupt on the GIC?
			*lh = Some(Binding {
				handler: handler,
				info: info,
				});
			Ok( IRQHandle(gsi as u32) )
		}
	}
}

