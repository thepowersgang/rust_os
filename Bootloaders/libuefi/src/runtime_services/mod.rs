//! Runtime-usable services
//!
//!
use super::{Guid, Status, CStr16Ptr, Void};
use super::CStr16;
use super::{PhysicalAddress};
use core::mem;

/// UEFI-defined runtime services structure
///
/// Contains the raw function pointers to the services, use the `make_handle_*` functions to get safe/rustic interfaces to these functions
#[repr(C)]
pub struct RuntimeServices
{
	pub hdr: super::TableHeader,

	pub get_time: efi_fcn!{ fn(&mut Time, Option<&mut TimeCapabilities>) -> Status },
	pub set_time: efi_fcn!{ fn(&Time) -> Status },

	pub get_wakeup_time: efi_fcn!{ fn(&mut bool, &mut bool, &mut Time) -> Status },
	pub set_wakeup_time: efi_fcn!{ fn(bool, &Time) -> Status },

	/// Pointer will be invalid (still physical) after being called
	pub set_virtual_address_map: efi_fcn!{ fn(map_size: usize, descriptor_size: usize, descriptor_version: u32, virtual_map: *const super::boot_services::MemoryDescriptor) -> Status },
	/// Pointer will be invalid (still physical) after `set_virtual_address_map` is called
	pub convert_pointer: efi_fcn!{ fn(debug_disposition: usize, address: &mut *const Void) -> Status },

	pub get_variable: efi_fcn!{ fn(CStr16Ptr, &Guid, Option<&mut u32>, /*data_size:*/ &mut usize, /*data:*/ *mut Void) -> Status },
	pub get_next_variable_name: efi_fcn!{ fn(&mut usize, *mut u16, &mut Guid) -> Status },
	// NOTE: UEFI spec specifies the last parameter here as `void*`, but does not permit mutation
	pub set_variable: efi_fcn!{ fn(CStr16Ptr, &Guid, u32, usize, *const Void) -> Status },

	pub get_next_high_monotonic_count: efi_fcn!{ fn(&mut u32) -> Status },
	pub reset_system: efi_fcn!{ fn(ty: ResetType, sys_status: Status, data_size: usize, reset_data: *const u16) -> Status },

	pub update_capsule: efi_fcn!{ fn(*const *const CapsuleHeader, usize, PhysicalAddress) -> Status },
	pub query_capsure_capabilities: efi_fcn!{ fn(*const *const CapsuleHeader, usize, &mut u64, &mut ResetType) -> Status },
	pub query_variable_info: efi_fcn!{ fn(unk: u32, max_variable_storage_size: &mut u64, remaining_variable_storage_size: &mut u64, maximum_variable_size: &mut u64) -> Status },
}

impl RuntimeServices
{
	fn make_handle(&mut self) -> RuntimeServicesHandle {
		RuntimeServicesHandle {
			time: RuntimeServicesTime(self),
			storage: RuntimeServicesStorage(self),
			}
	}
	/// Create a handle to the runtime services that will be called with virtual memory disabled
	pub unsafe fn make_handle_physical(&mut self) -> Result<RuntimeServicesHandle,Status> {
		Ok(self.make_handle())
	}
	/// Create a handle to the runtime services, providing new virtual locations for each 
	// TODO: Take a ::boot_services::MemoryMap handle that contains the reported size/version
	pub unsafe fn make_handle_virtual(&mut self, map: &[super::boot_services::MemoryDescriptor]) -> Result<RuntimeServicesHandle,Status> {
		(self.set_virtual_address_map)(map.len(), mem::size_of_val(&map[0]), 1, map.as_ptr())?;
		Ok(self.make_handle())
	}
}

pub struct RuntimeServicesHandle<'a>
{
	/// Subset of runtime services that relate to the system timers.
	///
	/// None of these functions can be called at the same time as each other
	pub time: RuntimeServicesTime<'a>,

	/// Subset of runtime services related to firmware storage
	pub storage: RuntimeServicesStorage<'a>,
}
impl<'a> RuntimeServicesHandle<'a>
{
	//pub fn reset_system(&mut self) -> Result<!,Status> {
	//	Err( unsafe { (self.time.0.reset_system)() } )
	//}
}

