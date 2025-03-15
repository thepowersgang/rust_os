use ::core::sync::atomic::Ordering;
use crate::hw;
use super::DESC_COUNT;
use super::{BYTES_PER_RX_BUF,RX_BUF_PER_PAGE};

impl super::Card
{
	pub(super) fn rx_packet_inner(&self) -> Option<PacketHandle<'_>> {
		let pos = self.rx_desc_head_os.load(Ordering::Relaxed);
		let end = self.rx_desc_head_hw.load(Ordering::Relaxed);
		if pos != end {
			// Seek forwards until DESC0_LS is set
			let mut last = RxDescIdx(pos);
			while self.rx_descs[last.0 as usize][0].load(Ordering::Relaxed) & hw::DESC0_LS == 0 {
				last = last.next();
			}
			// Put that packet into a handle
			Some(PacketHandle {
				card: self,
				first_desc: RxDescIdx(pos),
				last_desc: last,
			})
		}
		else {
			None
		}
	}

	pub(super) fn update_rx_queue(&self) {
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
			if let Some(ref v) = self.rx_waiter_handle.take() {
				v.signal();
			}
		}
	}

	/// Release a Rx descriptor back to the card
	/// - UNSAFE: Caller must "own" the specified descriptor
	unsafe fn release_rx_desc(&self, idx: RxDescIdx) {
		// Rewrite the first word, resetting the buffer size and handing ownership back to the hardware
		let v = hw::DESC0_OWN | (if idx.0 == DESC_COUNT as u16-1 { hw::DESC0_EOR } else { 0 }) | (BYTES_PER_RX_BUF as u32);
		self.rx_descs[idx.0 as usize][0].store(v, Ordering::Relaxed);
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


pub(super) struct PacketHandle<'a> {
	card: &'a super::Card,
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