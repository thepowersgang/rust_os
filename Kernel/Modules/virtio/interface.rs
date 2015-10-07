// 
//
//
//! Interface support
use kernel::prelude::*;
use kernel::device_manager::IOBinding;
use queue::Queue;

pub trait Interface
{
	fn new(io: IOBinding, irq: u32) -> Self;
	fn get_queue(&mut self, idx: usize, size: usize) -> Option<Queue>;

	fn notify_queue(&self, idx: usize);

	//fn cfg_read_8(&self, ofs: usize) -> u8;
	//fn cfg_read_16(&self, ofs: usize) -> u16;
	unsafe fn cfg_read_32(&self, ofs: usize) -> u32;
	//fn cfg_write_8(&self, ofs: usize) -> u8;
	//fn cfg_write_16(&self, ofs: usize) -> u16;
	unsafe fn cfg_write_32(&self, ofs: usize, v: u32);
}


pub struct Mmio {
	io: IOBinding,
	//irq: 
}
impl Interface for Mmio
{
	fn new(io: IOBinding, irq_gsi: u32) -> Self {
		Mmio {
			io: io,
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
				self.io.write_32(0x40, (queue.phys_addr() / 0x1000) as u32);
			}

			Some(queue)
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
