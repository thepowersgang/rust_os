use ::core::sync::atomic::{Ordering, AtomicU16};

use crate::hw;
use crate::hw::Regs;

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
			// TODO: Set RCR
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

	fn update_rx_queue(&self) {
		// Just update `rx_desc_head_hw`
		let init_pos = RxDescIdx(self.rx_desc_head_hw.load(Ordering::Relaxed));
		let mut looped = false;
		let mut pos = init_pos;
		loop {
			if self.rx_descs[pos.0 as usize][0].load(Ordering::Relaxed) & hw::DESC0_OWN != 0 {
				break;
			}
			looped = true;
			pos = pos.next();
			if pos == init_pos {
				// Stops an infinite loop
				break;
			}
		}
		self.rx_desc_head_hw.store(pos.0, Ordering::Relaxed);
		if looped {
			if let Some(ref v) = *self.rx_waiter_handle.lock() {
				v.signal();
			}
		}
	}
	fn update_tx_queue(&self) {
		let init_pos = TxDescIdx(self.tx_desc_head_hw.load(Ordering::Relaxed));
		let mut pos = init_pos;
		loop {
			if self.tx_descs[pos.0 as usize][0].load(Ordering::Relaxed) & hw::DESC0_OWN != 0 {
				break;
			}

			// Inform senders of the status
			if let Some(v) = self.tx_sleepers[pos.0 as usize].take() {
				v.signal();
			}

			pos = pos.next();
			if pos == init_pos {
				// Stops an infinite loop
				break;
			}
		}
		self.tx_desc_head_hw.store(pos.0, Ordering::Relaxed);
	}
}

/// Descriptor queue handling
impl Card
{
	/// Release a Rx descriptor back to the card
	/// - UNSAFE: Caller must "own" the specified descriptor
	unsafe fn release_rx_desc(&self, idx: RxDescIdx) {
		// Rewrite the first word, resetting the buffer size and handing ownership back to the hardware
		let v = hw::DESC0_OWN | (if idx.0 == DESC_COUNT as u16-1 { hw::DESC0_EOR } else { 0 }) | (BYTES_PER_RX_BUF as u32);
		self.rx_descs[idx.0 as usize][0].store(v, Ordering::Relaxed);
	}

	/// Fill a TX descriptor with the contents of a structure
	/// 
	/// - NOTE: This clears the OWN/FS/LS bits, those will be set when [Card::activate_tx_descs] is called
	/// - UNSAFE: Caller must ensure that the buffers pointed in `info` are valid and written used until the card indicates it is done.
	unsafe fn fill_tx_desc(&self, idx: TxDescIdx, mut info: hw::TxDesc) {
		info.flags3 &= 0x3F;
		for (a,b) in Iterator::zip(self.tx_descs[idx.0 as usize].iter(), info.into_array())
		{
			a.store(b, Ordering::Relaxed);
		}
	}
	/// Hand a range of TX descriptors over to the hardware
	unsafe fn activate_tx_descs(&self, first: TxDescIdx, last: TxDescIdx) {
		// TODO: Since this is shared with the hardware, would want to ensure that all of these sync.
		self.tx_descs[last.0 as usize][0].fetch_or(hw::DESC0_LS, Ordering::Relaxed);
		self.tx_descs[first.0 as usize][0].fetch_or(hw::DESC0_FS, Ordering::Relaxed);
		// - Set OWN, working backwards
		let mut cur = last;
		while cur != first {
			self.tx_descs[cur.0 as usize][0].fetch_or(hw::DESC0_OWN, Ordering::SeqCst);
			cur = cur.prev();
		}
		::core::sync::atomic::fence(Ordering::SeqCst);
	}
}

#[derive(Copy, Clone, PartialEq)]
struct RxDescIdx(u16);
impl RxDescIdx {
	fn next(self) -> Self {
		RxDescIdx(if self.0 == DESC_COUNT as u16 - 1 { 0 } else { self.0 + 1 })
	}
	//fn prev(self) -> Self {
	//	RxDescIdx(if self.0 == 0 { DESC_COUNT as u16 - 1 } else { self.0 - 1 })
	//}

