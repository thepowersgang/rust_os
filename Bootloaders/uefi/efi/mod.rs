//!
//!
//!

pub enum Void {}
pub type Handle = *mut Void;
pub type Status = i32;
pub type Event = *mut Void;

pub type CStr16Ptr = *const u16;

pub struct Guid( pub u32, pub u16, pub u16, pub [u8; 8] );

macro_rules! efi_fcn {
	(fn $name:ident ( $($n:ident: $t:ty),* ) -> $rv:ty) => {
		extern "win64" fn $name( $($n: $ty),* ) -> $rv
	}
}

mod con;
mod str16;
pub use self::str16::Str16;

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
pub struct SystemTable
{
	pub hdr: TableHeader,

	pub firmware_vendor: CStr16Ptr,
	pub firmware_revision: u32,

	pub console_in_handle: Handle,
	pub con_in: *const SimpleInputInterface,

	pub console_out_handle: Handle,
	pub con_out: *const SimpleTextOutputInterface,

	pub standard_error_handle: Handle,
	pub std_err: *const SimpleTextOutputInterface,

	pub runtime_services: *const RuntimeServices,
	pub boot_services: *const BootServices,

	pub configuraton_table: SizePtr<ConfigurationTable>
}
impl SystemTable
{
	pub fn firmware_vendor(&self) -> &Str16 {
		unsafe {
			Str16::from_nul_terminated(self.firmware_vendor)
		}
	}
	pub fn con_in(&self) -> &SimpleInputInterface {
		unsafe { &*self.con_in }
	}
	pub fn con_out(&self) -> &SimpleTextOutputInterface {
		unsafe { &*self.con_out }
	}
	pub fn std_err(&self) -> &SimpleTextOutputInterface {
		unsafe { &*self.std_err }
	}

	pub fn runtime_services(&self) -> &RuntimeServices {
		unsafe { &*self.runtime_services }
	}
	pub fn boot_services(&self) -> &BootServices {
		unsafe { &*self.boot_services }
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
}

#[repr(C)]
pub struct BootServices
{
	pub hdr: TableHeader,
}

