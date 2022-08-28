//!
//! 
//! 
use ::core::cell::UnsafeCell;
use ::core::convert::TryInto;
use crate::hw_structs;
use super::UnsafeArrayHandle;

fn set_bit(bitset: &mut [u8], idx: usize) {
	bitset[idx / 8] |= 1 << (idx % 8);
}
fn get_bit(bitset: &[u8], idx: usize) -> bool {
	bitset[idx / 8] & 1 << (idx % 8) != 0
}

/// Queue head pool
pub struct QhPool {
	alloc: UnsafeArrayHandle<hw_structs::QueueHead>,
	sem: ::kernel::sync::Semaphore,
	alloced: ::kernel::sync::Spinlock<[u8; (Self::COUNT + 7) / 8]>,
	released: ::kernel::sync::Spinlock<[u8; (Self::COUNT + 7) / 8]>,
	meta: [UnsafeCell<QhMeta>; Self::COUNT],

	/// Indicates that the QH "owned" by the hardware (it's a bug to access while this is set)
	running: ::kernel::sync::Spinlock<[u8; (Self::COUNT + 7) / 8]>,
	waiters: [::kernel::futures::flag::SingleFlag; Self::COUNT],
}
unsafe impl Sync for QhPool {}
unsafe impl Send for QhPool {}
impl QhPool {
	const COUNT: usize = ::kernel::PAGE_SIZE / ::core::mem::size_of::<hw_structs::QueueHead>();

