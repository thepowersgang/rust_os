//
//
//
//!
use kernel::prelude::*;
use interface::Interface;
use core::sync::atomic::{AtomicUsize,Ordering};

pub struct Queue {
	idx: usize,
	size: usize,
	buffer: ::kernel::memory::virt::AllocHandle,
	descriptors_lock: ::kernel::sync::Mutex<()>,
	avail_ring_lock: ::kernel::sync::Mutex<()>,

	last_seen_used: AtomicUsize,
	interrupt_flag: ::kernel::sync::Semaphore,
	avail_ring_res: Vec<AtomicUsize>,
}

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
pub struct VRingDesc {
	addr: u64,
	length: u32,
	flags: u16,
	next: u16,
}
#[repr(C)]
#[derive(Debug)]
struct AvailRing {
	flags: u16,
	idx: u16,
	ents: [u16],
	//used_event: u16,
}
#[repr(C)]
#[derive(Debug)]
struct UsedRing {
	flags: u16,
	idx: u16,
	ents: [UsedElem],
	//avail_event: u16,
}
#[repr(C)]
#[derive(Debug)]
struct UsedElem {
	id: u32,
	len: u32,
}

impl Queue
{
	fn get_first_size(count: usize) -> usize {
		let first = 16 * count + (2 + count + 1) * 2;
		(first + 0xFFF) & !0xFFF
	}
	fn get_alloc_size(count: usize) -> usize {
		let second = 2*3 + 8 * count;
		Self::get_first_size(count) + ((second + 0xFFF) & !0xFFF)
	}

	pub fn new(idx: usize, count: usize) -> Queue
	{
		let n_pages = Self::get_alloc_size(count) / ::kernel::PAGE_SIZE;
		assert!(n_pages > 0);
		Queue {
			idx: idx,
			size: count,
			buffer: ::kernel::memory::virt::alloc_dma(32+12, n_pages, "VirtIO").expect("TODO: Handle alloc failure VirtIO queue"),
			descriptors_lock: Default::default(),
			avail_ring_lock: Default::default(),

			last_seen_used: AtomicUsize::new(0),
			interrupt_flag: ::kernel::sync::Semaphore::new(0, count as isize),
			avail_ring_res: (0..count).map(|_| AtomicUsize::new(0)).collect(),
			}
	}

	pub fn check_interrupt(&self) {
		while self.last_seen_used.load(Ordering::Relaxed) as u16 != self.used_ring().idx {
			let idx = self.last_seen_used.fetch_add(1, Ordering::Relaxed) & 0xFFFF;
			log_debug!("idx={}, desc={:?}", idx, self.used_ring().ents[idx]);
			let UsedElem { id, len } = self.used_ring().ents[idx];

			assert!(len > 0);
			self.avail_ring_res[id as usize].store(len as usize, Ordering::Release);
			self.interrupt_flag.release();
		}
	}

	pub fn phys_addr(&self) -> u64 {
		::kernel::memory::virt::get_phys(self.buffer.as_ref::<u8>(0)) as u64
	}

	pub fn send_buffers<'a, I: Interface>(&'a self, interface: &I, buffers: &mut [Buffer<'a>]) -> Request<'a> {
		assert!(buffers.len() > 0);

		// Allocate a descriptor for each buffer (backwards to build up linked list)
		let mut it = buffers.iter_mut().rev();
		let mut descriptor = self.allocate_descriptor(None, it.next().unwrap());
		for buf in it
		{
			descriptor = self.allocate_descriptor(Some(descriptor), buf);
		}

		// Add to the active queue
		self.dispatch_descriptor(interface, descriptor)
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
				log_trace!("Desc {}: {:#x}+{}", idx, phys, len);
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
				let base_ptr: *const u16 = self.buffer.as_int_mut(16 * self.size);
				// NOTE: Constructing an unsized struct pointer
				let ptr: &mut AvailRing = ::core::mem::transmute( (base_ptr, self.size) );
				assert_eq!(&ptr.flags as *const _, base_ptr);
				ptr
				},
		}
	}
	fn used_ring(&self) -> &UsedRing {
		// SAFE: Unaliased memory
		unsafe {
			let ptr: *const () = self.buffer.as_ref( Self::get_first_size(self.size) );
			let rv: &UsedRing = ::core::mem::transmute(::core::slice::from_raw_parts(ptr, self.size));
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
	pub fn wait_for_completion(&self) -> Result<usize,()> {
		// XXX: HACK! No interrupts... yet
		while self.queue.avail_ring_res[self.first_desc as usize].load(Ordering::Relaxed) == 0 {
			self.queue.check_interrupt();
		}
		self.queue.interrupt_flag.acquire();
		loop
		{
			let v = self.queue.avail_ring_res[self.first_desc as usize].swap(0, Ordering::Acquire);
			if v != 0 {
				return Ok(v);
			}
			self.queue.interrupt_flag.release();
			// HACK: Yield here to prevent this wait from instantly waking
			::kernel::threads::yield_time();
			self.queue.interrupt_flag.acquire();
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
			log_trace!("- Desc {}: Release", idx);
			d[idx].length = 0;
			if d[idx].flags & VRING_DESC_F_NEXT == 0 {
				break ;
			}
			idx = d[idx].next as usize;
		}
	}
}

