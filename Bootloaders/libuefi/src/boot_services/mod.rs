//! Boot services
//!
//! These are functions that are only usable while the firmware has control of the system (i.e.
//! before `exit_boot_services` has been called
use super::{Void,Status,Guid,Handle};
use super::{PhysicalAddress,VirtualAddress};

pub mod protocols;

/// Task Priority Level
pub type Tpl = usize;

/// Raw type aliases
pub mod raw
{
	/// Event handle (raw)
	pub type Event = *mut ::Void;
}

//#[repr(C)]
pub type PoolPointer<T> = *mut T;

/// Wrapped `Event` handle
pub struct Event(raw::Event);

#[repr(C)]
pub struct BootServices
{
	pub hdr: super::TableHeader,
	
	// Task Priority
	pub raise_tpl: efi_fcn! { fn(Tpl) -> Tpl },
	pub restore_tpl: efi_fcn! { fn(Tpl) -> Tpl },

	// Memory
	pub allocate_pages: efi_fcn!{ fn(AllocateType, MemoryType, /*no_pages:*/ usize, &mut PhysicalAddress) -> Status },
	pub free_pages: efi_fcn! { fn(PhysicalAddress, usize) -> Status },
	pub get_memory_map: efi_fcn!{ fn(&mut usize, *mut MemoryDescriptor, /*map_key:*/ &mut usize, /*descriptor_size:*/ &mut usize, /*descriptor_version:*/ &mut u32) -> Status },
	pub allocate_pool: efi_fcn!{ fn(MemoryType, usize, &mut PoolPointer<Void>) -> Status },
	pub free_pool: efi_fcn!{ fn(*mut Void) -> Status },

	// Timing and events
	pub create_event: efi_fcn!{ fn(u32, /*notify_tpl:*/ Tpl, /*notify_function:*/ Option<EventNotifyFcn>, *mut Void, &mut raw::Event) -> Status },
	pub set_timer: efi_fcn!{ fn(raw::Event, TimerDelay, u64) -> Status },
	pub wait_for_event: efi_fcn!{ fn(usize, /*events:*/ *const raw::Event, &mut usize) -> Status },
	pub signal_event: efi_fcn!{ fn(raw::Event) -> Status },
	pub close_event: efi_fcn!{ fn(raw::Event) -> Status },
	pub check_event: efi_fcn!{ fn(raw::Event) -> Status },

	// Protocol handler functions
	pub install_protocol_interface: efi_fcn!{ fn(&mut Handle, &Guid, InterfaceType, *mut Void) -> Status },
	pub reinstall_protocol_interface: efi_fcn!{ fn(Handle, &Guid, /*old:*/ *mut Void, /*new:*/ *mut Void) -> Status },
	pub uninstall_protocol_interface: efi_fcn!{ fn(Handle, &Guid, *mut Void) -> Status },
	pub handle_protocol: efi_fcn!{ fn(Handle, &Guid, &mut *mut Void) -> Status },
	pub pc_handle_protocol: efi_fcn!{ fn(Handle, &Guid, &mut *mut Void) -> Status },
	pub register_protocol_notify: efi_fcn!{ fn(&Guid, Event, &mut *mut Void) -> Status },
	pub locate_handle: efi_fcn!{ fn(LocateSearchType, Option<&Guid>, *mut Void, &mut usize, *mut Handle) -> Status },
	pub locate_device_path: efi_fcn!{ fn(&Guid, &mut *mut DevicePath, &mut Handle) -> Status },
	pub install_configuration_table: efi_fcn!{ fn(&Guid, *mut Void) -> Status },

	// Image functions
	pub load_image: efi_fcn!{ fn(bool, Handle, &DevicePath, *mut Void, usize, &mut Handle) -> Status },
	pub start_image: efi_fcn!{ fn(Handle, &mut usize, Option<&mut *mut u16>) -> Status },
	pub exit: efi_fcn!{ fn(Handle, Status, usize, *const u16) -> Status },
	pub unload_image: efi_fcn!{ fn(Handle) -> Status },
	pub exit_boot_services: efi_fcn!{ fn(Handle, /*map_key:*/ usize) -> Status },
	
	// Misc functions
	pub get_next_monotonic_count: efi_fcn!{ fn() -> Status },
	pub stall: efi_fcn!{ fn() -> Status },
	pub set_watchdog_timer: efi_fcn!{ fn() -> Status },

	// DriverSupport Services
	pub connect_controller: efi_fcn!{ fn() -> Status },
	pub disconnect_controller: efi_fcn!{ fn() -> Status },

