// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/acpica/shim_ext.rs
//! ACPICA outbound bindings
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
use core::fmt;

pub type ACPI_SIZE = usize;
pub type ACPI_PHYSICAL_ADDRESS = ::memory::PAddr;
pub type ACPI_IO_ADDRESS = u16;

pub type ACPI_PCI_ID = u32;

pub type ACPI_CPU_FLAGS = u32;
pub type ACPI_SPINLOCK = *const ::sync::Spinlock<()>;
pub type ACPI_MUTEX = *const ::sync::Mutex<()>;
pub type ACPI_SEMAPHORE = *const ::sync::Semaphore;

#[repr(C)]
pub enum ACPI_EXECUTE_TYPE
{
	OSL_GLOBAL_LOCK_HANDLER,
	OSL_NOTIFY_HANDLER,
	OSL_GPE_HANDLER,
	OSL_DEBUGGER_THREAD,
	OSL_EC_POLL_HANDLER,
	OSL_EC_BURST_HANDLER
}
pub type ACPI_OSD_EXEC_CALLBACK = extern "C" fn(*const ());
pub type ACPI_OSD_HANDLER = extern "C" fn (*const ())->u32;

#[repr(C)]
pub struct ACPI_PREDEFINED_NAMES
{
	Name: *const u8,
	Type: i8,
	Val: *const u8,
}
pub type ACPI_STRING = *mut u8;

pub type ACPI_THREAD_ID = u64;

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

pub type ACPI_TABLE_HEADER = super::super::SDTHeader;

pub const ACPI_FULL_INITIALIZATION  : u32 = 0x00;
pub const ACPI_NO_ADDRESS_SPACE_INIT: u32 = 0x01;
pub const ACPI_NO_HARDWARE_INIT     : u32 = 0x02;
pub const ACPI_NO_EVENT_INIT        : u32 = 0x04;
pub const ACPI_NO_HANDLER_INIT      : u32 = 0x08;
pub const ACPI_NO_ACPI_ENABLE       : u32 = 0x10;
pub const ACPI_NO_DEVICE_INIT       : u32 = 0x20;
pub const ACPI_NO_OBJECT_INIT       : u32 = 0x40;

#[no_mangle]
extern "C" {
	pub fn AcpiInitializeSubsystem() -> ACPI_STATUS;
	
	pub fn AcpiInitializeTables(InitialStorage: *mut ACPI_TABLE_DESC, InitialTableCount: u32, AllowResize: bool) -> ACPI_STATUS;
	pub fn AcpiFindRootPointer(TableAddress: *mut ACPI_SIZE) -> ACPI_STATUS;
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