	pub fn new() -> Result<Self,&'static str> {
		Ok(QhPool {
			alloc: UnsafeArrayHandle::new( ::kernel::memory::virt::alloc_dma(32, 1, module_path!())? ),
			sem: ::kernel::sync::Semaphore::new(Self::COUNT as isize, Self::COUNT as isize),
			alloced: ::kernel::sync::Spinlock::new( [0; (Self::COUNT + 7) / 8] ),
			released: ::kernel::sync::Spinlock::new( [0; (Self::COUNT + 7) / 8] ),
			meta: [(); Self::COUNT].map(|_| UnsafeCell::new(QhMeta { td: None })),
			running: ::kernel::sync::Spinlock::new( [0; (Self::COUNT + 7) / 8] ),
			waiters: [(); Self::COUNT].map(|_| Default::default()),
		})
	}
	pub fn alloc(&self, endpoint_id: u32, endpoint_ext: u32) -> QhHandle {
		let mut rv = self.alloc_raw(crate::hw_structs::QueueHead {
			hlink: 1,
			endpoint: endpoint_id,
			endpoint_ext: endpoint_ext,
			current_td: 0,
			overlay_link: 0,
			overlay_link2: 0,
			overlay_token: 0,
			overlay_pages: [0; 5]
			});
		self.get_meta_mut(&mut rv).td = None;
		rv
	}
	pub fn alloc_raw(&self, v: hw_structs::QueueHead) -> QhHandle {
		self.sem.acquire();
		let mut lh = self.alloced.lock();
		match super::set_first_zero_bit(&mut lh[..], 0)
		{
		Some(i) => {
			let mut rv = QhHandle(i);
			*self.get_data_mut(&mut rv) = v;
			self.waiters[rv.0].reset();
			rv
			},
		None => panic!("All slots are used, but semaphore was acquired"),
		}
	}
	pub fn release(&self, handle: QhHandle) {
		log_debug!("QhPool::release({:?})", handle);
		let idx = handle.0;
		::core::mem::forget(handle);
		set_bit(&mut self.released.lock()[..], idx);
	}

	/// Assigns a TD to the queue, and starts it executing
	pub fn assign_td(&self, handle: &mut QhHandle, td_pool: &super::TdPool, first_td: super::TdHandle) {
		let d = self.get_data_mut(handle);
		d.current_td = td_pool.get_phys(&first_td)/*| crate::hw_structs::QH*/;
		self.get_meta_mut(handle).td = Some(first_td);
		// SAFE: Correct ordering of operations to ensure that the hardware behaves predictably.
		unsafe {
			self.mark_running(handle);
			// Set the new link value, and clear any set `HALT` bit
			// - As soon as both are met, the controller will start processing (next time it loops around)
			::core::ptr::write_volatile(&mut d.overlay_link, d.current_td);
			::core::ptr::write_volatile(&mut d.overlay_token, 0);
		}
		log_debug!("QH {:?} {:#x} {:?}", handle, ::kernel::memory::virt::get_phys(d), d);
	}
	/// Remove the current TD from a completed/idle QH
	pub fn clear_td(&self, handle: &mut QhHandle) -> Option<super::TdHandle> {
		self.assert_not_running(handle, "clear_td");
		self.get_data_mut(handle).current_td = 0;
		self.get_meta_mut(handle).td.take()
	}

	fn get_idx_from_phys(&self, addr: u32) -> usize {
		let phys0: u32 = self.alloc.get_phys(0).try_into().unwrap();
		assert!(addr >= phys0, "{:#x} is not a valid QH address for this pool", addr);
		assert!(addr < phys0 + 0x1000, "{:#x} is not a valid QH address for this pool", addr);
		assert!(addr % ::core::mem::size_of::<hw_structs::QueueHead>() as u32 == 0, "{:#x} is not a valid QH address for this pool", addr);
		let idx = (addr - phys0) / ::core::mem::size_of::<hw_structs::QueueHead>() as u32;
		let idx = idx as usize;
		assert!(idx < Self::COUNT, "{:#x} is not a valid QH address for this pool", addr);
		idx
	}

	pub fn get_phys(&self, h: &QhHandle) -> u32 {
		self.alloc.get_phys(h.0).try_into().unwrap()
	}
	pub fn get_data(&'_ self, h: &'_ QhHandle) -> &'_ hw_structs::QueueHead {
		self.assert_not_running(h, "get_data");
		// SAFE: The handle is owned
		unsafe { self.alloc.get(h.0) }
	}
	pub fn get_data_mut(&'_ self, h: &'_ mut QhHandle) -> &'_ mut hw_structs::QueueHead {
		self.assert_not_running(h, "get_data_mut");
		// SAFE: The handle is owned
		unsafe { self.alloc.get_mut(h.0) }
	}
	/*pub*/ fn get_meta_mut(&'_ self, h: &'_ mut QhHandle) -> &'_ mut QhMeta {
		self.assert_not_running(h, "get_meta_mut");
		// SAFE: Mutable access to the handle implies mutable access to the data
		unsafe { &mut *self.meta[h.0].get() }
	}


	/// UNSAFE: Only call this once the controller is no longer accessing any released entries
	/// (i.e. the queue is stopped, or the queue has been advanced)
	pub unsafe fn trigger_gc(&self) {
		log_trace!("QhPool::trigger_gc");
		// Iterate all entries, look for one marked as released
		let mut lh_release = self.released.lock();
		let mut lh_alloc = self.alloced.lock();
		for idx in 0 .. Self::COUNT {
			if super::get_and_clear_bit(&mut lh_release[..], idx) {
				assert!(super::get_and_clear_bit(&mut lh_alloc[..], idx));
				self.sem.release();
			}
		}
	}
	/// Remove a QH from a list
	/// 
	/// UNSAFE: Caller must ensure that the entry is on the queue/loop started by `root`
	pub unsafe fn remove_from_list(&self, root: &mut QhHandle, ent: &QhHandle) {
		let mut cur_idx = root.0;
		loop {
			let hlink = self.alloc.get(cur_idx).hlink;
			if hlink == 0 {
				// Not found?
				return ;
			}
			let next = self.get_idx_from_phys(hlink & !0xF);
			if next == ent.0 {
				// Found it!
				log_debug!("QhPool::remove_from_list: Stich {cur_idx} to {next}, removing {ent}", ent=ent.0);
				self.alloc.get_mut(cur_idx).hlink = self.alloc.get(next).hlink;
				return ;
			}
			cur_idx = next;
			if cur_idx == root.0 {
				// Uh-oh, we've looped. Error?
				return ;
			}
		}
	}

	#[track_caller]
	fn assert_not_running(&self, h: &QhHandle, fcn: &str) {
		assert!( !get_bit(&self.running.lock()[..], h.0), "TdPool::{}({:?}) with running TD", fcn, h );
	}
	/// Marks a QH as now controlled by the hardware - must be called for `wait` to work properly
	/// UNSAFE: Callers cannot access the data until `wait` returns
	unsafe fn mark_running(&self, handle: &mut QhHandle) {
		let mut lh = self.running.lock();
		assert!( !get_bit(&lh[..], handle.0), "mark_running({:?}) on already running QH", handle);
		set_bit(&mut lh[..], handle.0);
	}

	/// Check completion on any running task
	pub fn check_any_complete(&self) {
		let mut lh = self.running.lock();
		for idx in 0 .. Self::COUNT
		{
			// Is the queue running?
			if get_bit(&lh[..], idx)
			{
				// SAFE: Since the bit in `running` is set, hardware (and this logic) owns the QH
				let (token, link) = unsafe {
					(
						::core::ptr::read(::core::ptr::addr_of!((*self.alloc.get(idx)).overlay_token)),
						::core::ptr::read(::core::ptr::addr_of!((*self.alloc.get(idx)).overlay_link)),
						)
					};
				// If the overlay's active bit is zero and there's nothing in `link`, the queue is now complete
				if token & hw_structs::QTD_TOKEN_STS_ACTIVE == 0 && link & 1 == 1
				{
					log_debug!("check_any_complete: QhHandle({}) complete (token = {:#x}, link = {:#x})", idx, token, link);
					// Clear the `running` bit (it should be set, because we checked above)
					assert!( super::get_and_clear_bit(&mut lh[..], idx), "How did this bit get unset? We're locked" );

					// NOTE: Drop and re-acquire the lock so it doesn't overlap with the mutex within the waiter
					drop(lh);
					self.waiters[idx].trigger();
					lh = self.running.lock();
				}
			}
		}
	}

	/// Async wait for the QH to be removed from the async queue
	pub async fn wait(&self, h: &mut QhHandle) {
		assert!( get_bit(&self.running.lock()[..], h.0), "TdPool::wait({:?}) with non-running TD", h );
		self.waiters[h.0].wait().await
	}


	// --- Periodic List ---
	pub unsafe fn get_next_and_period(&self, addr: u32) -> (u32, usize)
	{
		let idx = self.get_idx_from_phys(addr);
		let next = ::core::ptr::read( ::core::ptr::addr_of!( (*self.alloc.get_raw(idx)).hlink) );
		(next, 1)
	}

	pub unsafe fn set_next(&self, ent_addr: u32, hlink: u32) {
		let idx = self.get_idx_from_phys(ent_addr);
		::core::ptr::write( ::core::ptr::addr_of_mut!((*self.alloc.get_raw(idx)).hlink), hlink );
	}

}
#[derive(Debug)]
pub struct QhHandle(usize);
impl ::core::ops::Drop for QhHandle
{
	fn drop(&mut self) {
		log_error!("BUG: {:?} dropped, should be released back to the pool", self);
	}
}
struct QhMeta {
	/// The first item in the linked list of owned TDs
	td: Option<super::TdHandle>,
}