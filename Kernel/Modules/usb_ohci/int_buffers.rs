// "Tifflin" Kernel - OHCI USB driver
// - By John Hodge (Mutabah / thePowersGang)
//
// Modules/usb_ohci/int_buffers.rs
//! Buffer pool for interrupt endpoints
use ::core::sync::atomic::{Ordering, AtomicU32};

/// A single page used as a pool of interrupt transfer buffers
pub struct InterruptBuffers
{
	// TODO: Re-write this to have a fixed unit size and be shared across devices
	// 4096 bytes w/ 64 buffers (and overhead) = 63 bytes per alloc max
	page: ::kernel::memory::virt::AllocHandle,
	max_packet_size: usize,
}
impl InterruptBuffers
{
	pub fn new(max_packet_size: usize) -> InterruptBuffers {
		InterruptBuffers {
			page: ::kernel::memory::virt::alloc_dma(32, 1, "usb_ohci interrupt").expect(""),
			max_packet_size: max_packet_size,
			}
	}
	fn get_bitset(&self) -> &AtomicU32 {
		self.page.as_ref(::kernel::PAGE_SIZE - 4)
	}

	pub fn max_packet_size(&self) -> usize {
		self.max_packet_size
	}

	/// Obtain a buffer from the pool
	pub fn get_buffer(&self) -> Option<FillingHandle> {
		let max_count = (::kernel::PAGE_SIZE - 4) / self.max_packet_size;
		let max_count = ::core::cmp::min(max_count, 32);
		let bitset = self.get_bitset();
		loop
		{
			let v = bitset.load(Ordering::SeqCst);
			if v == !0 {
				return None;
			}
			let i = (!v).trailing_zeros() as usize;
			if i >= max_count {
				return None;
			}
			let mask = 1 << i;
			assert!(v & mask == 0, "{:x} & {:x}", v, mask);
			if bitset.compare_exchange(v, v | mask, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
				//log_debug!("{:p}: Alloc {}", self.page.as_ref(0) as *const u8, i);
				return Some(FillingHandle {
					ptr: self.page.as_ref(i * self.max_packet_size) as *const u8,
					cap: self.max_packet_size,
					});
			}
		}
	}

	/// UNSAFE: Pointer must be to a location within an interrupt buffer page
	unsafe fn drop_handle(ptr: *const u8, size: usize) {
		let page_base = ptr as usize & !(::kernel::PAGE_SIZE - 1);
		let bitset = {
			let bitset_ptr = page_base + (::kernel::PAGE_SIZE - 4);
			&*(bitset_ptr as *const ::core::sync::atomic::AtomicU32)
			};
		let idx = (ptr as usize - page_base) / size;
		let mask = 1 << idx;
		//log_debug!("{:p}: Free {}", page_base as *const u8, idx);
		bitset.fetch_and(!mask, Ordering::SeqCst);
	}
}
impl ::core::ops::Drop for InterruptBuffers
{
	fn drop(&mut self) {
		let v = self.get_bitset().load(Ordering::SeqCst);
		assert!(v == 0);
	}
}

/// Handle to a buffer being filled by hardware
pub struct FillingHandle
{
	ptr: *const u8,
	cap: usize,
}
unsafe impl Sync for FillingHandle {}
unsafe impl Send for FillingHandle {}
impl FillingHandle
{
	pub fn get_phys_range(&self) -> (u32, u32) {
		let first = ::kernel::memory::virt::get_phys(self.ptr);
		// SAFE: Pointer is within allocation bounds
		let last = ::kernel::memory::virt::get_phys(unsafe { self.ptr.offset(self.cap as isize - 1) });
		(first as u32, last as u32,)
	}
	/// UNSAFE: Hardware must not be accessing the buffer any more
	pub unsafe fn filled(self, len: usize) -> FilledHandle {
		assert!(len <= self.cap);
		let rv = FilledHandle {
			ptr: self.ptr,
			cap: self.cap as u16,
			len: len as u16,
			};
		::core::mem::forget(self);
		rv
	}
}
impl ::core::ops::Drop for FillingHandle
{
	fn drop(&mut self) {
		// SAFE: Points within allocation
		unsafe {
			InterruptBuffers::drop_handle(self.ptr, self.cap)
		}
	}
}
pub struct FilledHandle
{
	ptr: *const u8,
	cap: u16,
	len: u16,
}
unsafe impl Sync for FilledHandle {}
unsafe impl Send for FilledHandle {}
impl ::usb_core::handle::RemoteBuffer for FilledHandle
{
	fn get(&self) -> &[u8] {
		// SAFE: Backing allocation not freed without checking reference counts
		// SAFE: Hardware isn't accessing self now (contract for `FillingHandle::filled`)
		unsafe {
			::core::slice::from_raw_parts(self.ptr, self.len as usize)
		}
	}
}
impl ::core::ops::Drop for FilledHandle
{
	fn drop(&mut self) {
		// SAFE: Points within allocation
		unsafe {
			InterruptBuffers::drop_handle(self.ptr, self.cap as usize)
		}
	}
}
