//! 8.2 ACPI Table Management
use super::common::*;
use super::ACPI_STATUS;
use super::Void;

// AcpiInstallTableHandler
pub type ACPI_TABLE_HANDLER = extern "C" fn (Event: u32, Table: *const Void, Context: *const Void) -> ACPI_STATUS;

extern "C" {
	/// Initialize the ACPICA table manager.
	pub fn AcpiInitializeTables(InitialStorage: *mut ACPI_TABLE_DESC, InitialTableCount: u32, AllowResize: bool) -> ACPI_STATUS;
	/// Copy the root ACPI information table into dynamic memory.
	pub fn AcpiReallocateRootTable() -> ACPI_STATUS;
	
	/// Locate the RSDP via memory scan (IA-32).
	#[allow(improper_ctypes)]
	pub fn AcpiFindRootPointer(TableAddress: *mut ACPI_SIZE) -> ACPI_STATUS;
	
	/// Early installation of a single host-provided ACPI table.
	pub fn AcpiInstallTable(Address: ACPI_PHYSICAL_ADDRESS, Physical: bool) -> ACPI_STATUS;
	/// Load the BIOS-provided ACPI tables and build an internal ACPI namespace.
	pub fn AcpiLoadTables() -> ACPI_STATUS;
	/// Load a single host-provided ACPI table.
	pub fn AcpiLoadTable(Table: *mut ACPI_TABLE_HEADER) -> ACPI_STATUS;
	/// Unloads an ACPI table via a namespace object that is owned by the table.
	pub fn AcpiUnloadParentTable(Object: ACPI_HANDLE) -> ACPI_STATUS;
	/// Get the header portion of a specific installed ACPI table.
	pub fn AcpiGetTableHeader(Signature: *const u8, Instance: u32, OutTableHeader: *mut ACPI_TABLE_HEADER) -> ACPI_STATUS;
	/// Obtain a specific installed ACPI table.
	pub fn AcpiGetTable(signature: *const u8, instance: u32, table: *mut *const ACPI_TABLE_HEADER) -> ACPI_STATUS;
	/// Obtain an installed ACPI table via an index into the Root Table
	pub fn AcpiGetTableByIndex(TableIndex: u32, OutTable: *mut *const ACPI_TABLE_HEADER) -> ACPI_STATUS;
	/// Install a global handler for ACPI table load and unload events.
	pub fn AcpiInstallTableHandler(Handler: ACPI_TABLE_HANDLER, Context: *const Void) -> ACPI_STATUS;
	/// Remove a handler for ACPI table events.
	pub fn AcpiRemoveTableHandler(Handler: ACPI_TABLE_HANDLER) -> ACPI_STATUS;
}

#[repr(C)]
pub struct ACPI_TABLE_DESC(crate::Void);

#[repr(C)]
#[derive(Copy,Clone)]
pub struct ACPI_TABLE_HEADER
{
	pub signature: [u8; 4],
	pub length: u32,
	pub revision: u8,
	pub checksum: u8,
	pub oemid: [u8; 6],
	pub oem_table_id: [u8; 8],
	pub oem_revision: u32,
	pub creator_id: u32,
	pub creator_revision: u32,
}
