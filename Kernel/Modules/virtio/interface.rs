// "Tifflin" Kernel - VirtIO Driver
// - By John Hodge (thePowersGang)
//
// virtio/interface.rs
//! VirtualIO Interface (bus binding)
use kernel::prelude::*;
use kernel::device_manager::IOBinding;
use queue::Queue;

pub trait Interface
{
	fn new(io: IOBinding, irq: u32) -> Self;

	fn bind_interrupt(&mut self, cb: Box<FnMut()->bool + Send + 'static>);

	fn negotiate_features(&mut self, supported: u32) -> u32;
	fn get_queue(&mut self, idx: usize, size: usize) -> Option<Queue>;
	fn set_driver_ok(&mut self);

	fn notify_queue(&self, idx: usize);

	//fn cfg_read_8(&self, ofs: usize) -> u8;
	//fn cfg_read_16(&self, ofs: usize) -> u16;
	unsafe fn cfg_read_32(&self, ofs: usize) -> u32;
	//fn cfg_write_8(&self, ofs: usize) -> u8;
	//fn cfg_write_16(&self, ofs: usize) -> u16;
	unsafe fn cfg_write_32(&self, ofs: usize, v: u32);
}


/// Memory-Mapped IO binding
pub struct Mmio {
	io: IOBinding,
	irq_gsi: u32,
	irq_handle: Option<::kernel::irqs::ObjectHandle>,
}
impl Interface for Mmio
{
	fn new(io: IOBinding, irq_gsi: u32) -> Self {
		let mut rv = Mmio {
			io: io,
			irq_gsi: irq_gsi,
			irq_handle: None,
			};
		// SAFE: Unique access
		unsafe {
			rv.set_device_status(0x0);	// Reset
			rv.set_device_status(0x1);	// Acknowledge
			rv.io.write_32(0x28, 0x1000);	// "GuestPageSize"
		}
		rv
	}

	fn bind_interrupt(&mut self, cb: Box<FnMut()->bool + Send + 'static>) {
		self.irq_handle = Some( ::kernel::irqs::bind_object(self.irq_gsi, cb) );
	}

	fn negotiate_features(&mut self, supported: u32) -> u32 {
		// SAFE: Unique access
		unsafe {
			let dev_supported = self.io.read_32(0x10);
			let common = dev_supported & supported;
			self.io.write_32(0x20, common);
			common
		}
	}

	fn get_queue(&mut self, idx: usize, size: usize) -> Option<Queue> {
		// SAFE: Unique access, so no race possible
		unsafe {
			self.io.write_32(0x30, idx as u32);
		}
		// SAFE: Unique access
		let max_size = unsafe { self.io.read_32(0x34) as usize };
		if max_size == 0 {
			None
		}
		else {
			let size = if size == 0 || size > max_size { max_size } else { size };
			let queue = Queue::new(idx, size);

			// SAFE: Unique access, so no race possible
			unsafe {
				self.io.write_32(0x38, size as u32);
				//self.io.write_32(0x3C, );	// QueueAlign - TODO: What value to use here
				let page = queue.phys_addr() / 0x1000;
				log_debug!("size = {}, page={:#x}", size, page);
				self.io.write_32(0x40, page as u32);
			}

			Some(queue)
		}
	}

	fn set_driver_ok(&mut self) {
		// SAFE: Unique access
		unsafe {
			self.set_device_status(0x4);
		}
	}
	
	fn notify_queue(&self, idx: usize) {
		// SAFE: Atomic write
		unsafe {
			self.io.write_32(0x50, idx as u32)
		}
	}

	unsafe fn cfg_read_32(&self, ofs: usize) -> u32 {
		assert!(ofs + 4 <= 0x100);
		self.io.read_32(0x100 + ofs)
	}
	unsafe fn cfg_write_32(&self, ofs: usize, v: u32) {
		assert!(ofs + 4 <= 0x100);
		self.io.write_32(0x100 + ofs, v);
	}
}
impl Mmio {
	unsafe fn set_device_status(&mut self, val: u32) {
			self.io.write_32(0x70, val);
	}
}
