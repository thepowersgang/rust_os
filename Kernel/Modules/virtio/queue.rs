//
//
//
//!
use interface::Interface;

pub struct Queue {
	idx: usize,
	size: usize,
	buffer: ::kernel::memory::virt::AllocHandle,
	descriptors_lock: ::kernel::sync::Mutex<()>,
	avail_ring_lock: ::kernel::sync::Mutex<()>,
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
			}
	}

	pub fn phys_addr(&self) -> u64 {
		::kernel::memory::virt::get_phys(self.buffer.as_ref::<u8>(0)) as u64
	}

	pub fn send_buffers<'a, I: 'a + Interface>(&self, interface: &'a I, buffers: &mut [Buffer<'a>]) -> Request<'a, I> {
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
				return DescriptorHandle { pd: ::core::marker::PhantomData, idx: idx as u16 };
			}
		}
		todo!("allocate_descriptor - out of descriptors");
	}
	fn dispatch_descriptor<'a, I: 'a + Interface>(&self, interface: &'a I, handle: DescriptorHandle<'a>) -> Request<'a, I> {
		
		self.avail_ring().push( handle.idx );
		// TODO: Memory barrier

		interface.notify_queue(self.idx);
		Request {
			interface: interface,
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
				let ptr: &mut AvailRing = ::core::mem::transmute( (base_ptr, self.size) );
				assert_eq!(&ptr.flags as *const _, base_ptr);
				ptr
				},
		}
	}

	/// Return a lock handle to the descriptor table
	fn descriptors(&self) -> LockedDescriptors {
		LockedDescriptors {
			_lh: self.descriptors_lock.lock(),
			// SAFE: Becomes raw pointer instantly
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

struct Request<'a, I: 'a>
{
	interface: &'a I,
	first_desc: u16,
}
impl<'a, I: 'a + Interface> Request<'a, I>
{
	pub fn wait_for_completion(&self) {
		// TODO: Actually wait
	}
}

