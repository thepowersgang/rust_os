//! Interrupt endpoints
use ::kernel::prelude::*;
use ::core::sync::atomic::Ordering;
use ::kernel::memory::helpers::iter_contiguous_phys;

pub struct Interrupt
{
	host: crate::HostRef,

	addr: u8,
	index: u8,
	
	cur_buffer: ::core::sync::atomic::AtomicBool,
	other_borrowed: ::core::sync::atomic::AtomicBool,

	max_packet_size: usize,
	buffers: Vec<u8>,
}
impl Interrupt
{
	pub(crate) fn new(host: crate::HostRef, endpoint: ::usb_core::host::EndpointAddr, period_ms: usize, max_packet_size: usize) -> Result<Self,::kernel::memory::virt::MapError> {
		let index = endpoint.endpt() * 2 + 1;
		let period_128us_log2 = ((usize::BITS - period_ms.leading_zeros()) + 3) as u8;
		host.claim_endpoint(endpoint.dev_addr(), index, crate::device_state::EndpointType::InterruptIn { period_128us_log2 }, max_packet_size)?;

		let rv = Interrupt {
			host,
			addr: endpoint.dev_addr(),
			index,
			cur_buffer: Default::default(),
			other_borrowed: Default::default(),
			max_packet_size,
			buffers: vec![0; max_packet_size * 2],
		};
		rv.enqueue();
		Ok(rv)
	}

	fn get_buf(&self, idx: bool) -> &[u8] {
		&self.buffers[self.max_packet_size * idx as usize..][..self.max_packet_size]
	}
	fn enqueue(&self) {
		let mut state = self.host.push_ep_trbs(self.addr, self.index);
		
		let buffer = self.get_buf(self.cur_buffer.load(::core::sync::atomic::Ordering::SeqCst));
		for (paddr, len, is_last) in iter_contiguous_phys(buffer) {
			// SAFE: Trusting ourselves to wait until the hardware is done
			unsafe {
				let (data,transfer_length) = (crate::hw::structs::TrbNormalData::Pointer(paddr), len as u32);
				state.push(crate::hw::structs::TrbNormal {
					data,
					transfer_length,
					chain_bit: !is_last,
					evaluate_next_trb: !is_last,
					interrupt_on_short_packet: false,
					ioc: is_last,
					no_snoop: false,
					td_size: 1, // TODO
					interrupter_target: 0,
					block_event_interrupt: false,
					});
			}
		}
	}
}
impl ::core::ops::Drop for Interrupt {
	fn drop(&mut self) {
		self.host.release_endpoint(self.addr, self.index);
	}
}

impl ::usb_core::host::InterruptEndpoint for Interrupt
{
	fn wait<'a>(&'a self) -> ::usb_core::host::AsyncWaitIo<'a, ::usb_core::host::IntBuffer<'a>> {
		super::make_asyncwaitio(async move {
			let unused_len = self.host.wait_for_completion(self.addr, self.index).await.expect("TODO");
			let ret_len = self.max_packet_size as u32 - unused_len;
			let buf = self.cur_buffer.fetch_xor(true, Ordering::Relaxed);
			assert!( !self.other_borrowed.swap(true, Ordering::Relaxed), "Buffer already borrowed?");
			self.enqueue();
			log_debug!("Interrupt::wait: {} {:?}", buf, ::kernel::logging::HexDump(&self.get_buf(buf)[..ret_len as usize]));
			::usb_core::host::IntBuffer::new(IntBuffer { src: self, len: ret_len }).ok().unwrap()
		})
	}
}

struct IntBuffer<'a> {
	src: &'a Interrupt,
	len: u32,
}
impl<'a> ::usb_core::handle::RemoteBuffer for IntBuffer<'a> {
	fn get(&self) -> &[u8] {
		&self.src.get_buf(!self.src.cur_buffer.load(Ordering::Relaxed))[..self.len as usize]
	}
}
impl<'a> ::core::ops::Drop for IntBuffer<'a> {
	fn drop(&mut self) {
		self.src.other_borrowed.store(false, Ordering::Relaxed)
	}
}