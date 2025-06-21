// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/acpica/shim_ext.rs
//! ACPICA outbound bindings
#![no_std]
#![allow(non_camel_case_types,non_snake_case)]	// follows C naming conventions

pub enum Void {}

mod common;
pub use self::common::*;
mod status;
pub use self::status::*;
mod init_ctrl;
pub use self::init_ctrl::*;
mod tables;
pub use self::tables::*;
mod namespace_mgmt;
pub use self::namespace_mgmt::*;
mod events;
pub use self::events::*;

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
pub type ACPI_OSD_EXEC_CALLBACK = extern "C" fn(*const Void);
pub type ACPI_OSD_HANDLER = extern "C" fn (*const Void)->u32;

#[repr(C)]
pub struct ACPI_PREDEFINED_NAMES
{
	Name: *const u8,
	Type: i8,
	Val: *const u8,
}
extern "C" {
	// 8.4 ACPI Hardware Management
	/// Put the system into ACPI mode.
	pub fn AcpiEnable() -> ACPI_STATUS;
	/// Take the system out of ACPI mode.
	pub fn AcpiDisable() -> ACPI_STATUS;
	// ...
}

// 8.5 ACPI Sleep/Wake Support

// 8.7 ACPI General Purpose Event (GPE) Management

extern "C" {
	// 8.8 Miscellaneous Handler Support
	// /// Install a handler for ACPI System Control Interrupts (SCIs).
	// /// Remove an ACPI SCI handler.
	// /// Install a global handler for all ACPI General Purpose and Fixed Events.
	// /// Install a handler for notification events on an ACPI object.
	// /// Remove a handler for ACPI notification events.
	// /// Install handlers for ACPI Operation Region events.
	// /// Remove an ACPI Operation Region handler.
	// /// Install a handler for ACPI interpreter run-time exceptions.
	// 8.9 ACPI Resource Management
	// 8.10 Memory Management
	// 8.11 Formatted Output
	// 8.12 Miscellaneous Utilities
	// 8.13 Global Variables
	/// Bit field that enables/disables the various debug output levels.
	pub static mut AcpiDbgLevel: u32;
	/// Bit field that enables/disables debug output from entire subcomponents within the ACPICA subsystem.
	pub static mut AcpiDbgLayer: u32;
	/// This is a local copy of the system FADT, converted to a common internal format.
	pub static AcpiGbl_FADT: ACPI_TABLE_HEADER;
	/// The current number of active (available) system GPEs.
	pub static AcpiCurrentGpeCount: u32;
	/// This boolean is set to FALSE just before the system sleeps. It is then set to TRUE as the system wakes.
	pub static AcpiGbl_SystemAwakeAndRunning: bool;
}

