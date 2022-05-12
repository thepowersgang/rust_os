// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/nic_rtl8139/lib.rs
//! Realtek 8139 driver
#![no_std]
#![feature(linkage)]	// for module_define!
use ::kernel::prelude::*;
use ::kernel::sync::Mutex;
use ::kernel::device_manager;
use ::core::sync::atomic::{Ordering,AtomicU8,AtomicU16,AtomicBool};
use ::network::nic;
use crate::hw::Regs;

#[macro_use]
extern crate kernel;
extern crate network;

mod hw;

//mod buffer_set;
mod buffer_ring;

module_define!{nic_rtl8139, [Network], init}

fn init()
{
	static PCI_DRIVER: PciDriver = PciDriver;
	device_manager::register_driver(&PCI_DRIVER);
}

const RX_BUFFER_LENGTH: usize = 0x2000+16;
const RX_BUFFER_LIMIT : usize = 0x3000;

struct BusDev( nic::Registration<Card>, ::kernel::irqs::ObjectHandle );
struct Card
{
	io_base: device_manager::IOBinding,
	
	// Buffer: Three contigious pages
	rx_buffer: ::kernel::memory::virt::ArrayHandle<u8>,
	rx_seen_ofs: AtomicU16,
	rx_packet_out: AtomicBool,	// TODO: Support having multiple packets held by the IPStack at once?

	waiter_handle: Mutex<Option<::kernel::threads::SleepObjectRef>>,

	// Transmit Buffers
	tx_buffer_handles: [ ::kernel::memory::virt::ArrayHandle<u8>; 2 ],
	//tx_slots: buffer_set::BufferSet4<TxSlot>,
	tx_slots: buffer_ring::BufferRing4<TxSlot>,
	tx_slots_active: AtomicU8,
}
struct TxSlot
{
	buffer: *mut [u8],
	//async: Option<async::ObjectHandle>,
}
unsafe impl Send for TxSlot {}
impl TxSlot
{
	fn fill(&mut self, buf: &[u8], ofs: usize) -> Result<(),usize> {
		// SAFE: Just gets the length from the slice, no memory access
		let buflen = unsafe { (*self.buffer).len() };
		if ofs > buflen || ofs + buf.len() > buflen {
			Err( buflen - (ofs + buf.len()) )
		}
		else {
			// SAFE: This object owns this buffer.
			unsafe { (*self.buffer)[ofs ..][.. buf.len()].copy_from_slice(buf); }
			Ok( () )
		}
	}
}

impl BusDev
{
	fn new_boxed(irq: u32, io: device_manager::IOBinding) -> Result<Box<BusDev>, &'static str> {

		// SAFE: Just reads MAC addr
		let mac = unsafe {[
			io.read_8(0), io.read_8(1), io.read_8(2),
			io.read_8(3), io.read_8(4), io.read_8(5),
			]};
		log_notice!("RTL8139 {:?} IRQ={} MAC={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
				io, irq,
				mac[0], mac[1], mac[2], mac[3], mac[4], mac[5],
				);
		
		let mut tx_buffer_handles = [
			::kernel::memory::virt::alloc_dma(32, 1, "rtl8139")?.into_array(),
			::kernel::memory::virt::alloc_dma(32, 1, "rtl8139")?.into_array(),
			];
		let tx_slots = [
			TxSlot { buffer: &mut tx_buffer_handles[0][..0x800], /* async: None, */ },
			TxSlot { buffer: &mut tx_buffer_handles[0][0x800..], /* async: None, */ },
			TxSlot { buffer: &mut tx_buffer_handles[1][..0x800], /* async: None, */ },
			TxSlot { buffer: &mut tx_buffer_handles[1][0x800..], /* async: None, */ },
			];

		let rx_buffer = ::kernel::memory::virt::alloc_dma(32, 3, "rtl8139")?.into_array();
		
		let card = Card {
			io_base: io,
			rx_buffer: rx_buffer,
			rx_seen_ofs: AtomicU16::new(0),
			rx_packet_out: AtomicBool::new(false),
			waiter_handle: Default::default(),
			tx_buffer_handles: tx_buffer_handles,
			tx_slots: buffer_ring::BufferRing::new(tx_slots),
			tx_slots_active: AtomicU8::new(0),
			};
		
