//!
//!
//!
#![no_std]
#![crate_name="uefi"]
#![crate_type="lib"]

pub enum Void {}
pub type Handle = *mut Void;
pub type Event = *mut Void;

#[repr(C)]
#[derive(Copy,Clone)]
pub struct Status(i32);
impl Status
{
	pub fn new(val: i32) -> Status {
		Status(val)
	}
	pub fn err_or<T>(self, v: T) -> Result<T,Status> {
		if self.0 == 0 {
			Ok(v)
		}
		else {
			Err(self)
		}
	}
}

pub type CStr16Ptr = *const u16;

pub struct Guid( pub u32, pub u16, pub u16, pub [u8; 8] );

macro_rules! efi_fcn {
	(fn $name:ident ( $($n:ident: $t:ty),* ) -> $rv:ty) => {
		extern "win64" fn $name( $($n: $ty),* ) -> $rv
	}
}

#[macro_export]
macro_rules! loge {
	($l:expr, $($t:tt)*) => {{
		use ::core::fmt::Write;
		let mut logger = $crate::EfiLogger::new($l);
		let _ = write!(&mut logger, "[{}] ", module_path!());
		let _ = write!(&mut logger, $($t)*); 
	}};
}

mod con;
mod str16;
pub use self::str16::Str16;

pub use self::con::{EfiLogger};
pub use self::con::{SimpleInputInterface,SimpleTextOutputInterface};

#[repr(C)]
pub struct TableHeader
{
	pub signature: u64,
	pub revision: u32,
	pub header_size: u32,
	pub crc32: u32,
	_reserved: u32,
}

#[repr(C)]
pub struct SizePtr<T>
{
	count: usize,
	data: *const T,
}
impl<T> ::core::ops::Deref for SizePtr<T>
{
	type Target = [T];
	fn deref(&self) -> &[T] {
		// SAFE: (assumed) from FFI and defined to be correct
		unsafe {
			::core::slice::from_raw_parts(self.data, self.count)
		}
	}
}

#[repr(C)]
/// System Table (top-level EFI structure)
pub struct SystemTable<'a>
{
	pub hdr: TableHeader,

	pub firmware_vendor: CStr16Ptr,
	pub firmware_revision: u32,

	pub console_in_handle: Handle,
	pub con_in: &'a SimpleInputInterface,

	pub console_out_handle: Handle,
	pub con_out: &'a SimpleTextOutputInterface,

	pub standard_error_handle: Handle,
	pub std_err: &'a SimpleTextOutputInterface,

	pub runtime_services: *const RuntimeServices,
	pub boot_services: &'a BootServices,

	pub configuraton_table: SizePtr<ConfigurationTable>
}
impl<'a> SystemTable<'a>
{
	pub fn firmware_vendor(&self) -> &Str16 {
		unsafe {
			Str16::from_nul_terminated(self.firmware_vendor)
		}
	}
	pub fn con_in(&self) -> &SimpleInputInterface {
		self.con_in
	}
	pub fn con_out(&self) -> &SimpleTextOutputInterface {
		self.con_out
	}
	pub fn std_err(&self) -> &SimpleTextOutputInterface {
		self.std_err
	}

	pub fn runtime_services(&self) -> &RuntimeServices {
		unsafe { &*self.runtime_services }
	}
	pub fn boot_services(&self) -> &BootServices {
		self.boot_services
	}
	pub fn configuraton_table(&self) -> &[ConfigurationTable] {
		&self.configuraton_table[..]
	}
}

pub struct ConfigurationTable
{
	pub vendor_guid: Guid,
	pub vendor_table: *const Void,
}


#[repr(C)]
pub struct RuntimeServices
{
	pub hdr: TableHeader,

	pub get_time: extern "win64" fn(&mut Time, Option<&mut TimeCapabilities>) -> Status,
	pub set_time: extern "win64" fn(&Time) -> Status,

	pub get_wakeup_time: extern "win64" fn(&mut bool, &mut bool, &mut Time) -> Status,
	pub set_wakeup_time: extern "win64" fn(bool, &Time) -> Status,

	pub set_virtual_address_map: extern "win64" fn(map_size: usize, descriptor_size: usize, descriptor_version: u32, virtual_map: *const MemoryDescriptor) -> Status,
	pub convert_pointer: extern "win64" fn(debug_disposition: usize, address: &mut *const Void) -> Status,

	pub get_variable: extern "win64" fn(CStr16Ptr, &Guid, Option<&mut u32>, data_size: &mut usize, data: *mut Void) -> Status,
	pub get_next_variable_name: extern "win64" fn(&mut usize, *mut u16, &mut Guid) -> Status,
	pub set_variable: extern "win64" fn(CStr16Ptr, &Guid, u32, usize, *mut Void) -> Status,

	pub get_next_high_monotonic_count: extern "win64" fn(&mut u32) -> Status,
	pub reset_system: extern "win64" fn(ResetType, Status, data_size: usize, reset_data: *const u16) -> Status,

	pub update_capsule: extern "win64" fn(*const *const CapsuleHeader, usize, PhysicalAddress) -> Status,
	pub query_capsure_capabilities: extern "win64" fn(*const *const CapsuleHeader, usize, &mut u64, &mut ResetType) -> Status,
	pub query_variable_info: extern "win64" fn(u32, max_variable_storage_size: &mut u64, remaining_variable_storage_size: &mut u64, maximum_variable_size: &mut u64) -> Status,
}

#[repr(C)]
pub enum ResetType
{
	Cold,
	Warm,
	Shutdown,
}

pub type Tpl = usize;