	fn ofs(self, v: usize) -> Self {
		let n = self.0 + v as u16;
		RxDescIdx(n % DESC_COUNT as u16)
	}
	/// Descriptor indexes between these two, increasing from `self` to `other`
	fn dist_to(self, other: RxDescIdx) -> usize {
		((other.0 + DESC_COUNT as u16 - self.0) % DESC_COUNT as u16) as usize
	}
}
#[derive(Copy, Clone, PartialEq)]
struct TxDescIdx(u16);
impl TxDescIdx {
	fn next(self) -> Self {
		TxDescIdx(if self.0 == DESC_COUNT as u16 - 1 { 0 } else { self.0 + 1 })
	}
	fn prev(self) -> Self {
		TxDescIdx(if self.0 == 0 { DESC_COUNT as u16 - 1 } else { self.0 - 1 })
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
	//pub unsafe fn write_32(&self, reg: Regs, val: u32) {
	//	self.io.write_32(reg as u8 as usize, val);
	//}
	pub unsafe fn write_64_pair(&self, reg: Regs, val: u64) {
		self.io.write_32(reg as u8 as usize + 0, val as u32);
		self.io.write_32(reg as u8 as usize + 4, (val >> 32) as u32);
	}
}

struct IterPhysExtents<'a> {
	remain: &'a [u8],
}
impl IterPhysExtents<'_> {
	fn new(v: &[u8]) -> IterPhysExtents {
		IterPhysExtents { remain: v }
	}
}
impl Iterator for IterPhysExtents<'_> {
	type Item = (u64,u16);
	fn next(&mut self) -> Option<Self::Item> {
		use ::kernel::memory::virt::get_phys;
		if self.remain.is_empty() {
			None
		}
		else {
			let a = get_phys(self.remain.as_ptr());
			let space = ::kernel::PAGE_SIZE - (a as usize) % ::kernel::PAGE_SIZE;
			if space >= self.remain.len() {
				self.remain = &[];
				Some((a, self.remain.len() as u16))
			}
			else {
				let mut rv_len = space as u16;
				self.remain = &self.remain[space..];
				while !self.remain.is_empty() && rv_len < u16::MAX && a + rv_len as u64 == get_phys(self.remain.as_ptr()) {
					// Contigious physical, so can advance rv.1
					let space = ::kernel::PAGE_SIZE;
					let space = space.min( (u16::MAX - rv_len) as usize );
					let space = space.min( self.remain.len() );
					self.remain = &self.remain[space..];
					rv_len += space as u16;
				}
				Some((a, rv_len))
			}
		}
	}
}
impl ::network::nic::Interface for Card {
	fn tx_raw(&self, pkt: network::nic::SparsePacket) {
		// Count how many descriptors are needed
		let n_desc = {
			let mut n_desc = 0;
			for extent in &pkt {
				for _ in IterPhysExtents::new(extent) {
					n_desc += 1;
				}
			}
			n_desc
		};
		if n_desc > 0 {
			// Obtain that many from the pool
			let first_desc = {
				let mut p1 = self.tx_desc_head_os.load(Ordering::Relaxed);
				loop {
					let p2 = self.tx_desc_head_hw.load(Ordering::Relaxed);
					let space = if p1 == p2 {
							if self.tx_descs[p1 as usize][0].load(Ordering::Relaxed) & hw::DESC0_OWN != 0 {
								// Full, need to wait
								todo!("TX buffers exhausted - wait for more?")
							}
							else {
								// Empty
								DESC_COUNT
							}
						}
						else {
							(p2 + DESC_COUNT as u16 - p1) as usize % DESC_COUNT
						};
					if space < n_desc {
						todo!("Not enough buffers, use a bounce buffer")
					}
					else {
						let new_end = (p1 + n_desc as u16) % DESC_COUNT as u16;
						match self.tx_desc_head_os.compare_exchange(p1, new_end, Ordering::Relaxed, Ordering::Relaxed) {
						Ok(_) => break TxDescIdx(p1),
						Err(v) => p1 = v,
						}
					}
				}
			};
			// SAFE: Destructor will be called
			let so = unsafe { ::kernel::threads::SleepObject::new("rtl8168 tx") };
			self.tx_sleepers[first_desc.0 as usize].set(so.get_ref());
			// SAFE: Buffer addresses are correct, and we will wait until the hardware releases
			unsafe {
				let mut cur_desc = first_desc;
				// - Fill buffer addresses
				for extent in &pkt {
					for (paddr,len) in IterPhysExtents::new(extent) {
						self.fill_tx_desc(cur_desc, hw::TxDesc {
								tx_buffer_addr: paddr,
								frame_length: len,
								flags3: 0,
								vlan_info: 0,
							});
						cur_desc = cur_desc.next();
					}
				}
				// - Set FS/LS
				self.activate_tx_descs(first_desc, cur_desc.prev());
			}

			// Set TPPoll.NPQ to inform the card that there's data here
			// SAFE: Just a flag to the device
			unsafe {
				self.write_8(Regs::TPPoll, 0x40);
			}
			// Wait until TX is complete
			// NOTE: This can't wake unless someone explicitly wakes the object?
			so.wait();
			// TODO: Get TX status? Hard to do, as that would require this code to release the tx descs
		}
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
		let pos = self.rx_desc_head_os.load(Ordering::Relaxed);
		let end = self.rx_desc_head_hw.load(Ordering::Relaxed);
		if pos != end {
			// Seek forwards until DESC0_LS is set
			let mut last = RxDescIdx(pos);
			while self.rx_descs[last.0 as usize][0].load(Ordering::Relaxed) & hw::DESC0_LS == 0 {
				last = last.next();
			}
			// Put that packet into a handle
			Ok(::network::nic::PacketHandle::new(PacketHandle {
				card: self,
				first_desc: RxDescIdx(pos),
				last_desc: last,
			}).ok().unwrap())
		}
		else {
			Err(::network::nic::Error::NoPacket)
		}
	}
}

