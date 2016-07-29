///
use {Status,Handle,Void,Guid};
use boot_services::{MemoryType};

/// Protocol GUID
pub const GUID: Guid = Guid(0x5B1B31A1,0x9562,0x11d2,[0x8E,0x3F,0x00,0xA0,0xC9,0x69,0x72,0x3B]);

#[repr(C)]
pub struct LoadedImage<'a>
{
	pub revision: u32,
	pub parent_handle: Handle,
	pub system_table: &'a ::SystemTable<'a>,
	
	// Source location of the image
	pub device_handle: Handle,
	pub file_path: &'a super::DevicePath,
	reserved: *mut Void,
	
	// Image’s load options
	pub load_options_size: u32,
	pub load_options: *mut Void,
	
	// Location where image was loaded
	pub image_base: *mut Void,
	pub image_size: u64,
	
	pub image_code_type: MemoryType,
	pub image_data_type: MemoryType,
	
	pub unload: extern "win64" fn(Handle) -> Status,
}


impl<'a> super::Protocol for LoadedImage<'a>
{
	fn guid() -> Guid {
		GUID
	}
	unsafe fn from_ptr(ptr: *const Void) -> *const LoadedImage<'a> {
		ptr as *const _
	}
}

