//
//
//
//! AHCI Controller root
use kernel::prelude::*;
use kernel::device_manager;
use kernel::lib::mem::aref::ArefInner;
use hw;

use port::{Port, PortRegs};

/// ACHI Controller
pub struct Controller
{
	inner: ArefInner<ControllerInner>,
	ports: Vec<Port>,
	#[allow(dead_code)]
	irq_handle: Option<::kernel::irqs::ObjectHandle>,
}
pub struct ControllerInner
{
	pub io_base: device_manager::IOBinding,
	pub max_commands: u8,
	pub supports_64bit: bool,
}

impl Controller
{
	pub fn new(irq: u32, io: device_manager::IOBinding) -> Result<Box<Controller>, device_manager::DriverBindError>
	{

		// Enumerate implemented ports
		let ports_implemented;
		// SAFE: Enumerate access to hardware
		let (n_ports, max_commands, supports_64bit) = unsafe {
			io.write_32(hw::REG_GHC, hw::GHC_AE);
			ports_implemented = io.read_32(hw::REG_PI);
			
			// Enumerate ports, returning all to idle if needed
			let mut n_ports = 0;
			for port_idx in 0 .. 32
			{
				if ports_implemented & (1 << port_idx) == 0 {
					continue ;
				}
				let port = PortRegs::new(&io, port_idx);
				
				// If the port is not idle, then tell it to go idle
				let cmd = port.read(hw::REG_PxCMD);
				if cmd & (hw::PxCMD_ST|hw::PxCMD_CR|hw::PxCMD_FRE|hw::PxCMD_FR) != 0 {
					port.write(hw::REG_PxCMD, 0);
				}

				n_ports += 1;
			}

			let capabilities = io.read_32(hw::REG_CAP);
			let supports_64bit = capabilities & hw::CAP_S64A != 0;
			let max_commands = ((capabilities & hw::CAP_NCS) >> hw::CAP_NCS_ofs) + 1;
			
			(n_ports, max_commands, supports_64bit,)
			};
		
		// Construct controller structure
		let mut ret = Box::new( Controller {
			// SAFE: The inner is boxed (and hence gets a fixed address) before it's borrowed
			inner: unsafe {ArefInner::new(ControllerInner {
				io_base: io,
				supports_64bit: supports_64bit,
				max_commands: max_commands as u8,
				}) },
			ports: Vec::with_capacity(n_ports),
			irq_handle: None,
			});
		
		// Allocate port information
		for port_idx in 0 .. 32
		{
			let mask = 1 << port_idx;
			if ports_implemented & mask == 0 {
				continue ;
			}


			// Check that port actually went idle
			{
				let port = PortRegs::new(&ret.inner.io_base, port_idx);
			
				let cmd = port.read(hw::REG_PxCMD);
				if cmd & (hw::PxCMD_CR|hw::PxCMD_FR) != 0 {
					todo!("AHCI Init: Wait for ports to become idle");
				}
			}

			// SAFE: Passed index is unique, will not move once stored in Vec
			let port = unsafe { try!(Port::new(ret.inner.borrow(), port_idx, max_commands as usize)) };
			ret.ports.push( port );
		}

		// Enable interrupts
		// SAFE: Exclusive access to these registers
		unsafe {
			ret.inner.io_base.write_32(hw::REG_IS, !0);
			// TODO: What else shoud be set?
			ret.inner.io_base.write_32(hw::REG_GHC, hw::GHC_AE|hw::GHC_IE);
		}
		
		// Bind interrupt
		{
			struct RawSend<T: Send>(*const T);
			unsafe impl<T: Send> Send for RawSend<T> {}
			let ret_raw = RawSend(&*ret);
			// SAFE: Pointer _should_ be valid as long as this IRQ binding exists
			ret.irq_handle = Some(::kernel::irqs::bind_object(irq, Box::new(move || unsafe { (*ret_raw.0).handle_irq() } )));
		}

		// Update port status once fully populated
		for port in &ret.ports
		{
			port.update_connection();
		}

		Ok( ret )
	}


	fn handle_irq(&self) -> bool
	{
		// SAFE: Readonly register
		let root_is = unsafe { self.inner.io_base.read_32(hw::REG_IS) };

		let mut rv = false;
		for port in &self.ports
		{
			if root_is & (1 << port.index) != 0
			{
				port.handle_irq();
				rv = true;
			}
		}
		rv
	}
}
impl_fmt! {
	Display(self, f) for ControllerInner {
		write!(f, "AHCI ?")
	}
}
impl device_manager::DriverInstance for Controller
{

}
