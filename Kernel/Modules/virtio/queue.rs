//
//
//
//!
use ::core::sync::atomic::{AtomicUsize,AtomicU16,Ordering};
use ::kernel::prelude::*;
use ::kernel::lib::mem::aref::{Aref/*,ArefBorrow*/};
use crate::interface::Interface;

pub struct Queue {
	idx: usize,
	size: usize,
	buffer: ::kernel::memory::virt::AllocHandle,
	descriptors_lock: ::kernel::sync::Mutex<()>,
	avail_ring_lock: ::kernel::sync::Mutex<()>,
	
	int_state: Aref<QueueIntState>,
}
pub struct QueueIntState {
	used_ring: *const UsedRing,
	last_seen_used: AtomicU16,
	interrupt_flag: ::kernel::sync::Semaphore,
	avail_ring_res: Vec<AtomicUsize>,
}
unsafe impl Send for QueueIntState { }
unsafe impl Sync for QueueIntState { }

pub enum Buffer<'a> {
	Read(&'a [u8]),
	Write(&'a mut [u8]),
}
impl<'a> Buffer<'a> {
	fn is_write(&self) -> bool {
		match self
		{
		&Buffer::Read(_) => false,
		&Buffer::Write(_) => true,
		}
	}
	fn as_slice(&self) -> &[u8] {
		match self
		{
		&Buffer::Read(v) => v,
		&Buffer::Write(ref v) => &**v,
		}
	}
}

pub const VRING_DESC_F_NEXT 	: u16 = 1;
pub const VRING_DESC_F_WRITE	: u16 = 2;
#[allow(dead_code)]
pub const VRING_DESC_F_INDIRECT	: u16 = 4;

#[repr(C)]
// sizeof = 16
pub struct VRingDesc {
	addr: u64,
	length: u32,
	flags: u16,
	next: u16,
}
const SIZEOF_VRING_DESC: usize = ::core::mem::size_of::<VRingDesc>();
#[repr(C)]
#[derive(Debug)]
struct AvailRing {
	flags: u16,
	idx: u16,
	ents: [u16],
	//used_event: u16,
}
#[repr(C)]
struct UsedRing {
	flags: u16,
	idx: u16,
	ents: [UsedElem],
	//avail_event: u16,
}
#[repr(C)]
struct UsedElem {
	id: u32,
	len: u32,
}

impl Queue
{
	fn get_first_size(count: usize) -> usize {
		let first =
			SIZEOF_VRING_DESC * count	// VRingDesc entries
			+ (2 + count + 1) * 2	// AvailRing
			;
		(first + 0xFFF) & !0xFFF
	}
	fn get_alloc_size(count: usize) -> usize {
		let second = 2*3 + 8 * count;	// UsedRing header (and footer) plus UsedElem entries
		Self::get_first_size(count) + ((second + 0xFFF) & !0xFFF)
	}

	pub fn new(idx: usize, count: usize) -> Queue
	{
		let n_pages = Self::get_alloc_size(count) / ::kernel::PAGE_SIZE;
		assert!(n_pages > 0);
		let buffer = ::kernel::memory::virt::alloc_dma(32+12, n_pages, "VirtIO").expect("TODO: Handle alloc failure VirtIO queue");
		let int_state = QueueIntState {
			used_ring: Self::used_ring(&buffer, count),
			last_seen_used: Default::default(),
			interrupt_flag: ::kernel::sync::Semaphore::new(0, count as isize),
			avail_ring_res: (0..count).map(|_| AtomicUsize::new(!0)).collect(),
			};
		Queue {
			idx: idx,
			size: count,
			buffer: buffer,
			descriptors_lock: Default::default(),
			avail_ring_lock: Default::default(),

			int_state: Aref::new(int_state),
			}
	}

	//pub fn get_int_state(&self) -> ArefBorrow<QueueIntState> {
	//	self.int_state.borrow()
	//}

	pub fn check_interrupt_fn(&self) -> impl Fn() {
		let is = self.int_state.borrow();
		let idx = self.idx;
		move || { is.check_interrupt(idx); }
	}

	pub fn phys_addr_desctab(&self) -> u64 {
		::kernel::memory::virt::get_phys(self.buffer.as_ref::<u8>(0)) as u64
	}
	pub fn phys_addr_avail(&self) -> u64 {
		::kernel::memory::virt::get_phys(&self.avail_ring().flags) as u64
	}
	pub fn phys_addr_used(&self) -> u64 {
		::kernel::memory::virt::get_phys(Self::used_ring(&self.buffer, self.size) as *const _) as u64
	}

	/// Send/receive to/from the device using caller-provided buffers
	///
	/// TODO: Could UAF if the request is leaked and `Buffer`'s backing goes out of scope
	pub fn send_buffers_blocking<'a, I: Interface>(&'a self, interface: &I, buffers: &mut [Buffer<'a>]) -> Result<usize,()> {
		assert!(buffers.len() > 0);

		// Allocate a descriptor for each buffer (backwards to build up linked list)
		let mut it = buffers.iter_mut().rev();
		let mut descriptor = self.allocate_descriptor(None, it.next().unwrap());
		for buf in it
		{
			descriptor = self.allocate_descriptor(Some(descriptor), buf);
		}

		// Add to the active queue
		self.dispatch_descriptor(interface, descriptor).busy_wait_for_completion()
	}

	/// Convert the queue into a stream (internally allocating a buffer and enqueing those items)
	pub fn into_stream<I: Interface>(self, int: &I, item_size: usize, buffer_len: usize, mut cb: impl FnMut(&[u8]))
	{
		// Allocate buffers
		let size = item_size * buffer_len;
		let mut data = vec![0u8; size];
		let mut slots = Vec::with_capacity(buffer_len);

		for i in 0 .. item_size
		{
			let d = self.allocate_descriptor(None, &mut Buffer::Write(&mut data[i*item_size..][..item_size]));
			self.avail_ring().push(d.idx);
			slots.push(d.idx);
		}
		int.notify_queue(self.idx);
		loop
		{
			for (i,idx) in slots.iter().copied().enumerate()
			{
				self.int_state.interrupt_flag.acquire();

				let len = self.int_state.avail_ring_res[idx as usize].swap(!0, Ordering::Relaxed);
				assert!(len != !0, "Interrupt flag set, but slot not populated");
				cb(&data[i*item_size..][..len]);
				self.avail_ring().push(idx);
				int.notify_queue(self.idx);
			}
		}
	}

	fn allocate_descriptor<'a>(&self, mut next: Option<DescriptorHandle<'a>>, buffer: &mut Buffer<'a>) -> DescriptorHandle<'a> {
		let write = buffer.is_write();
		for (phys, len) in ::kernel::memory::helpers::DMABuffer::new(buffer.as_slice(), 64).phys_ranges().rev()
		{
			next = Some( self.allocate_descriptor_raw(next, write, phys as u64, len as u32) );
		}
		next.unwrap()
	}
	fn allocate_descriptor_raw<'a>(&self, next: Option<DescriptorHandle<'a>>, write: bool, phys: u64, len: u32) -> DescriptorHandle<'a> {
		// TODO: Semaphore to ensure sufficient quantity
		// TODO: Use a "free chain" instead
		for (idx, desc) in self.descriptors().iter_mut().enumerate()
		{
			if desc.length == 0 {
				desc.addr = phys;
				desc.length = len;
				desc.flags = (if next.is_some() { VRING_DESC_F_NEXT } else { 0 }) | (if write { VRING_DESC_F_WRITE } else { 0 });
				desc.next = match next { Some(v) => v.idx, None => 0 };
				//log_trace!("Desc {}: {:#x}+{}", idx, phys, len);
				return DescriptorHandle { pd: ::core::marker::PhantomData, idx: idx as u16 };
			}
		}
		todo!("allocate_descriptor - out of descriptors");
	}
	fn dispatch_descriptor<'a, I: Interface>(&'a self, interface: &I, handle: DescriptorHandle<'a>) -> Request<'a> {
		
		self.avail_ring().push( handle.idx );
		// TODO: Memory barrier

		interface.notify_queue(self.idx);
		Request {
			queue: self,
			first_desc: handle.idx
			}
	}

	/// Return a lock handle to the "avaliable" ring buffer (the list of descriptors handed to the device)
	fn avail_ring(&self) -> LockedAvailRing {
		LockedAvailRing {
			_lh: self.avail_ring_lock.lock(),
			// SAFE: Locked
			ptr: unsafe { 
				let base_ptr: *mut u16 = self.buffer.as_int_mut(SIZEOF_VRING_DESC * self.size);
				// NOTE: Constructing an unsized struct pointer
				let ptr: &mut AvailRing = ::core::mem::transmute(::core::slice::from_raw_parts_mut(base_ptr, self.size));
				assert_eq!(&ptr.flags as *const _, base_ptr);
				ptr
				},
		}
	}
	/// Returns a fat pointer to the (device-managed) used ring buffer
	fn used_ring(buffer: &::kernel::memory::virt::AllocHandle, size: usize) -> *const UsedRing {
		// SAFE: Unaliased memory
		unsafe {
			let ptr: *const () = buffer.as_ref( Self::get_first_size(size) );
			let rv: &UsedRing = ::core::mem::transmute(::core::slice::from_raw_parts(ptr, size));
			assert_eq!(&rv.flags as *const _, ptr as *const u16);
			rv
		}
	}

	/// Return a lock handle to the descriptor table
	fn descriptors(&self) -> LockedDescriptors {
		LockedDescriptors {
			_lh: self.descriptors_lock.lock(),
			// SAFE: Locked access
			slice: unsafe { self.buffer.as_int_mut_slice(0, self.size) },
		}
	}
}

