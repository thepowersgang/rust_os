// "Tifflin" Kernel - VirtIO Driver
// - By John Hodge (thePowersGang)
//
// virtio/interface.rs
//! VirtualIO Interface (bus binding)
use kernel::prelude::*;
use kernel::device_manager::IOBinding;
use crate::queue::Queue;

/// A virtio interface (PCI or MMIO)
pub trait Interface
{
	fn bind_interrupt<Cb>(&mut self, cb: Cb) where Cb: FnMut() + Send + 'static;

	fn negotiate_features(&mut self, supported: u32) -> u32;
	fn get_queue(&mut self, idx: usize, size: usize) -> Option<Queue>;
	fn set_driver_ok(&mut self);

	fn notify_queue(&self, idx: usize);

	unsafe fn cfg_read_8(&self, ofs: usize) -> u8;
	//fn cfg_read_16(&self, ofs: usize) -> u16;
	unsafe fn cfg_read_32(&self, ofs: usize) -> u32;
	unsafe fn cfg_write_8(&self, ofs: usize, v: u8);
	//fn cfg_write_16(&self, ofs: usize) -> u16;
	unsafe fn cfg_write_32(&self, ofs: usize, v: u32);
}

pub struct PciRegions {
	// TODO: Avoid duplicated IO bindings (use references into a pool)
	pub common: IOBinding,
	pub notify: IOBinding,
	pub notify_off_mult: u32,
	/// Interrupt Status Register
	pub isr: IOBinding,
	pub dev_cfg: IOBinding,
}
#[repr(usize)]
#[allow(dead_code,non_camel_case_types)]
enum PciCommonReg {
	device_feature_select = 0x0,	// u32 RW
	device_feature        = 0x4,	// u32 RO
	driver_feature_select = 0x8,	// u32 RW
	driver_feature        = 0xC,	// u32 RW
	msix_config           = 0x10,	// u16 RW
	num_queues            = 0x12,	// u16 RO
	device_status         = 0x14,	// u8 RW
	config_generation     = 0x15,	// u8 RO
	queue_select          = 0x16,	// u16
	queue_size            = 0x18,	// u16
	queue_msix_vector     = 0x1a,	// u16
	queue_enable          = 0x1c,	// u16
	queue_notify_off      = 0x1e,	// u16
	queue_desc            = 0x20,	// u64
	queue_avail           = 0x28,	// u64
	queue_used            = 0x30,	// u64
}
pub struct Pci {
	bars: PciRegions,

	irq_gsi: u32,
	#[allow(dead_code)]
	irq_handle: Option<::kernel::irqs::ObjectHandle>,

