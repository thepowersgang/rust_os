//! UEFI Interface Crate
//!
//! Provides FFI access to a UEFI environment for UEFI Applications and bootloaders
//!
//! ```no_run
//! #[no_mangle]
//! pub extern "win64" fn efi_main(_image_handle: ::uefi::Handle, system_table: &::uefi::SystemTable) -> ::uefi::Status
//! {
//!     system_table.con_out.output_string_utf8("Hello, world.");
//!     ::uefi::status::SUCCESS
//! }
//! ```
#![no_std]
#![crate_name="uefi"]
#![crate_type="lib"]
#![feature(try_trait_v2)]	// Makes Status a little easier to use
#![feature(ptr_internals)]	// rawptr as_ref

pub use self::str16::Str16;
pub use self::str16::{CStr16Ptr, CStr16};

pub use self::console::{EfiLogger};
pub use self::console::{SimpleInputInterface,SimpleTextOutputInterface};

pub use self::status::Status;

macro_rules! efi_fcn {
	(fn $name:ident ( $($n:ident: $t:ty),* ) -> $rv:ty) => {
		extern "win64" fn $name( $($n: $t),* ) -> $rv
	};
	(fn ( $($n:ident: $t:ty),* ) -> $rv:ty) => {
		unsafe extern "win64" fn( $($n: $t),* ) -> $rv
	};
	(fn ( $($t:ty),* ) -> $rv:ty) => {
		unsafe extern "win64" fn( $($t),* ) -> $rv
	};
}

mod console;
mod str16;
pub mod status;
pub mod runtime_services;
pub mod boot_services;

// libstd miniature clones
pub mod borrow;

pub enum Void {}
pub type Handle = *mut Void;
pub type PhysicalAddress = u64;
pub type VirtualAddress = u64;

/// GUID
pub struct Guid( pub u32, pub u16, pub u16, pub [u8; 8] );

#[macro_export]
/// Log to the provided UEFI SimpleTextOutputInterface sink
macro_rules! loge {
	($l:expr, $($t:tt)*) => {{
		use ::core::fmt::Write;
		let mut logger = $crate::EfiLogger::new($l);
		let _ = write!(&mut logger, "[{}] ", module_path!());
		let _ = write!(&mut logger, $($t)*); 
	}};
}

#[repr(C)]
/// Header for a UEFI table
pub struct TableHeader
{
	pub signature: u64,
	pub revision: u32,
	pub header_size: u32,
	pub crc32: u32,
	_reserved: u32,
}

#[repr(C)]
/// Size+Pointer array pointer
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

// TODO: Have a SystemTablePtr structure that exposes the various boot services as borrows only
// - BUT: The only way to call ExitBootServices is to consume it

#[repr(C)]
/// System Table (top-level EFI structure)
///
/// A pointer to this is passed by the environment to the application as the second parameter to `efi_main`
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

	/// Runtime-accessible UEFI services (availiable after `boot_services.exit_boot_services` has been called)
	pub runtime_services: *const runtime_services::RuntimeServices,
	pub boot_services: &'a boot_services::BootServices,

	pub configuration_table: SizePtr<ConfigurationTable>
}
impl<'a> SystemTable<'a>
{
	#[inline]
	pub fn firmware_vendor(&self) -> &Str16 {
		unsafe {
			Str16::from_nul_terminated(self.firmware_vendor)
		}
	}
	#[inline]
	pub fn con_in(&self) -> &SimpleInputInterface {
		self.con_in
	}
	#[inline]
	pub fn con_out(&self) -> &SimpleTextOutputInterface {
		self.con_out
	}
	#[inline]
	pub fn std_err(&self) -> &SimpleTextOutputInterface {
		self.std_err
	}

	#[inline]
	pub fn runtime_services(&self) -> &runtime_services::RuntimeServices {
		unsafe { &*self.runtime_services }
	}
	#[inline]
	pub fn boot_services(&self) -> &boot_services::BootServices {
		self.boot_services
	}
	#[inline]
	pub fn configuration_table(&self) -> &[ConfigurationTable] {
		&self.configuration_table[..]
	}
}

pub struct ConfigurationTable
{
	pub vendor_guid: Guid,
	pub vendor_table: *const Void,
}