impl QueueIntState
{
	/// Check for changes in `used_ring` by the hardware
	pub fn check_interrupt(&self, queue_idx: usize) {
		// SAFE: Valid pointer (enforced by `Aref<QueueIntState>` stored within the `Queue`)
		while self.last_seen_used.load(Ordering::Relaxed) as u16 != unsafe { ::core::ptr::read_volatile(&(*self.used_ring).idx) } {
			let idx = self.last_seen_used.fetch_add(1, Ordering::Relaxed) as usize % self.avail_ring_res.len();
			// SAFE: Valid pointer (enforced by `Aref<QueueIntState>` stored within the `Queue`)
			let UsedElem { id, len }  = unsafe { ::core::ptr::read_volatile(&(*self.used_ring).ents[idx] ) };
			log_debug!("[INT queue {} {:p}] idx={}, ID={},len={}", queue_idx, self.used_ring, idx, id, len);

			self.avail_ring_res[id as usize].store(len as usize, Ordering::Release);
			self.interrupt_flag.release();
		}
	}

}

struct LockedAvailRing<'a> {
	_lh: ::kernel::sync::mutex::HeldMutex<'a, ()>,
	ptr: *mut AvailRing,
}
impl<'a> ::core::ops::Deref for LockedAvailRing<'a> {
	type Target = AvailRing;
	// SAFE: Locked access
	fn deref(&self) -> &AvailRing { unsafe { &*self.ptr }}
}
impl<'a> ::core::ops::DerefMut for LockedAvailRing<'a> {
	// SAFE: Locked access
	fn deref_mut(&mut self) -> &mut AvailRing { unsafe { &mut *self.ptr }}
}
impl AvailRing {
	fn push(&mut self, val: u16) {
		let count = self.ents.len();
		self.ents[self.idx as usize % count] = val;
		self.idx += 1;
		//log_debug!("AvailRing = {:?}", self);
	}
}

