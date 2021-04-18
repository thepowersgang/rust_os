// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/input_ps2/pl050.rs
//! ARM PL050 PS/2 Controller
use kernel::prelude::*;
use kernel::memory::virt::AllocHandle;
use kernel::sync::mutex::LazyMutex;
use kernel::irqs;

//const PL050_RXREADY: u32 = 0x??;
const PL050_TXBUSY: u32 = 0x20;

struct Port
{
	base: AllocHandle,
	dev: super::PS2Dev,
}

static S_PORTS: LazyMutex< Vec<irqs::ObjectHandle> > = lazymutex_init!();

pub fn init()
{
	let mut lh = S_PORTS.lock_init(|| Default::default());

	if false {
		// SAFE: Assumes the input addresses are sane
		unsafe {
			// Realview PB's keyboard port
			let mut port = Port::new(0x10006000).expect("PB PS/2 #1 binding failed");
			lh.push( irqs::bind_object(20, Box::new(move || port.handle_irq()) ) );
			let mut port = Port::new(0x10007000).expect("PB PS/2 #2 binding failed");
			lh.push( irqs::bind_object(21, Box::new(move || port.handle_irq()) ) );
		}
	}
}

impl Port
{
	#[inline(never)]
	unsafe fn new(addr: ::kernel::memory::PAddr) -> Result<Port, ::kernel::memory::virt::MapError> {
		let p = Port {
			base: ::kernel::memory::virt::map_hw_rw(addr, 1, "PL050")?,
			dev: super::PS2Dev::None,
			};
		log_debug!("p.base = {:?}", p.base);

		// TODO: Unknown what this does, Acess did it.
		//  According to the qemu source, this enables interrupts
		p.get_regs()[0] = 0x10u32;

		log_debug!("p.base = {:?}", p.base);

		Ok(p)
	}

	unsafe fn get_regs(&self) -> &mut [u32] {
		self.base.as_int_mut_slice(0, 4)
	}
	unsafe fn recv_byte(&self) -> u8 {
		let regs = self.get_regs();
		(regs[2] & 0xFF) as u8
	}
	unsafe fn send_byte(&self, byte: u8) {
		let regs = self.get_regs();

		assert!( regs[1] & PL050_TXBUSY != 0 );
		regs[2] = byte as u32;
	}

	fn handle_irq(&mut self) -> bool {

		// SAFE: Should never race, and any race will be benign
		unsafe {
			let b = self.recv_byte();
			if let Some(ob) = self.dev.recv_byte(b) {
				self.send_byte(ob);
			}
		}

		true
	}
}