pub type EventNotifyFcn = extern "win64" fn(Event, *mut Void);
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

#[repr(C)]
pub struct BootServices
{
	pub hdr: TableHeader,
	// Task Priority
	pub raise_tpl: extern "win64" fn(Tpl) -> Tpl,
	pub restore_tpl: extern "win64" fn(Tpl) -> Tpl,

	// Memory
	pub allocate_pages: extern "win64" fn(AllocateType, MemoryType, no_pages: usize, phys_addr: &mut PhysicalAddress) -> Status,
	pub free_pages: extern "win64" fn(PhysicalAddress, usize) -> Status,
	pub get_memory_map: extern "win64" fn(&mut usize, *mut MemoryDescriptor, map_key: &mut usize, descriptor_size: &mut usize, descriptor_version: &mut u32) -> Status,
	pub allocate_pool: extern "win64" fn(MemoryType, usize, &mut *mut Void) -> Status,
	pub free_pool: extern "win64" fn(*mut Void) -> Status,

	// Timing and events
	pub create_event: extern "win64" fn(u32, notify_tpl: Tpl, notify_function: EventNotifyFcn, *mut Void, &mut Event) -> Status,
	pub set_timer: extern "win64" fn(Event, TimerDelay, u64) -> Status,
	pub wait_for_event: extern "win64" fn(usize, *mut Event, &mut usize) -> Status,
	pub signal_event: extern "win64" fn(Event) -> Status,
	pub close_event: extern "win64" fn(Event) -> Status,
	pub check_event: extern "win64" fn(Event) -> Status,

	// Protocol handler functions
	pub install_protocol_interface: extern "win64" fn(&mut Handle, &Guid, InterfaceType, *mut Void) -> Status,
	pub reinstall_protocol_interface: extern "win64" fn(Handle, &Guid, old: *mut Void, new: *mut Void) -> Status,
	pub uninstall_protocol_interface: extern "win64" fn(Handle, &Guid, *mut Void) -> Status,
	pub handle_protocol: extern "win64" fn(Handle, &Guid, &mut *mut Void) -> Status,
	pub pc_handle_protocol: extern "win64" fn(Handle, &Guid, &mut *mut Void) -> Status,
	pub register_protocol_notify: extern "win64" fn(&Guid, Event, &mut *mut Void) -> Status,
	pub locate_handle: extern "win64" fn(LocateSearchType, Option<&Guid>, *mut Void, &mut usize, *mut Handle) -> Status,
	pub locate_device_path: extern "win64" fn(&Guid, &mut *mut DevicePath, &mut Handle) -> Status,
	pub install_configuration_table: extern "win64" fn(&Guid, *mut Void) -> Status,

	// Image functions
	pub load_image: extern "win64" fn(bool, Handle, &DevicePath, *mut Void, usize, &mut Handle) -> Status,
	pub start_image: extern "win64" fn(Handle, &mut usize, Option<&mut *mut u16>) -> Status,
	pub exit: extern "win64" fn(Handle, Status, usize, *const u16) -> Status,
	pub unload_image: extern "win64" fn(Handle) -> Status,
	pub exit_boot_services: extern "win64" fn(Handle, map_key: usize) -> Status,
	
	// Misc functions
	pub get_next_monotonic_count: extern "win64" fn() -> Status,
	pub stall: extern "win64" fn() -> Status,
	pub set_watchdog_timer: extern "win64" fn() -> Status,

	// DriverSupport Services
	pub connect_controller: extern "win64" fn() -> Status,
	pub disconnect_controller: extern "win64" fn() -> Status,

	// Open/Close Protocol Services
	pub open_protocol: extern "win64" fn() -> Status,
	pub close_protocol: extern "win64" fn() -> Status,
	pub open_protocol_information: extern "win64" fn() -> Status,

	// Library Services
	pub protocols_per_handle: extern "win64" fn() -> Status,
	pub locate_handle_buffer: extern "win64" fn() -> Status,
	pub locate_protocol: extern "win64" fn() -> Status,
	pub install_multiple_protocol_interfaces: extern "win64" fn() -> Status,
	pub uninstall_multiple_protocol_interfaces: extern "win64" fn() -> Status,

	// CRC
	pub calculate_crc32: extern "win64" fn() -> Status,

	// Misc Services
	pub copy_mem: extern "win64" fn() -> Status,
	pub set_mem: extern "win64" fn() -> Status,
	pub create_event_ex: extern "win64" fn() -> Status,
}

#[repr(C)]
pub struct Time
{
	pub year: u16,	// 
	pub month: u8,	// 1 - 12
	pub day: u8,	// 1 - 31
	pub hour: u8,
	pub minute: u8,
	pub second: u8,
	_pad: u8,
	pub nanosecond: u32,
	pub time_zone: u16,	// -1440 to 1440 or 2047
	pub daylight: u8,
	_pad2: u8,

}
#[repr(C)]
pub struct TimeCapabilities
{
	pub resolution: u32,
	pub accuracy: u32,
	pub sets_to_zero: bool,
}

#[repr(C)]
pub struct MemoryDescriptor
{
	pub ty: u32,
	_pad: u32,
	pub physical_start: PhysicalAddress,
	pub virtual_start: VirtualAddress,
	pub number_of_pages: u64,
	pub attribute: u64,
}

pub type PhysicalAddress = u64;
pub type VirtualAddress = u64;

#[repr(C)]
pub struct CapsuleHeader
{
	pub guid: Guid,
	pub header_size: u32,
	pub flags: u32,
	pub capsule_image_size: u32,
}