#[repr(C)]
pub enum ResetType
{
	Cold,
	Warm,
	Shutdown,
}

pub struct RuntimeServicesTime<'a>(&'a RuntimeServices);
impl<'a> RuntimeServicesTime<'a>
{
	pub fn get_time(&mut self) -> Result<Time,Status> {
		// SAFE: `Time` is repr(C), so is valid to be zero
		let mut rv = unsafe { mem::zeroed() };
		// SAFE: Call has no memory unsafety
		unsafe { (self.0.get_time)(&mut rv, None)?; }
		Ok(rv)
	}
	pub fn get_time_with_caps(&mut self) -> Result<(Time, TimeCapabilities),Status> {
		// SAFE: `Time` is repr(C), so is valid to be zero
		let (mut time, mut time_caps) = unsafe { (mem::zeroed(), mem::zeroed()) };
		// SAFE: Unique access to time subsystem, Call has no memory unsafety
		unsafe { (self.0.get_time)(&mut time, Some(&mut time_caps))?; }
		Ok( (time, time_caps) )
	}

	pub fn set_time(&mut self, new_time: Time) -> Result<(),Status> {
		// SAFE: Unique access to time subsystem, no memory unsafety
		unsafe { (self.0.set_time)(&new_time) }?;
		Ok( () )
	}

	pub fn get_wakeup_time(&mut self) -> Result< (bool, Option<Time>), Status > {
		// SAFE: POD
		let mut time = unsafe { mem::zeroed() };
		let mut enabled = false;
		let mut pending = false;
		// SAFE: Unique access to time subsystem, no memory unsafety
		unsafe { (self.0.get_wakeup_time)(&mut enabled, &mut pending, &mut time) }?;
		Ok( (pending, if enabled { Some(time) } else { None }) )
	}
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

pub struct RuntimeServicesStorage<'a>(&'a RuntimeServices);
impl<'a> RuntimeServicesStorage<'a>
{
	//get_variable: efi_fcn!{ fn(CStr16Ptr, &Guid, Option<&mut u32>, /*data_size:*/ &mut usize, /*data:*/ *mut Void) -> Status },
	pub fn get_variable_info(&mut self, name: &CStr16, guid: &Guid) -> Result<(usize,VariableAttributes), Status> {
		let mut len = 0;
		let mut attrs = 0;
		// SAFE: Call is informed that buffer has a length of 0
		unsafe {
			match (self.0.get_variable)(name.as_ptr(), guid, Some(&mut attrs), &mut len, 0 as *mut _)
			{
			::status::SUCCESS => {},
			::status::BUFFER_TOO_SMALL => {},
			s => return Err(s),
			}
		}
		Ok( (len, VariableAttributes(attrs)) )
	}
	pub fn get_variable<'b>(&mut self, name: &CStr16, guid: &Guid, buffer: &'b mut [u8]) -> Result<&'b mut [u8], Status> {
		let mut len = buffer.len();
		// SAFE: Call is informed that buffer is of a particular length
		unsafe {
			(self.0.get_variable)(name.as_ptr(), guid, None, &mut len, buffer.as_mut_ptr() as *mut Void)?;
		}
		Ok(&mut buffer[..len])
	}
	//pub get_next_variable_name: efi_fcn!{ fn(&mut usize, *mut u16, &mut Guid) -> Status },
	pub fn get_next_variable_name<'b>(&mut self, buffer: &'b mut [u16], mut last_guid: Guid) -> Result< (&'b CStr16, Guid), (Status, Option<usize>) > {
		assert!( buffer.iter().any(|&x| x == 0) );
		let mut len = buffer.len();
		// SAFE: All parameters checked, no unsafety (but could be unpredictable)
		unsafe {
			match (self.0.get_next_variable_name)(&mut len, buffer.as_mut_ptr(), &mut last_guid)
			{
			::status::SUCCESS => Ok( (CStr16::from_slice(buffer), last_guid) ),
			::status::BUFFER_TOO_SMALL => return Err( (::status::BUFFER_TOO_SMALL, Some(len)) ),
			s => return Err( (s, None) ),
			}
		}
	}
	//pub query_variable_info: efi_fcn!{ fn(unk: u32, max_variable_storage_size: &mut u64, remaining_variable_storage_size: &mut u64, maximum_variable_size: &mut u64) -> Status },
	pub fn query_variable_info(&mut self, attr_mask: VariableAttributes) -> Result<VariableInfo,Status> {
		let mut rv = VariableInfo {
			maximum_variable_storage_size: 0,
			remaining_variable_storage_size: 0,
			maximum_variable_size: 0,
			};
		unsafe {
			(self.0.query_variable_info)(attr_mask.0, &mut rv.maximum_variable_storage_size, &mut rv.remaining_variable_storage_size, &mut rv.maximum_variable_size)?; 
		}
		Ok(rv)
	}
	//pub set_variable: efi_fcn!{ fn(CStr16Ptr, &Guid, u32, usize, *const Void) -> Status },
	pub fn set_variable(&mut self, name: &CStr16, guid: &Guid, attrs: VariableAttributes, data: &[u8]) -> Status {
		// SAFE: All parameters checked
		unsafe {
			(self.0.set_variable)(name.as_ptr(), guid, attrs.0, data.len(), data.as_ptr() as *const Void)
		}
	}

	//pub get_next_high_monotonic_count: efi_fcn!{ fn(&mut u32) -> Status },
	pub fn get_next_high_monotonic_count(&mut self) -> Result<u32,Status> {
		let mut v = 0;
		// SAFE: No memory unsafety
		unsafe { (self.0.get_next_high_monotonic_count)(&mut v) }?;
		Ok(v)
	}

	//pub update_capsule: efi_fcn!{ fn(*const *const CapsuleHeader, usize, PhysicalAddress) -> Status },
	pub unsafe fn update_capsule(&mut self, capsule_headers: &[&CapsuleHeader]) -> Status {
		(self.0.update_capsule)(capsule_headers.as_ptr() as *const _, capsule_headers.len(), 0)
	}
	//pub query_capsure_capabilities: efi_fcn!{ fn(*const *const CapsuleHeader, usize, &mut u64, &mut ResetType) -> Status },
	pub unsafe fn query_capsure_capabilities(&mut self, capsule_headers: &[&CapsuleHeader]) -> Result<(u64,ResetType), Status> {
		let mut rt = ResetType::Warm;
		let mut max_size = 0;
		(self.0.query_capsure_capabilities)(capsule_headers.as_ptr() as *const _, capsule_headers.len(), &mut max_size, &mut rt)?;
		Ok( (max_size, rt) )
	}
}

