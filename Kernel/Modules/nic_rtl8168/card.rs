use ::core::sync::atomic::AtomicU16;

use crate::hw;
use crate::hw::Regs;

mod tx;
mod rx;

const DESC_COUNT: usize = ::kernel::PAGE_SIZE / 16;
const RX_BUF_PER_PAGE: usize = 4;
const BYTES_PER_RX_BUF: usize = ::kernel::PAGE_SIZE / RX_BUF_PER_PAGE;

pub struct Card
{
	io: ::kernel::device_manager::IOBinding,
	/// Recive descriptors
	/// 256 descriptors per page (0x1000 / 0x10)
	/// 
	/// Rx buffer default size is 256K, so each descriptor addresses 1KiB
	rx_descs: ::kernel::memory::virt::ArrayHandle<[::core::sync::atomic::AtomicU32; 4]>,
	/// Actual RX buffers
	rx_buffers: [::kernel::memory::virt::ArrayHandle<u8>; DESC_COUNT / RX_BUF_PER_PAGE],
	/// TX descriptors
	tx_descs: ::kernel::memory::virt::ArrayHandle<[::core::sync::atomic::AtomicU32; 4]>,

	tx_sleepers: [::kernel::threads::AtomicSleepObjectRef; DESC_COUNT],

	rx_waiter_handle: ::kernel::sync::Spinlock<Option<::kernel::threads::SleepObjectRef>>,
	/// Next descriptor to be used by the hardware
	/// 
	/// Updated by the interrupt handler
	rx_desc_head_hw: AtomicU16,
	/// Next descriptor to be read by the OS (this code)
	/// 
	/// When it's not equal to `rx_desc_head_hw`, there's packets waiting
	rx_desc_head_os: AtomicU16,

	/// Next descriptor to be read by the hardware
	/// 
	/// Advanced in the interrupt handler
	tx_desc_head_hw: AtomicU16,
	/// Next descriptor available for use for TX (this code)
	tx_desc_head_os: AtomicU16,
}
impl Card
{
	pub fn new(io: ::kernel::device_manager::IOBinding) -> Result<Self,::kernel::device_manager::DriverBindError> {
		use ::kernel::memory::virt::get_phys;

		let mut card = Card {
			io,
			rx_descs: ::kernel::memory::virt::alloc_dma(64, 1, "nic_rtl8168")?.into_array(),
			tx_descs: ::kernel::memory::virt::alloc_dma(64, 1, "nic_rtl8168")?.into_array(),
			rx_buffers: ::core::array::try_from_fn(|_| ::kernel::memory::virt::alloc_dma(64, 1, "nic_rtl8168").map(|v| v.into_array()))?,
			rx_desc_head_os: AtomicU16::new(0),
			rx_desc_head_hw: AtomicU16::new(0),
			tx_desc_head_hw: AtomicU16::new(0),
			tx_desc_head_os: AtomicU16::new(0),
			rx_waiter_handle: ::kernel::sync::Spinlock::new(None),
			tx_sleepers: [const { ::kernel::threads::AtomicSleepObjectRef::new() }; DESC_COUNT],
			};
		
		// Fill the Rx descriptors with buffer addresses
		for (i,d) in card.rx_descs.iter_mut().enumerate() {
			let ofs = (i % RX_BUF_PER_PAGE) * BYTES_PER_RX_BUF;
			*d = hw::RxDescOwn::new(
				get_phys(card.rx_buffers[i / RX_BUF_PER_PAGE].as_ptr().wrapping_add(ofs)),
				BYTES_PER_RX_BUF as u16,
				).into_array().map(|v| v.into());
		}
		// Empty the TX buffers (importantly - clearing the OWN bit)
		for d in card.tx_descs.iter_mut() {
			*d = [Default::default(), Default::default(), Default::default(), Default::default()];
		}
		// Set EOR on the final entry of both rings
		*card.rx_descs.last_mut().unwrap()[0].get_mut() |= hw::DESC0_EOR;
		*card.tx_descs.last_mut().unwrap()[0].get_mut() |= hw::DESC0_EOR;

		// SAFE: Checked hardware accesses
		unsafe {
			// Reset
			card.write_8(Regs::CR, 0x10);
			while card.read_8(Regs::CR) & 0x10 == 0x10 {
			}

			// Set the descriptor pool addresses
			card.write_64_pair(Regs::RDSAR, get_phys(card.rx_descs.as_ptr()));
			card.write_64_pair(Regs::TNPDS, get_phys(card.tx_descs.as_ptr()));
			// Set RCR and TCR
			card.write_32(Regs::RCR, 0x0000_820E);	// RCR: DMA after 256, 64 burst, accept all addressed packets
			card.write_32(Regs::TCR, 0x3000_0000);	// TCR: DMA 64 burst
			// RMS and MTPS have to be set to non-zero for things to work
			card.write_16(Regs::RMS, 9000);	// Jumbo frames!
			card.write_16(Regs::MTPS, 9000);	// Jumbo frames!

			// NOTE: CR is updated by caller
		}

		Ok(card)
	}

	pub fn handle_irq(&self) -> bool {
		// SAFE: Reading ISR has no side-effects
		// SAFE: Writing just clears the interrupt bit
		let isr = unsafe {
			let v = self.read_16(Regs::ISR);
			self.write_16(Regs::ISR, v);
			v
		};

		if isr & hw::ISR_ROK != 0 {
			// Rx OK
			// - Check Rx head
			self.update_rx_queue();
		}
		if isr & hw::ISR_TOK != 0 {
			// Tx OK - check Rx head
			self.update_tx_queue();
		}

		isr != 0
	}
}

impl Card 
{
	// TODO: Is reading safe?
	pub unsafe fn read_8(&self, reg: Regs) -> u8 {
		self.io.read_8(reg as u8 as usize)
	}
	pub unsafe fn read_16(&self, reg: Regs) -> u16 {
		self.io.read_16(reg as u8 as usize)
	}

	pub unsafe fn write_8(&self, reg: Regs, val: u8) {
		self.io.write_8(reg as u8 as usize, val);
	}
	pub unsafe fn write_16(&self, reg: Regs, val: u16) {
		self.io.write_16(reg as u8 as usize, val);
	}
	pub unsafe fn write_32(&self, reg: Regs, val: u32) {
		self.io.write_32(reg as u8 as usize, val);
	}
	pub unsafe fn write_64_pair(&self, reg: Regs, val: u64) {
		self.io.write_32(reg as u8 as usize + 0, val as u32);
		self.io.write_32(reg as u8 as usize + 4, (val >> 32) as u32);
	}
}

impl ::network::nic::Interface for Card {
	fn tx_raw(&self, pkt: network::nic::SparsePacket) {
		self.tx_raw_inner(pkt)
	}

	fn rx_wait_register(&self, channel: &kernel::threads::SleepObject) {
		*self.rx_waiter_handle.lock() = Some(channel.get_ref());
	}

	fn rx_wait_unregister(&self, channel: &kernel::threads::SleepObject) {
		let mut lh = self.rx_waiter_handle.lock();
		match *lh {
		Some(ref v) if v.is_from(channel) => *lh = None,
		_ => {},
		}
	}

	fn rx_packet(&self) -> Result<network::nic::PacketHandle<'_>, network::nic::Error> {
		match self.rx_packet_inner()
		{
		Some(v) => Ok(::network::nic::PacketHandle::new(v).ok().expect("Cannot fit PacketHandle")),
		None => Err(::network::nic::Error::NoPacket),
		}
	}
}
