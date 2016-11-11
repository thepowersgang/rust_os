// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/nic_rtl8139/lib.rs
//! Realtek 8139 driver
#![no_std]
#![feature(linkage)]
use kernel::prelude::*;
use network::nic;
use hw::Regs;

#[macro_use]
extern crate kernel;
extern crate network;

mod hw;

module_define!{nic_Rtl8139, [Network], init}

fn init()
{
	static PCI_DRIVER: PciDriver = PciDriver;
	::kernel::device_manager::register_driver(&PCI_DRIVER);
}


struct BusDev( nic::Registration<Card> );
struct Card
{
	io_base: ::kernel::device_manager::IOBinding,
	irq_handle: Option<::kernel::irqs::ObjectHandle>,
	
	// Buffer: Three contigious pages
	rx_buffer: ::kernel::memory::virt::ArrayHandle<u8>,

	tx_buffers: [ ::kernel::memory::virt::ArrayHandle<u8>; 2 ],
}

impl BusDev
{
	fn new_boxed(irq: u32, io: ::kernel::device_manager::IOBinding) -> Result<Box<BusDev>, &'static str> {

		// SAFE: Just reads MAC addr
		let mac = unsafe {[
			io.read_8(0), io.read_8(1), io.read_8(2),
			io.read_8(3), io.read_8(4), io.read_8(5),
			]};

		let rv = Box::new( BusDev( nic::register(mac, Card {
			io_base: io,
			irq_handle: None,
			rx_buffer: ::kernel::memory::virt::alloc_dma(32, 3, "rtl8139")?.into_array(),
			tx_buffers: [
				::kernel::memory::virt::alloc_dma(32, 1, "rtl8139")?.into_array(),
				::kernel::memory::virt::alloc_dma(32, 1, "rtl8139")?.into_array(),
				],
			}) ) );
		
		{
			let card = &*rv.0;

			// SAFE: I hope so
			unsafe {
				// - Power on
				card.write_8(Regs::CONFIG1, 0x00);
				// - Reset and wait for reset bit to clear
				card.write_8(Regs::CMD, 0x10);
				while card.read_8(Regs::CMD) & 0x10 != 0 {
					// TODO: Timeout
				}

				// - Mask all interrupts on
				card.write_16(Regs::IMR, 0xE07F);

				// Receive buffer
				card.write_32(Regs::RBSTART, ::kernel::memory::virt::get_phys(&card.rx_buffer[0]) as u32);
				card.write_32(Regs::CBA, 0);
				card.write_32(Regs::CAPR, 0);
				// Transmit buffers
				// - TODO: These need protected access
				card.write_32(Regs::TSAD0, ::kernel::memory::virt::get_phys(&card.tx_buffers[0][    0]) as u32);
				card.write_32(Regs::TSAD1, ::kernel::memory::virt::get_phys(&card.tx_buffers[0][0x800]) as u32);
				card.write_32(Regs::TSAD2, ::kernel::memory::virt::get_phys(&card.tx_buffers[1][    0]) as u32);
				card.write_32(Regs::TSAD3, ::kernel::memory::virt::get_phys(&card.tx_buffers[1][0x800]) as u32);
				
				//card.write_16(Regs::RCR, hw::RCR_DMA_BURST_1024|hw::RCR_BUFSZ_8K16|hw::RCR_FIFO_1024|hw::RCR_OVERFLOW|0x1F);
				card.write_16(Regs::RCR, (6<<13)|(0<<11)|(6<<8)|0x80|0x1F);

				// Enable Rx and Tx engines
				card.write_8(Regs::CMD, 0x0C);
			}
		}

		Ok(rv)
	}
}
impl ::kernel::device_manager::DriverInstance for BusDev
{
}
impl nic::Interface for Card
{
	fn tx_raw(&self, pkt: nic::SparsePacket) {
		// 1. Pick a TX buffer (what to do when there is none?)
		// 2. Populate the buffer with the contents of the packet
		todo!("tx_raw");
	}
	fn rx_wait_register(&self, channel: &::kernel::async::Waiter) {
		todo!("rx_wait_register");
	}
	fn rx_packet(&self) -> Result<nic::PacketHandle, nic::Error> {
		todo!("rx_packet");
	}
}
#[allow(dead_code)]
impl Card
{
	unsafe fn write_8 (&self, reg: Regs, val: u8)  { self.io_base.write_8( reg as usize, val)  }
	unsafe fn write_16(&self, reg: Regs, val: u16) { self.io_base.write_16(reg as usize, val)  }
	unsafe fn write_32(&self, reg: Regs, val: u32) { self.io_base.write_32(reg as usize, val)  }
	unsafe fn read_8 (&self, reg: Regs) -> u8  { self.io_base.read_8( reg as usize) }
	unsafe fn read_16(&self, reg: Regs) -> u16 { self.io_base.read_16(reg as usize) }
	unsafe fn read_32(&self, reg: Regs) -> u32 { self.io_base.read_32(reg as usize) }
}

struct PciDriver;
impl ::kernel::device_manager::Driver for PciDriver {
	fn name(&self) -> &str {
		"rtl8139-pci"
	}
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &::kernel::device_manager::BusDevice) -> u32
	{
		let classcode = bus_dev.get_attr("class").unwrap_u32();
		// [class] [subclass] [IF] [ver]
		if classcode & 0xFFFFFF00 == 0x01060100 {
			1	// Handle as weakly as possible (vendor-provided drivers bind higher)
		}
		else {
			0
		}
	}
	fn bind(&self, bus_dev: &mut ::kernel::device_manager::BusDevice) -> Box<::kernel::device_manager::DriverInstance+'static>
	{
		let irq = bus_dev.get_irq(0);
		let base = bus_dev.bind_io(0);

		BusDev::new_boxed(irq, base).unwrap()
	}
}