#[repr(C)]
pub struct CapsuleHeader
{
	pub guid: Guid,
	pub header_size: u32,
	pub flags: u32,
	pub capsule_image_size: u32,
}

pub struct VariableAttributes(u32);
macro_rules! def_bits {
	($($mask:expr => $set:ident,$unset:ident,$test:ident),*$(,)*) => {
		$(
		pub fn $set(self) -> Self {
			VariableAttributes(self.0 | $mask)
		}
		pub fn $unset(self) -> Self {
			VariableAttributes(self.0 & !$mask)
		}
		pub fn $test(&self) -> bool {
			self.0 & $mask != 0
		}
		)*
	}
}
impl VariableAttributes
{
	pub fn new() -> Self {
		VariableAttributes(0)
	}
	pub fn full_mask() -> Self {
		VariableAttributes(0xF)
	}

	def_bits!{
		0x01 => non_volatile                         , not_non_volatile                         , is_non_volatile,
		0x02 => bootservice_access                   , not_bootservice_access                   , is_bootservice_access,
		0x04 => runtime_access                       , not_runtime_access                       , is_runtime_access,
		0x08 => hardware_error_record                , not_hardware_error_record                , is_hardware_error_record,
		0x10 => authenticated_write_access           , not_authenticated_write_access           , is_authenticated_write_access,
		0x20 => time_based_authenticated_write_access, not_time_based_authenticated_write_access, is_time_based_authenticated_write_access,
		0x40 => append_write                         , not_append_write                         , is_append_write,
		}
}
pub struct VariableInfo
{
	/// Maximum amount of space (in storage) for variables of the specified atribute mask
	pub maximum_variable_storage_size: u64,
	/// Remaining free space for variable storage
	pub remaining_variable_storage_size: u64,
	/// Size of the largest variable
	pub maximum_variable_size: u64,
}