	queue_notify_offsets: Vec<u32>,
}
impl Pci
{
	pub fn new(io: PciRegions, irq_gsi: u32) -> Self {
		// SAFE: Unique access, read-only
		let nqueues = unsafe { io.common.read_16(PciCommonReg::num_queues as usize) as usize };
		let queue_notify_offsets = (0 .. nqueues).map(|q| {
			// SAFE: Unique access, no memory
			unsafe {
				io.common.write_16(PciCommonReg::queue_select as usize, q as u16);
				io.common.read_16(PciCommonReg::queue_notify_off as usize) as u32 * io.notify_off_mult
			}
			}).collect();
		log_debug!("nqueues = {}, queue_notify_offsets={:?}", nqueues, queue_notify_offsets);

		let mut rv = Pci {
			bars: io,
			irq_gsi: irq_gsi,
			irq_handle: None,
			queue_notify_offsets: queue_notify_offsets,
			};

		// SAFE: Unique access
		unsafe {
			rv.set_device_status(0x0);	// Reset
			rv.set_device_status(0x1);	// Acknowledge
		}
		rv
	}
	unsafe fn set_device_status(&mut self, val: u8) {
		self.bars.common.write_8(0x10, val);	// device_status
	}
}
impl Interface for Pci
{
	fn bind_interrupt<Cb>(&mut self, mut cb: Cb) where Cb: FnMut() + Send + 'static {
		self.irq_handle = Some( ::kernel::irqs::bind_object(self.irq_gsi, Box::new(move || { cb(); true })) );
	}

	fn negotiate_features(&mut self, supported: u32) -> u32 {
		// SAFE: Unique access
		unsafe {
			let dev_supported = self.bars.common.read_32(PciCommonReg::device_feature as usize);
			let common = dev_supported & supported;
			self.bars.common.write_32(PciCommonReg::device_feature_select as usize, common);
			common
		}
	}

	fn get_queue(&mut self, idx: usize, size: usize) -> Option<Queue> {
		if idx >= self.queue_notify_offsets.len() {
			log_error!("Request for queue {} is out of valid range {}", idx, self.queue_notify_offsets.len());
			return None
		}
		// SAFE: Unique access, so no race possible
		unsafe {
			self.bars.common.write_16(PciCommonReg::queue_select as usize, idx as u16);
		}
		// SAFE: Unique access
		let max_size = unsafe { self.bars.common.read_16(PciCommonReg::queue_size as usize) as usize };
		if max_size == 0 {
			None
		}
		else {
			let size = if size == 0 || size > max_size { max_size } else { size };
			let queue = Queue::new(idx, size);

			// SAFE: Unique access, so no race possible
			unsafe {
				self.bars.common.write_32(PciCommonReg::queue_size as usize, size as u32);	// queue_size
				let addr = queue.phys_addr_desctab();
				self.bars.common.write_32(PciCommonReg::queue_desc as usize, addr as u32);
				self.bars.common.write_32(PciCommonReg::queue_desc as usize + 4, (addr >> 32) as u32);
				let addr = queue.phys_addr_avail();
				self.bars.common.write_32(PciCommonReg::queue_avail as usize, addr as u32);
				self.bars.common.write_32(PciCommonReg::queue_avail as usize + 4, (addr >> 32) as u32);
				let addr = queue.phys_addr_used();
				self.bars.common.write_32(PciCommonReg::queue_used as usize, addr as u32);
				self.bars.common.write_32(PciCommonReg::queue_used as usize + 4, (addr >> 32) as u32);

				self.bars.common.write_16(PciCommonReg::queue_enable as usize, 1);
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
		log_trace!("PCI: notify_queue({})", idx);
		// SAFE: Atomic write
		unsafe {
			self.bars.notify.write_16(self.queue_notify_offsets[idx] as usize, idx as u16)
		}
	}

	unsafe fn cfg_read_8(&self, ofs: usize) -> u8 {
		assert!(ofs + 1 <= 0x100);
		self.bars.dev_cfg.read_8(ofs)
	}
	unsafe fn cfg_read_32(&self, ofs: usize) -> u32 {
		assert!(ofs + 4 <= 0x100);
		self.bars.dev_cfg.read_32(ofs)
	}
	unsafe fn cfg_write_8(&self, ofs: usize, v: u8) {
		assert!(ofs + 1 <= 0x100);
		self.bars.dev_cfg.write_8(ofs, v);
	}
	unsafe fn cfg_write_32(&self, ofs: usize, v: u32) {
		assert!(ofs + 4 <= 0x100);
		self.bars.dev_cfg.write_32(ofs, v);
	}
}

/// Memory-Mapped IO binding
pub struct Mmio {
	// Note: First so it gets dropped first (before the IO binding it might be referencing)
	#[allow(dead_code)]
	irq_handle: Option<::kernel::irqs::ObjectHandle>,
	io: IOBinding,
	irq_gsi: u32,
}
impl Mmio
{
	pub fn new(io: IOBinding, irq_gsi: u32) -> Self {
		let mut rv = Mmio {
			io: io,
			irq_gsi: irq_gsi,
			irq_handle: None,
			};
		// SAFE: Unique access
		unsafe {
			rv.set_device_status(0x0);	// Reset
			rv.set_device_status(0x1);	// Acknowledge
			rv.io.write_32(0x28, ::kernel::PAGE_SIZE as u32);	// "GuestPageSize"
		}
		rv
	}
	unsafe fn set_device_status(&mut self, val: u32) {
		self.io.write_32(0x70, val);
	}
}
impl Interface for Mmio
{
	fn bind_interrupt<Cb>(&mut self, mut cb: Cb) where Cb: FnMut() + Send + 'static {
		struct IntIo(*mut u32);
		unsafe impl Send for IntIo {}
		impl IntIo {
			unsafe fn status(&self) -> u32 { ::core::ptr::read_volatile(self.0.offset(0)) }
			unsafe fn ack(&mut self, v: u32) { ::core::ptr::write_volatile(self.0.offset(1), v); }
		}
		let mut io = IntIo(match self.io
			{
			IOBinding::Memory(ref ah) => ah.as_mut_ptr::<[u32; 2]>(0x60) as *mut _,	// 0x60=InterruptStatus, 0x64=InterruptACK
			_ => panic!(""),
			});
		// SAFE: Since this callback is tied to the interrupt handle, and `irq_handle` is never cleared - the IO binding will be maintained
		let int_handler = move || unsafe {
			let v = io.status();
			if v & 1 != 0 {
				// Queue update
				cb();
			}
			if v & 2 != 0 {
				// Configuration change
			}
			io.ack(v);
			v != 0
			};
		self.irq_handle = Some( ::kernel::irqs::bind_object(self.irq_gsi, Box::new(int_handler)) );
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
				// TODO: This is for the legacy spec
				let page = queue.phys_addr_desctab() / ::kernel::PAGE_SIZE as u64;
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

	unsafe fn cfg_read_8(&self, ofs: usize) -> u8 {
		assert!(ofs + 1 <= 0x100);
		self.io.read_8(0x100 + ofs)
	}
	unsafe fn cfg_read_32(&self, ofs: usize) -> u32 {
		assert!(ofs + 4 <= 0x100);
		self.io.read_32(0x100 + ofs)
	}
	unsafe fn cfg_write_8(&self, ofs: usize, v: u8) {
		assert!(ofs + 1 <= 0x100);
		self.io.write_8(0x100 + ofs, v);
	}
	unsafe fn cfg_write_32(&self, ofs: usize, v: u32) {
		assert!(ofs + 4 <= 0x100);
		self.io.write_32(0x100 + ofs, v);
	}
}