		// SAFE: I hope so (NOTE: All addresses taken here are stable addresses)
		unsafe {
			// - Power on
			card.write_8(Regs::CONFIG1, 0x00);
			// - Reset and wait for reset bit to clear
			card.write_8(Regs::CMD, 0x10);
			while card.read_8(Regs::CMD) & 0x10 != 0 {
				// TODO: Timeout
			}

			// - Mask all interrupts off
			card.write_16(Regs::IMR, 0x0);

			// Receive buffer
			card.write_32(Regs::RBSTART, ::kernel::memory::virt::get_phys(&card.rx_buffer[0]) as u32);
			card.write_32(Regs::CBA, 0);	// NOTE: Although these two are nominally 16 bit registers, they seem to need 32 bit writes here
			card.write_32(Regs::CAPR, 0);
			// Transmit buffers
			// - TODO: These need protected access
			card.write_32(Regs::TSAD0, ::kernel::memory::virt::get_phys(&card.tx_buffer_handles[0][    0]) as u32);
			card.write_32(Regs::TSAD1, ::kernel::memory::virt::get_phys(&card.tx_buffer_handles[0][0x800]) as u32);
			card.write_32(Regs::TSAD2, ::kernel::memory::virt::get_phys(&card.tx_buffer_handles[1][    0]) as u32);
			card.write_32(Regs::TSAD3, ::kernel::memory::virt::get_phys(&card.tx_buffer_handles[1][0x800]) as u32);
			
			//card.write_16(Regs::RCR, hw::RCR_DMA_BURST_1024|hw::RCR_BUFSZ_8K16|hw::RCR_FIFO_1024|hw::RCR_OVERFLOW|0x1F);
			card.write_16(Regs::RCR, (6<<13)|(0<<11)|(6<<8)|0x80|0x1F);

			// Enable Rx and Tx engines
			card.write_8(Regs::CMD, 0x0C);
		}
		
		let card_nic_reg = nic::register(mac, card);
		let irq_handle = {
			struct RawSend<T: Send>(*const T);
			unsafe impl<T: Send> Send for RawSend<T> {}
			let ret_raw = RawSend(&*card_nic_reg);
			// SAFE: Pointer _should_ be valid as long as this IRQ binding exists
			// SAFE: The network stack garuntees that the pointer is stable.
			::kernel::irqs::bind_object(irq, Box::new(move || unsafe { (*ret_raw.0).handle_irq() } ))
			};
		// SAFE: Single register access that doesn't impact memory safety
		unsafe {
			// Mask interrupts on
			card_nic_reg.write_16(Regs::IMR, 0xE07F);
		}

		Ok( Box::new( BusDev(card_nic_reg, irq_handle) ) )
	}
}
impl device_manager::DriverInstance for BusDev
{
}

impl Card
{
	fn start_tx(&self, slot: buffer_ring::Handle<[TxSlot; 4]>, len: usize)
	{
		let idx = slot.get_index();
		log_debug!("start_tx: idx={}, len={}", idx, len);
		// SAFE: Handing a uniquely-owned buffer to the card
		unsafe {
			// - Prevent the slot's destructor from running once we trigger the hardware.
			::core::mem::forget(slot);

			let tx_status: u32
				= (len as u32 & 0x1FFF)
				| (0 << 13)	// OWN bit, clear becuase we're sending to the card.
				| (0 & 0x3F) << 16	// Early TX Threshold (0=8 bytes,n=32*n bytes)
				;
			assert!(idx < 4);
			self.io_base.write_32(Regs::TSD0 as usize + idx, tx_status);
			self.tx_slots_active.fetch_or(1 << idx, Ordering::SeqCst);
		}
		
		// Card now owns the TX descriptor, return
		// - IRQ handler will release the descriptor
	}
	