	// Open/Close Protocol Services
	pub open_protocol: efi_fcn!{ fn(Handle, &Guid, Option<&mut *mut Void>, Handle, Handle, u32) -> Status },
	pub close_protocol: efi_fcn!{ fn(Handle, &Guid, Handle, Handle) -> Status },
	pub open_protocol_information: efi_fcn!{ fn() -> Status },

	// Library Services
	pub protocols_per_handle: efi_fcn!{ fn(Handle, &mut PoolPointer<&Guid>, &mut usize) -> Status },
	pub locate_handle_buffer: efi_fcn!{ fn(LocateSearchType, Option<&Guid>, *const Void, &mut usize, &mut *mut Handle) -> Status },
	pub locate_protocol: efi_fcn!{ fn() -> Status },
	pub install_multiple_protocol_interfaces: efi_fcn!{ fn() -> Status },
	pub uninstall_multiple_protocol_interfaces: efi_fcn!{ fn() -> Status },

	// CRC
	pub calculate_crc32: efi_fcn!{ fn() -> Status },

	// Misc Services
	pub copy_mem: efi_fcn!{ fn() -> Status },
	pub set_mem: efi_fcn!{ fn() -> Status },
	pub create_event_ex: efi_fcn!{ fn(u32, /*notify_tpl:*/ Tpl, /*notify_function:*/ Option<EventNotifyFcn>, *mut Void, &Guid, &mut raw::Event) -> Status },
}

/// Event, Timer, and Task Priority Services
impl BootServices
{
	/// Create a new signalable event.
	pub fn create_event(&self, ty: u32, notify_tpl: Tpl, notify_fcn: Option<(EventNotifyFcn,*mut Void)>) -> Result<Event,Status>
	{
		let (nf, nc) = match notify_fcn
			{
			Some(v) => (Some(v.0), v.1),
			None => (None, ::core::ptr::null_mut()),
			};
		let mut rv = 0 as raw::Event;	 // `Event` is a pointer
		// SAFE: Passed function pointer is inherently 'static, and the pointer isn't dereferenced by the environment
		(unsafe { (self.create_event)(ty, notify_tpl, nf, nc, &mut rv) })
			.err_or(Event(rv))
	}

	/// Create a new signalable event attached to a group
	pub fn create_event_for_group(&self, ty: u32, notify_tpl: Tpl, notify_fcn: Option<(EventNotifyFcn,*mut Void)>, group: Guid) -> Result<Event, Status>
	{
		let (nf, nc) = match notify_fcn
			{
			Some(v) => (Some(v.0), v.1),
			None => (None, ::core::ptr::null_mut()),
			};
		let mut rv = 0 as raw::Event;	 // `Event` is a pointer
		// SAFE: Passed function pointer is inherently 'static, and the pointer isn't dereferenced by the environment
		(unsafe { (self.create_event_ex)(ty, notify_tpl, nf, nc, &group, &mut rv) })
			.err_or(Event(rv))
	}

	/// Close (destroy) an event
	pub fn close_event(&self, ev: Event) -> Status {
		// SAFE: No memory unsafety because the wrapped handle can only have come from a successful `create_event*`
		(unsafe { (self.close_event)(ev.0) })
	}

	/// Signal an event (signals entire group if the event is part of a group)
	pub fn signal_event(&self, ev: Event) -> Status {
		// SAFE: No memory unsafety because the wrapped handle can only have come from a successful `create_event*`
		(unsafe { (self.signal_event)(ev.0) })
	}

	/// Wait for an event to be signaled, returns the index of the signalled event
	pub fn wait_for_event(&self, events: &[Event]) -> Result<usize, Status> {
		if false {
			// SAFE: Never run
			unsafe { ::core::mem::transmute::<raw::Event,Event>(0 as _); }
		}
		let mut rv = 0;
		// SAFE: Valid array of transparent structures
		(unsafe { (self.wait_for_event)(events.len(), events.as_ptr() as *const raw::Event, &mut rv) })
			.err_or(rv)
	}

	/// Check if an event has been signaled
	pub fn check_event(&self, ev: &Event) -> Result<bool,Status> {
		match unsafe { (self.check_event)(ev.0) }
		{
		::status::SUCCESS => Ok(true),
		::status::NOT_READY => Ok(false),
		v => Err(v),
		}
	}

	/// Set/reset a timer event
	pub fn set_timer(&self, ev: &Event, ty: TimerDelay, delay: u64) -> Result<(), Status> {
		// SAFE: No memory unsafety
		unsafe { (self.set_timer)(ev.0, ty, delay).err_or( () ) }
	}
}