impl UsedRing {
	//pub fn used_event(&self) -> &u16 {
	//}
}

struct LockedDescriptors<'a> {
	_lh: ::kernel::sync::mutex::HeldMutex<'a, ()>,
	slice: *mut [VRingDesc],
}
impl<'a> ::core::ops::Deref for LockedDescriptors<'a> {
	type Target = [VRingDesc];
	// SAFE: Uniquely controlled at this point
	fn deref(&self) -> &[VRingDesc] { unsafe { &*self.slice } }
}
impl<'a> ::core::ops::DerefMut for LockedDescriptors<'a> {
	// SAFE: Uniquely controlled at this point
	fn deref_mut(&mut self) -> &mut [VRingDesc] { unsafe { &mut *self.slice } }
}

struct DescriptorHandle<'a>
{
	pd: ::core::marker::PhantomData<&'a [u8]>,
	idx: u16,
}

pub struct Request<'a>
{
	queue: &'a Queue,
	first_desc: u16,
}
impl<'a> Request<'a>
{
	pub fn busy_wait_for_completion(&self) -> Result<usize,()> {
		// XXX: HACK! No interrupts... yet
		while self.queue.int_state.avail_ring_res[self.first_desc as usize].load(Ordering::Relaxed) == !0 {
			self.queue.int_state.check_interrupt(self.queue.idx);
		}
		self.wait_for_completion()
	}
	pub fn wait_for_completion(&self) -> Result<usize,()> {
		self.queue.int_state.interrupt_flag.acquire();
		loop
		{
			let v = self.queue.int_state.avail_ring_res[self.first_desc as usize].swap(!0, Ordering::Acquire);
			if v != !0 {
				return Ok(v);
			}
			self.queue.int_state.interrupt_flag.release();
			// HACK: Yield here to prevent this wait from instantly waking
			::kernel::threads::yield_time();
			self.queue.int_state.interrupt_flag.acquire();
		}
	}
}
impl<'a> ::core::ops::Drop for Request<'a>
{
	fn drop(&mut self) {
		let mut d = self.queue.descriptors();
		let mut idx = self.first_desc as usize;
		loop
		{
			//log_trace!("Desc {}: Release", idx);
			d[idx].length = 0;
			if d[idx].flags & VRING_DESC_F_NEXT == 0 {
				break ;
			}
			idx = d[idx].next as usize;
		}
	}
}