	fn handle_irq(&self) -> bool
	{
		let status = self.read_16(Regs::ISR);
		if status == 0 { return false; }
		let mut status_clear = 0;
		log_trace!("handle_irq: status=0x{:02x}", status);
		
		// ---
		// Transmit OK - Release completed descriptors
		// ---
		if status & hw::FLAG_ISR_TOK != 0
		{
			while let Some(idx) = self.tx_slots.get_first_used()
			{
				// SAFE: Read has no side-effects
				let tsd = unsafe { self.io_base.read_32(Regs::TSD0 as usize + idx) };
				if tsd & hw::FLAG_TSD_TOK == 0 {
					// This descriptor isn't done, stop
					break ;
				}
				else if self.tx_slots_active.fetch_and(!(1 << idx), Ordering::SeqCst) & 1 << idx == 0 {
					// This descriptor isn't even active
					break ;
				}
				else {
					// Activated and complete (and now marked as inactive), release it to the pool
					// SAFE: Index is corret and active
					let /*mut*/ slot = unsafe { self.tx_slots.handle_from_async(idx) };
					//if let Some(s) = slot.async.take() {
					//	s.signal(0);
					//}
					self.tx_slots.release(slot);
				}
				log_trace!("handle_irq: TOK {}", idx);
			}
			status_clear |= hw::FLAG_ISR_TOK;
		}
		// ---
		// Receive OK
		// ---
		if status & hw::FLAG_ISR_ROK != 0
		{
			// Starting at the last known Rx address, enumerate packets
			let mut read_ofs = self.rx_seen_ofs.load(Ordering::Relaxed);
			let end_ofs = self.read_16(Regs::CBA);

			let mut num_packets = 0;
			if read_ofs > end_ofs
			{
				// Rx buffer has wrapped around, read until the end of the buffer and reset read_ofs to 0
				while read_ofs < RX_BUFFER_LENGTH as u16
				{
					// NOTE: The maximum valid address is RX_BUFFER_LIMIT (larger)
					let (size, _hdr, _data) = self.get_packet(read_ofs as usize, RX_BUFFER_LIMIT);
					num_packets += 1;
					read_ofs += size as u16;
				}
				read_ofs = 0;
			}

			while read_ofs < end_ofs
			{
				let (size, _hdr, _data) = self.get_packet(read_ofs as usize, end_ofs as usize);
				num_packets += 1;
				read_ofs += size as u16;
			}

			self.rx_seen_ofs.store(read_ofs, Ordering::Relaxed);
			// NOTE: Don't write back CAPR here - It's updated once the packet has been seen by the network stack
			status_clear |= hw::FLAG_ISR_ROK;
			
			if num_packets > 0
			{
				if let Some(ref v) = *self.waiter_handle.lock()
				{
					v.signal();
					//todo!("Wake waiting thread - {:?}", v);
				}
			}
		}
		
		if status & hw::FLAG_ISR_RXOVW != 0
		{
			log_error!("{} RX buffer overflow", self.io_base);
			status_clear |= hw::FLAG_ISR_RXOVW;
		}

		if status & !status_clear != 0
		{
			todo!("Handle other status bits - 0x{:04x}", status & !status_clear);
		}

		// SAFE: No memory triggered by this, only thread active
		unsafe { self.write_16(Regs::ISR, status_clear) };

		true
	}


	fn get_packet(&self, ofs: usize, max_ofs: usize) -> (usize, u16, &[u8]) {
		assert!(ofs < max_ofs);
		assert!(ofs+4 < max_ofs);
		assert!(ofs%4 == 0);
		
		let pkt_flags = self.rx_buffer[ofs+0] as u16 | (self.rx_buffer[ofs+1] as u16 * 256);
		let raw_len   = self.rx_buffer[ofs+2] as u16 | (self.rx_buffer[ofs+3] as u16 * 256);

		let size = (raw_len + 4 + 3) & !4;
		log_trace!("get_packet({}): len={} flags=0x{:04x}", ofs, raw_len, pkt_flags);
		(size as usize, pkt_flags, &self.rx_buffer[ofs+4..][..raw_len as usize])
	}
}

impl nic::Interface for Card
{
	fn tx_raw(&self, pkt: nic::SparsePacket) {
		log_trace!("tx_raw()");
		// 1. Pick a TX buffer (waiting until one is free)
		let mut buf = self.tx_slots.acquire_wait();
		// 2. Populate the buffer with the contents of the packet
		let mut total_len = 0;
		for span in &pkt {
			buf.fill(span, total_len).expect("TODO: Error when packet TX overflows buffer");
			total_len += span.len();
		}
		//buf.async = None;

		self.start_tx(buf, total_len);
		// - No need to wait.
	}

	/*
	fn tx_async<'a, 's>(&'s self, async: async::ObjectHandle, mut stack: async::StackPush<'a, 's>, pkt: nic::SparsePacket) -> Result<(), nic::Error> {
		log_trace!("tx_async()");
		// If there's an immediately-avaliable slot, take it
		if let Some(mut buf) = self.tx_slots.try_acquire()
		{
			// Populate the hardware buffer with the contents of the packet
			let mut total_len = 0;
			for span in &pkt {
				buf.fill(span, total_len).expect("TODO: Error when packet TX overflows buffer");
				total_len += span.len();
			}

			buf.async = Some(async);
			self.start_tx(buf, total_len);
		}
		else
		{
			// Take a copy of the input packet for later use.
			// - TODO: Get a async-safe version of `pkt` instead
			let buf = {
				let mut buf = Vec::new();
				for span in &pkt {
					buf.extend_from_slice(span);
				}
				buf.into_boxed_slice()
				};
			// Handler that will pause for us (Just as cheap as using an enum)
			stack.push_closure(|_async, _stack, v| if v == !0 { None } else { Some(v) }).expect("Insufficient space when pushing closure");
			// Push a handler to start the run.
			stack.push_closure(move |async, _stack, slot_idx| {
				// SAFE: This (should) only be called when tx_slots has found a slot.
				let mut hw_buf = unsafe { self.tx_slots.handle_from_async(slot_idx) };
				hw_buf.fill(&buf, 0).expect("TODO: Error when TX overflows buffer");
				hw_buf.async = Some(async);
				self.start_tx(hw_buf, buf.len());
				Some(!0)	// Return a value that will pop the state, then pause at the above handler.
				}).expect("Insufficient space when pushing closure");
			self.tx_slots.acquire_async(async, stack);
		}
		
		Ok( () )
	}
	*/
	