impl BootServices
{
	/// Allocate a `Vec`-alike from the firmware's general use pool
	pub fn allocate_pool_vec<T>(&self, mt: MemoryType, capacity: usize) -> Result<PoolVec<T>, Status> {
		let mut ptr = ::core::ptr::null_mut();
		// NOTE: AllocatePool returns 8-byte aligned data
		assert!(::core::mem::align_of::<T>() <= 8);
		// SAFE: Allocation cannot cause unsafety
		(unsafe { (self.allocate_pool)(mt, capacity * ::core::mem::size_of::<T>(), &mut ptr) })
			// SAFE: Valid pointer, alignment checked above
			.err_or_else(|| unsafe { PoolVec::from_ptr(self, ptr as *mut T, capacity, 0) }) 
	}
}

impl BootServices
{
	//#[inline]
	//pub fn locate_handles_by_protocol(&self, protocol: &Guid) -> Result<PoolSlice<Handle>, Status> {
	//	let mut ptr = 0 as *mut _;
	//	let mut count = 0;
	//	(self.locate_handle_buffer)(LocateSearchType::ByProtocol, Some(protocol), 0 as *const _, &mut count, &mut ptr)
	//		.err_or_else(|| PoolSlice(ptr, count) )
	//}
	
	pub fn handle_protocol<'a, P: 'a + protocols::Protocol>(&'a self, handle: &Handle) -> Result<&'a P, Status> {
		let mut ptr = 0 as *mut Void;
		// SAFE: Pointer cannot cause unsafety
		unsafe { (self.handle_protocol)(*handle, &P::guid(), &mut ptr) }
			.err_or_else( || unsafe { &*P::from_ptr(ptr) } )
	}
}
/// Owned vector from the UEFI general pool
pub struct PoolVec<'a, T>
{
	bs: &'a BootServices,
	ptr: ::core::ptr::Unique<T>,
	cap: usize,
	len: usize,
}
impl<'a,T> PoolVec<'a, T>
{
	/// UNSAFE: Pointer must be to `len` valid items, `cap` capacity, and be non-zero
	pub unsafe fn from_ptr(bs: &BootServices, p: *mut T, cap: usize, len: usize) -> PoolVec<T> {
		PoolVec {
			bs: bs,
			ptr: ::core::ptr::Unique::new_unchecked(p),
			cap: cap,
			len: len,
			}
	}
	pub unsafe fn set_len(&mut self, len: usize) {
		assert!(len <= self.cap);
		self.len = len;
	}
}
impl<'a,T> ::core::ops::Deref for PoolVec<'a, T>
{
	type Target = [T];
	fn deref(&self) -> &[T] {
		unsafe {
			::core::slice::from_raw_parts(self.ptr.as_ptr(), self.len)
		}
	}
}
impl<'a,T> ::core::ops::DerefMut for PoolVec<'a, T>
{
	fn deref_mut(&mut self) -> &mut [T] {
		unsafe {
			::core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len)
		}
	}
}
impl<'a,T> ::core::ops::Drop for PoolVec<'a, T>
{
	fn drop(&mut self) {
		unsafe {
			for v in self.iter_mut() {
				::core::ptr::drop_in_place(v);
			}
			(self.bs.free_pool)(self.ptr.as_ptr() as *mut Void);
		}
	}
}

// TODO: Make a wrapper around an array of MemoryDescriptor
#[repr(C)]
pub struct MemoryDescriptor
{
	pub ty: u32,
	_pad: u32,
	pub physical_start: PhysicalAddress,
	pub virtual_start: VirtualAddress,
	pub number_of_pages: u64,
	pub attribute: u64,
	_pad2: u64,
}
#[repr(C)]
pub enum MemoryType
{
    ReservedMemoryType,
    LoaderCode,
    LoaderData,
    BootServicesCode,
    BootServicesData,
    RuntimeServicesCode,
    RuntimeServicesData,
    ConventionalMemory,
    UnusableMemory,
    AcpiReclaimMemory,
    AcpiMemoryNVS,
    MemoryMappedIO,
    MemoryMappedIOPortSpace,
    PalCode,
    MaxMemoryType
}

#[repr(C)]
pub enum TimerDelay
{
	Cancel,
	Periodic,
	Relative,
}
#[repr(C)]
pub enum AllocateType
{
	AnyPages,
	MaxAddress,
	Address,
}
#[repr(C)]
pub enum InterfaceType
{
	Native,
	PCode,
}
#[repr(C)]
pub enum LocateSearchType
{
	AllHandles,
	ByRegisterNotify,
	ByProtocol,
}
#[repr(C)]
pub struct DevicePath
{
	ty: u8,
	sub_type: u8,
	length: [u8; 2],
}

pub type EventNotifyFcn = efi_fcn!{ fn(Event, *mut Void) -> () };

