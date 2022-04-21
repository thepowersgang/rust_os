// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/mod_acpica.rs
//! ACPI Component Architecture binding
#[allow(unused_imports)]
use crate::prelude::*;
use core::ops;
use self::shim_ext::*;

// Shim - Functions called by rust
mod shim_ext;
// Shim - Functions called by ACPICA
mod os_int;

extern crate va_list;

pub struct SDTHandle<T: 'static>(&'static super::SDT<T>);

macro_rules! acpi_try {
	($fcn:ident ($($v:expr),*)) => (match $fcn($($v),*) {
		AE_OK => {},
		ec => { log_error!("ACPICA Init - {} returned {}", stringify!($fcn), ec); return; },
		});
}

pub fn init()
{
	// SAFE: Validly calls ACPICA calls
	unsafe {
		//AcpiDbgLevel = !0;
		
		log_trace!("AcpiInitializeSubsystem");
		acpi_try!(AcpiInitializeSubsystem());
		log_trace!("AcpiInitializeTables");
		acpi_try!(AcpiInitializeTables(0 as *mut _, 16, false));
		log_trace!("AcpiLoadTables");
		acpi_try!(AcpiLoadTables());
		log_trace!("AcpiEnableSubsystem");
		acpi_try!(AcpiEnableSubsystem(ACPI_FULL_INITIALIZATION));
	}
}

pub fn find_table<T:'static>(req_name: &str, idx: usize) -> Option<SDTHandle<T>>
{
	// SAFE:
	// 1. It's assumed (TODO) that the pointer returned is practically 'static
	// 2. TODO check thread-safety of ACPICA
	// SAFE: See above
	unsafe
	{
		let mut out_ptr: *const ACPI_TABLE_HEADER = 0 as *const _;
		
		// HACK: It's implied that the signature param is a C string, but stated that it's
		//   "A pointer to the 4-character ACPI signature for the requeste table", which implies
		//   that taking a pointer to a non NUL-terminated string is valid.
		match AcpiGetTable(req_name.as_bytes().as_ptr(), idx as u32 + 1, &mut out_ptr)
		{
		shim_ext::AE_OK => {
			log_debug!("AcpiGetTable: out_ptr = {:p}", out_ptr);
			crate::logging::hex_dump_t("AcpiGetTable", &*out_ptr);
			let handle = SDTHandle(&*(out_ptr as *const super::SDT<T>));
			if handle.raw_signature() != req_name.as_bytes() {
				log_warning!("AcpiGetTable: Signature mismatch {:?} != exp {:?}", handle.raw_signature(), req_name.as_bytes());
				None
			}
			else if ! handle.validate() {
				log_warning!("AcpiGetTable: Failed validation");
				None
			}
			else {
				Some( handle )
			}
			},
		shim_ext::AE_NOT_FOUND => None,

		shim_ext::AE_BAD_PARAMETER => panic!("BUGCHECK: AcpiGetTable returned AE_BAD_PARAMETER"),
		ec @ _ => {
			log_notice!("AcpiGetTable in find_table({},{}) returned {}", req_name, idx, ec);
			None
			},
		}
	}
}
pub fn count_tables(req_name: &str) -> usize {
	todo!("count_tables({})", req_name);
}

impl<T: 'static> ops::Deref for SDTHandle<T> {
	type Target = super::SDT<T>;
	fn deref(&self) -> &super::SDT<T> {
		self.0
	}
}

