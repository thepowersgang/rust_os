// Realtek 8168-compatible gigabit cards
#![no_std]
#![feature(linkage)]	// needed for `module_define`
#![feature(array_try_from_fn)]

#[macro_use]
extern crate kernel;

mod pci;
mod hw;

use hw::Regs;

::kernel::module_define!{nic_rtl8168, [Network], init}

fn init()
{
	::kernel::device_manager::register_driver(&pci::DRIVER);
}

struct BusDev
{
	_nic_registration: ::network::nic::Registration<Card>,
	_irq_handle: ::kernel::irqs::ObjectHandle,
}
impl BusDev
{
	fn new(irq_num: u32, io: ::kernel::device_manager::IOBinding) -> Result<BusDev,::kernel::device_manager::DriverBindError>
	{
		// SAFE: Just reads MAC addr
		let mac_addr = unsafe {[
			io.read_8(0), io.read_8(1), io.read_8(2),
			io.read_8(3), io.read_8(4), io.read_8(5),
			]};
		log_notice!("RTL8168 {:?} IRQ={} MAC={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
				io, irq_num,
				mac_addr[0], mac_addr[1], mac_addr[2], mac_addr[3], mac_addr[4], mac_addr[5],
				);
		
		let card = Card::new(io)?;
		
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
			card_nic_reg.write_8(Regs::CR, 3);
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

const RX_DESC_PER_PG: usize = 4;
const BYTES_PER_RX_DESC: usize = ::kernel::PAGE_SIZE / RX_DESC_PER_PG;
struct Card
{
	io: ::kernel::device_manager::IOBinding,
	/// Recive descriptors
	/// 256 descriptors per page (0x1000 / 0x10)
	/// 
	/// Rx buffer default size is 256K, so each descriptor addresses 1KiB
	rx_descs: ::kernel::memory::virt::ArrayHandle<[u32; 4]>,
	// RX buffers
	rx_buffers: [::kernel::memory::virt::ArrayHandle<u8>; 256 / 2],
	// TX descriptors
	tx_descs: ::kernel::memory::virt::ArrayHandle<[u32; 4]>,

	//last_rx_desc: AtomicU16,
	//last_rx_desc: AtomicU16,
}
impl Card
{
	fn new(io: ::kernel::device_manager::IOBinding) -> Result<Self,::kernel::device_manager::DriverBindError> {
		use ::kernel::memory::virt::get_phys;

		let mut card = Card {
			io,
			rx_descs: ::kernel::memory::virt::alloc_dma(64, 1, "nic_rtl8168")?.into_array(),
			tx_descs: ::kernel::memory::virt::alloc_dma(64, 1, "nic_rtl8168")?.into_array(),
			rx_buffers: ::core::array::try_from_fn(|_| ::kernel::memory::virt::alloc_dma(64, 1, "nic_rtl8168").map(|v| v.into_array()))?,
			};
		
		for (i,d) in card.rx_descs.iter_mut().enumerate() {
			let ofs = (i % RX_DESC_PER_PG) * BYTES_PER_RX_DESC;
			*d = hw::RxDescOwn::new(
				get_phys(card.rx_buffers[i / RX_DESC_PER_PG].as_ptr().wrapping_add(ofs)),
				BYTES_PER_RX_DESC as u16,
				).into_array();
		}
		for d in card.tx_descs.iter_mut() {
			*d = [0; 4];
		}

		// SAFE: Checked hardware accesses
		unsafe {
			card.write_64_pair(Regs::RDSAR, get_phys(card.rx_descs.as_ptr()));
			card.write_64_pair(Regs::TNPDS, get_phys(card.tx_descs.as_ptr()));
			// TODO: Set RCR
			// RMS and MTPS have to be set to non-zero for things to work
			card.write_16(Regs::RMS, 9000);	// Jumbo frames!
			card.write_16(Regs::MTPS, 9000);	// Jumbo frames!

			// NOTE: CR is updated by caller
		}

		Ok(card)
	}

	fn handle_irq(&self) -> bool {
		unimplemented!()
	}

	unsafe fn write_8(&self, reg: Regs, val: u8) {
		self.io.write_8(reg as u8 as usize, val);
	}
	unsafe fn write_16(&self, reg: Regs, val: u16) {
		self.io.write_16(reg as u8 as usize, val);
	}
	unsafe fn write_32(&self, reg: Regs, val: u32) {
		self.io.write_32(reg as u8 as usize, val);
	}
	unsafe fn write_64_pair(&self, reg: Regs, val: u64) {
		self.io.write_32(reg as u8 as usize + 0, val as u32);
		self.io.write_32(reg as u8 as usize + 4, (val >> 32) as u32);
	}
}
impl ::network::nic::Interface for Card {
	fn tx_raw(&self, pkt: network::nic::SparsePacket) {
		unimplemented!()
	}

	fn rx_wait_register(&self, channel: &kernel::threads::SleepObject) {
		unimplemented!()
	}

	fn rx_wait_unregister(&self, channel: &kernel::threads::SleepObject) {
		unimplemented!()
	}

	fn rx_packet(&self) -> Result<network::nic::PacketHandle, network::nic::Error> {
		unimplemented!()
	}
}