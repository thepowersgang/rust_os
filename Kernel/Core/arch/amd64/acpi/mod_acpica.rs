// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/mod_acpica.rs
//! ACPI Component Architecture binding
use core::fmt;

#[allow(non_camel_case_types)]
struct ACPI_STATUS(i32);
const AE_OK: ACPI_STATUS = ACPI_STATUS(0);

#[repr(C)]
struct ACPI_TABLE_DESC;

const ACPI_FULL_INITIALIZATION  : u32 = 0x00;
const ACPI_NO_ADDRESS_SPACE_INIT: u32 = 0x01;
const ACPI_NO_HARDWARE_INIT     : u32 = 0x02;
const ACPI_NO_EVENT_INIT        : u32 = 0x04;
const ACPI_NO_HANDLER_INIT      : u32 = 0x08;
const ACPI_NO_ACPI_ENABLE       : u32 = 0x10;
const ACPI_NO_DEVICE_INIT       : u32 = 0x20;
const ACPI_NO_OBJECT_INIT       : u32 = 0x40;

#[no_mangle]
#[allow(non_snake_case)]
extern "C" {
	fn AcpiInitializeSubsystem() -> ACPI_STATUS;
	
	fn AcpiInitializeTables(InitialStorage: *mut ACPI_TABLE_DESC, InitialTableCount: u32, AllowResize: bool) -> ACPI_STATUS;
	fn AcpiLoadTables() -> ACPI_STATUS;
	
	fn AcpiEnableSubsystem(flags: u32) -> ACPI_STATUS;
}

macro_rules! acpi_try {
	($fcn:ident ($($v:expr),*)) => (match $fcn($($v),*) {
		AE_OK => {},
		ec => { log_error!("ACPICA Init - {} returned {}", stringify!($fcn), ec); return; },
		});
}

impl fmt::Display for ACPI_STATUS {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self.0
		{
		0 => write!(f, "Success"),
		1 => write!(f, "Misc Error"),
		_ => write!(f, "Unknown({})", self.0),
		}
	}
}

pub fn init()
{
	unsafe {
		acpi_try!(AcpiOsInitialize());
		acpi_try!(AcpiInitializeTables(0 as *mut _, 16, false));
		acpi_try!(AcpiLoadTables());
		acpi_try!(AcpiEnableSubsystem(ACPI_FULL_INITIALIZATION));
	}
}

#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsInitialize() -> ACPI_STATUS
{
	AE_OK
}

