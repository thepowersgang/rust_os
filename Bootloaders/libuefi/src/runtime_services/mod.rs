
use super::{Guid, Status, CStr16Ptr, Void};
use super::{PhysicalAddress};

#[repr(C)]
pub struct RuntimeServices
{
	pub hdr: super::TableHeader,

	pub get_time: extern "win64" fn(&mut Time, Option<&mut TimeCapabilities>) -> Status,
	pub set_time: extern "win64" fn(&Time) -> Status,

	pub get_wakeup_time: extern "win64" fn(&mut bool, &mut bool, &mut Time) -> Status,
	pub set_wakeup_time: extern "win64" fn(bool, &Time) -> Status,

	pub set_virtual_address_map: extern "win64" fn(map_size: usize, descriptor_size: usize, descriptor_version: u32, virtual_map: *const super::boot_services::MemoryDescriptor) -> Status,
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
pub struct CapsuleHeader
{
	pub guid: Guid,
	pub header_size: u32,
	pub flags: u32,
	pub capsule_image_size: u32,
}

