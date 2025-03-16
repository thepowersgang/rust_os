//! Logging sink, using the "high priority" queue
//! 
//! It's not really HP, but this is a second queue so can be manged with simple lockless code

use core::sync::atomic::Ordering;
// Standard maximum packet size
// - One 4096 byte page needs to fit at least two of these, plus the 32 bytes of descriptors
// 16 bytes per descriptor.
// - (4096 - 2*16) / 2 = 2032
// - (4096 - 3*16) / 3 = 1349
// - (4096 - 4*16) / 4 = 1008
// Longer log lines are around 150 characters, shorter ones are ~60
const N_DESC: usize = 3;
const BUF_LEN: usize = (4096 - N_DESC*16) / N_DESC;
#[repr(C)]
struct Header {
	ticks: u64,
	thread: u32,
	cpu: u16,
	level: u8,
	source_len: u8,
}

/// MAC header
const HEADER: &[u8] = &[
	0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
	0x12,0x34,0x56, 0x01,0x01,0x01,
	0xBA,0x55
];

pub struct LogHandler {
	card: *const super::Card,
}
unsafe impl Send for LogHandler {}
unsafe impl Sync for LogHandler {}
impl LogHandler {
	/// UNSAFE: Caller must ensure that thr returned struct doesn't outlive `Card` at the specified address
	pub(crate) unsafe fn new(card: &super::Card) -> Self
	{
		// SAFE: This is called before shared access, and this code "owns" this buffer
		let descs: &mut [ [::core::sync::atomic::AtomicU32; 4] ] = unsafe { card.log_page.as_int_mut_slice(0, N_DESC) };
		let pb = ::kernel::memory::virt::get_phys(descs.as_ptr());
		for (i,d) in descs.iter_mut().enumerate() {
			let dv = crate::hw::TxDesc {
				tx_buffer_addr: pb + (16*N_DESC + i*BUF_LEN) as u64,
				frame_length: HEADER.len() as _,
				flags3: 0,
				vlan_info: 0,
			}.into_array();
			for (s,v) in d.iter_mut().zip(dv.iter()) {
				*s.get_mut() = *v;
			}
			*d[0].get_mut() |= crate::hw::DESC0_FS|crate::hw::DESC0_LS;
		}
		*descs.last_mut().unwrap()[0].get_mut() |= crate::hw::DESC0_EOR;
		// SAFE: This is called before shared access, and this code "owns" this buffer
		let buffers: &mut [ [u8; BUF_LEN] ] = unsafe { card.log_page.as_int_mut_slice(N_DESC*16, N_DESC) };
		for b in buffers {
			b[..HEADER.len()].copy_from_slice(HEADER);
		}

		// SAFE: This pointer is stable as long as the card exists
		unsafe {
			card.write_64_pair(crate::Regs::THPDS, pb);
		}

		Self {
			card
		}
	}

	fn write_raw(&mut self, data: &[u8], no_fragment: bool) {
		// SAFE: The constructor of this type assures that the pointer is vaid
		let (lp,buf_idx) = unsafe {
			( &(*self.card).log_page, &(*self.card).log_cur_buf )
		};
		let descs: &[ [::core::sync::atomic::AtomicU32; 4] ] = lp.as_slice(0, N_DESC);
		// SAFE: This is only called with the logging lock held, and this code "owns" this buffer
		let buffers: &mut [ [u8; BUF_LEN] ] = unsafe { lp.as_int_mut_slice(N_DESC*16, N_DESC) };
		// Determine which buffer is the current one
		let b = buf_idx.load(Ordering::Relaxed) as usize;
		if descs[b][0].load(Ordering::Relaxed) & crate::hw::DESC0_OWN != 0 {
			// Uh-oh, we've hit our own tail
		}
		let cur_len = crate::hw::RxDesc::get_len(&descs[b]);
		let space = BUF_LEN - cur_len;
		// Check if this blob of data fits
		if space < data.len() {
			buffers[b][cur_len..][..data.len()].copy_from_slice(data);
			// Note: This is writing to a u16 field, but since the buffer size is smaller than u16 - this won't overflow
			descs[b][0].fetch_add(data.len() as u32, Ordering::Relaxed);
		}
		else {
			if no_fragment {
				self.flush();
				self.write_raw(data, no_fragment);
			}
			else {
				buffers[b][cur_len..].copy_from_slice(&data[..space]);
				// Note: This is writing to a u16 field, but since the buffer size is smaller than u16 - this won't overflow
				descs[b][0].fetch_add(data.len() as u32, Ordering::Relaxed);

				self.flush();
				self.write_raw(&data[space..], no_fragment);
			}
		}
	}
	/// Set OWN on the current descriptor, then move to the next descriptor
	fn flush(&mut self) {

		// SAFE: The constructor of this type assures that the pointer is vaid
		let c = unsafe { &*self.card };
		let b = c.log_cur_buf.load(Ordering::Relaxed);
		let descs: &[ [::core::sync::atomic::AtomicU32; 4] ] = c.log_page.as_slice(0, N_DESC);
		descs[b as usize][0].fetch_or(crate::hw::DESC0_OWN, Ordering::Relaxed);
		let b2 = b + 1;
		let b2 = if b2 == N_DESC as u8 { 0 } else { b2 };
		if descs[b2 as usize][0].load(Ordering::Relaxed) & crate::hw::DESC0_OWN != 0 {
			// Uh-oh, we've hit our own tail
			// - Ideally, we'd ignore the rest of this log line and then skip logs until there's a free packet
			panic!("Ran out of buffers for log sink");
		}
		// Clear the size, then add back in the fixed header
		descs[b2 as usize][0].fetch_and(0xFFFF_0000, Ordering::Relaxed);
		descs[b2 as usize][0].fetch_add(HEADER.len() as u32, Ordering::Relaxed);
		// Update the hardware
		// SAFE: Just a flag to the device
		unsafe {
			c.write_8(crate::hw::Regs::TPPoll, 1<<7);	// TPPoll.HPQ
		}
	}
}
impl ::kernel::logging::Sink for LogHandler {
	fn start(&mut self, timestamp: kernel::time::TickCount, level: kernel::logging::Level, source: &'static str) {
		let h = Header {
			ticks: timestamp,
			thread: ::kernel::threads::get_thread_id() as _,
			cpu: ::kernel::arch::cpu_num() as _,
			level: level as u8,
			source_len: source.len() as u8,
		};
		use ::kernel::lib::PodHelpers;
		self.write_raw(h.as_byte_slice(), true);
		self.write_raw(source.as_bytes(), false);
	}

	fn write(&mut self, data: &str) {
		self.write_raw(data.as_bytes(), false);
	}

	fn end(&mut self) {
		self.write_raw(&[0xFF], false);
	}
}