	fn rx_wait_register(&self, channel: &::kernel::threads::SleepObject) {
		*self.waiter_handle.lock() = Some(channel.get_ref());
	}
	fn rx_wait_unregister(&self, _channel: &::kernel::threads::SleepObject) {
		// TODO: Check that the input matches the current
		self.waiter_handle.lock().take();
	}
	fn rx_packet(&self) -> Result<nic::PacketHandle, nic::Error> {
		struct RxPacketHandle<'a> {
			card: &'a Card,
			ofs: u16,
			len: u16,
		}
		impl<'a> nic::RxPacket for RxPacketHandle<'a> {
			fn len(&self) -> usize {
				self.len as usize
			}
			fn num_regions(&self) -> usize {
				1
			}
			fn get_region(&self, idx: usize) -> &[u8] {
				assert!(idx == 0);
				&self.card.rx_buffer[self.ofs as usize + 4 .. ][ .. self.len as usize]
			}
			fn get_slice(&self, range: ::core::ops::Range<usize>) -> Option<&[u8]> {
				let b = self.get_region(0);
				b.get(range)
			}
		}
		impl<'a> ::core::ops::Drop for RxPacketHandle<'a> {
			fn drop(&mut self) {
				// SAFE: This is the only place CAPR is written
				unsafe {
					let mut new_ofs = self.ofs + ((self.len + 3) & !3) + 4;
					log_debug!("Release packet at {} .. {} to hardware", self.ofs, new_ofs);
					if new_ofs as usize >= RX_BUFFER_LIMIT {
						new_ofs = 0;
					}
					self.card.write_16(Regs::CAPR, new_ofs.wrapping_sub(0x10));
					self.card.rx_packet_out.store(false, Ordering::Release);
				}
			}
		}
	
		// Compare rx_seen_ofs with CAPR
		let ofs = self.read_16(Regs::CAPR).wrapping_add(0x10);
		if self.rx_seen_ofs.load(Ordering::Relaxed) == ofs
		{
			Err(nic::Error::NoPacket)
		}
		// If there's already handle out, return NoPacket
		else if self.rx_packet_out.swap(true, Ordering::Relaxed) == true
		{
			Err(nic::Error::NoPacket)
		}
		else
		{
			log_debug!("RX Packet at {} being passed to stack", ofs);
			let (size, _hdr, _data) = self.get_packet(ofs as usize, RX_BUFFER_LIMIT);
			Ok(nic::PacketHandle::new(RxPacketHandle {
				card: self,
				ofs: ofs,
				len: size as u16,
				}).ok().unwrap())
		}
	}
}
#[allow(dead_code)]
impl Card
{
	unsafe fn write_8 (&self, reg: Regs, val: u8)  { self.io_base.write_8( reg as usize, val)  }
	unsafe fn write_16(&self, reg: Regs, val: u16) { self.io_base.write_16(reg as usize, val)  }
	unsafe fn write_32(&self, reg: Regs, val: u32) { self.io_base.write_32(reg as usize, val)  }
	// SAFE: All reads on this card have no side-effects
	fn read_8 (&self, reg: Regs) -> u8  { unsafe { self.io_base.read_8( reg as usize) } }
	// SAFE: All reads on this card have no side-effects
	fn read_16(&self, reg: Regs) -> u16 { unsafe { self.io_base.read_16(reg as usize) } }
	// SAFE: All reads on this card have no side-effects
	fn read_32(&self, reg: Regs) -> u32 { unsafe { self.io_base.read_32(reg as usize) } }
}

struct PciDriver;
impl device_manager::Driver for PciDriver {
	fn name(&self) -> &str {
		"rtl8139-pci"
	}
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &dyn device_manager::BusDevice) -> u32
	{
		let vendor = bus_dev.get_attr("vendor").unwrap_u32();
		let device = bus_dev.get_attr("device").unwrap_u32();
		if vendor == 0x10ec && device == 0x8139 {
			2
		}
		else {
			0
		}
	}
	fn bind(&self, bus_dev: &mut dyn device_manager::BusDevice) -> Box<dyn device_manager::DriverInstance+'static>
	{
		let irq = bus_dev.get_irq(0);
		let base = bus_dev.bind_io(0);

		BusDev::new_boxed(irq, base).unwrap()
	}
}

