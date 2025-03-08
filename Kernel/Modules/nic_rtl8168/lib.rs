// Realtek 8168-compatible gigabit cards
#![no_std]
#![feature(linkage)]	// needed for `module_define`
#![feature(array_try_from_fn)]

#[macro_use]
extern crate kernel;

mod pci;
mod card;
mod hw;

use hw::Regs;

::kernel::module_define!{nic_rtl8168, [Network], init}

fn init()
{
	::kernel::device_manager::register_driver(&pci::DRIVER);
}

struct BusDev
{
	_nic_registration: ::network::nic::Registration<card::Card>,
	_irq_handle: ::kernel::irqs::ObjectHandle,
}
impl BusDev
{
	fn new(irq_num: u32, io: ::kernel::device_manager::IOBinding) -> Result<BusDev,::kernel::device_manager::DriverBindError>
	{
		// SAFE: Just reads MAC addr
		let mac_addr = unsafe {[
			io.read_8(Regs::ID0 as _), io.read_8(Regs::ID1 as _), io.read_8(Regs::ID2 as _),
			io.read_8(Regs::ID3 as _), io.read_8(Regs::ID4 as _), io.read_8(Regs::ID5 as _),
			]};
		log_notice!("RTL8168 {:?} IRQ={} MAC={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
				io, irq_num,
				mac_addr[0], mac_addr[1], mac_addr[2], mac_addr[3], mac_addr[4], mac_addr[5],
				);
		
		let card = card::Card::new(io)?;
		
		let card_nic_reg = ::network::nic::register(mac_addr, card);
		let irq_handle = {
			struct RawSend<T: Send>(*const T);
			unsafe impl<T: Send> Send for RawSend<T> {}
			let ret_raw = RawSend(&*card_nic_reg);
			// SAFE: Pointer _should_ be valid as long as this IRQ binding exists
			// SAFE: The network stack garuntees that the pointer is stable.
			::kernel::irqs::bind_object(irq_num, ::kernel::lib::mem::Box::new(move || unsafe { (*ret_raw.0).handle_irq() } ))
			};
		// SAFE: Single register access that doesn't impact memory safety
		unsafe {
			// Mask interrupts on
			// - TOK,RER,ROK
			card_nic_reg.write_16(Regs::IMR, 0x7);
			// Enable Rx/Tx engines
			card_nic_reg.write_8(Regs::CR, 0x0C);
		}

		Ok(BusDev {
			_nic_registration: card_nic_reg,
			_irq_handle: irq_handle
		})
	}
}
impl ::kernel::device_manager::DriverInstance for BusDev
{
}