struct PacketHandle<'a> {
	card: &'a Card,
	first_desc: RxDescIdx,
	last_desc: RxDescIdx,
}
impl ::network::nic::RxPacket for PacketHandle<'_> {
	fn len(&self) -> usize {
		let mut len = 0;
		let mut pos = self.first_desc;
		while pos != self.last_desc {
			len += hw::RxDesc::get_len(&self.card.rx_descs[pos.0 as usize]);
			pos = pos.next();
		}
		len
	}

	fn num_regions(&self) -> usize {
		self.first_desc.dist_to(self.last_desc) + 1
	}

	fn get_region(&self, idx: usize) -> &[u8] {
		let pos = self.first_desc.ofs(idx);
		let len = hw::RxDesc::get_len(&self.card.rx_descs[pos.0 as usize]);
		let ofs = BYTES_PER_RX_BUF * (pos.0 as usize % RX_BUF_PER_PAGE);
		&self.card.rx_buffers[pos.0 as usize / RX_BUF_PER_PAGE][ofs..][..len]
	}

	fn get_slice(&self, range: ::core::ops::Range<usize>) -> Option<&[u8]> {
		let mut ofs = range.start;
		let mut pos = self.first_desc;
		while pos != self.last_desc {
			let len = hw::RxDesc::get_len(&self.card.rx_descs[pos.0 as usize]);
			if ofs >= len {
				ofs -= len;
			}
			else {
				let des_len = range.end - range.start;
				if des_len > len || ofs + des_len > len {
					return None
				}
				else {
					let buf = {
						let ofs = BYTES_PER_RX_BUF * (pos.0 as usize % RX_BUF_PER_PAGE);
						&self.card.rx_buffers[pos.0 as usize / RX_BUF_PER_PAGE][ofs..][..len]
						};
					return Some(&buf[ofs..][..des_len])
				}
			}
			pos = pos.next();
		}
		None
	}
}
impl Drop for PacketHandle<'_> {
	fn drop(&mut self) {
		let mut pos = self.first_desc;
		while pos != self.last_desc {
			// SAFE: This handle owns the descriptor, and won't use it again
			unsafe {
				self.card.release_rx_desc(pos);
			}
			pos = pos.next();
		}
	}
}