use core::fmt;

#[allow(non_camel_case_types)]
pub struct ACPI_STATUS(i32);

const AE_CODE_ENVIRONMENTAL: i32 = 0x0000;
const AE_CODE_PROGRAMMER   : i32 = 0x1000;

pub const AE_OK            : ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL|0);
pub const AE_ERROR         : ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL|1);
pub const AE_NO_ACPI_TABLES: ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL|2);
pub const AE_NO_MEMORY     : ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL|4);
pub const AE_NOT_FOUND     : ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL|5);
pub const AE_BAD_PARAMETER: ACPI_STATUS = ACPI_STATUS(AE_CODE_PROGRAMMER|1);

#[repr(C)]
pub struct ACPI_TABLE_DESC;

#[repr(C)]
pub struct ACPI_TABLE_HEADER;

pub const ACPI_FULL_INITIALIZATION  : u32 = 0x00;
pub const ACPI_NO_ADDRESS_SPACE_INIT: u32 = 0x01;
pub const ACPI_NO_HARDWARE_INIT     : u32 = 0x02;
pub const ACPI_NO_EVENT_INIT        : u32 = 0x04;
pub const ACPI_NO_HANDLER_INIT      : u32 = 0x08;
pub const ACPI_NO_ACPI_ENABLE       : u32 = 0x10;
pub const ACPI_NO_DEVICE_INIT       : u32 = 0x20;
pub const ACPI_NO_OBJECT_INIT       : u32 = 0x40;

#[no_mangle]
#[allow(non_snake_case)]
extern "C" {
	pub fn AcpiInitializeSubsystem() -> ACPI_STATUS;
	
	pub fn AcpiInitializeTables(InitialStorage: *mut ACPI_TABLE_DESC, InitialTableCount: u32, AllowResize: bool) -> ACPI_STATUS;
	pub fn AcpiLoadTables() -> ACPI_STATUS;
	pub fn AcpiGetTable(signature: *const u8, instance: u32, table: *mut *const ACPI_TABLE_HEADER) -> ACPI_STATUS;
	
	pub fn AcpiEnableSubsystem(flags: u32) -> ACPI_STATUS;
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

