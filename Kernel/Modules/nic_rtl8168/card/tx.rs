use ::core::sync::atomic::Ordering;
use crate::hw;
use crate::hw::Regs;

use super::DESC_COUNT;

impl super::Card {
	pub(super) fn tx_raw_inner(&self, pkt: network::nic::SparsePacket) {
		// Count how many descriptors are needed
		let n_desc = {
			let mut n_desc = 0;
			for extent in &pkt {
				for _ in ::kernel::memory::helpers::iter_contiguous_phys(extent) {
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
					for (paddr,len, _is_last) in ::kernel::memory::helpers::iter_contiguous_phys(extent) {
						assert!(len <= u16::MAX as u32);
						self.fill_tx_desc(cur_desc, hw::TxDesc {
								tx_buffer_addr: paddr as u64,
								frame_length: len as u16,
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

	/// Interrupt handler - TX status change
	pub(super) fn update_tx_queue(&self) {
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
