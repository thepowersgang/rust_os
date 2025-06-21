use super::common::*;
use super::ACPI_STATUS;

// AcpiInstallInitializationHandler
pub type ACPI_INIT_HANDLER = extern "C" fn(Object: ACPI_HANDLE, Function: u32) -> ACPI_STATUS;
// AcpiInstallInterfaceHandler
pub type ACPI_INTERFACE_HANDLER = extern "C" fn(InterfaceName: ACPI_STRING, Supported: u32) -> u32;

// AcpiEnableSubsystem, AcpiInitializeObjects
pub const ACPI_FULL_INITIALIZATION  : u32 = 0x00;
pub const ACPI_NO_ADDRESS_SPACE_INIT: u32 = 0x01;
pub const ACPI_NO_HARDWARE_INIT     : u32 = 0x02;
pub const ACPI_NO_EVENT_INIT        : u32 = 0x04;
pub const ACPI_NO_HANDLER_INIT      : u32 = 0x08;
pub const ACPI_NO_ACPI_ENABLE       : u32 = 0x10;
pub const ACPI_NO_DEVICE_INIT       : u32 = 0x20;
pub const ACPI_NO_OBJECT_INIT       : u32 = 0x40;
// AcpiUpdateInterfaces (TODO)
//pub const ACPI_DISABLE_ALL_VENDOR_STRINGS : u8 = 0x01;

extern "C" {
	// 8.1 ACPICA Subsystem Initialization and Control
	/// Initialize all ACPICA globals and sub-components.
	pub fn AcpiInitializeSubsystem() -> ACPI_STATUS;
	pub fn AcpiInstallInitializationHandler(Handler: ACPI_INIT_HANDLER, Function: u32) -> ACPI_STATUS;
	pub fn AcpiEnableSubsystem(flags: u32) -> ACPI_STATUS;
	pub fn AcpiInitializeObjects(flags: u32) -> ACPI_STATUS;
	pub fn AcpiSubsystemStatus() -> ACPI_STATUS;
	pub fn AcpiTerminate() -> ACPI_STATUS;
	/// Install an interface into the list of interfaces recognized by the _OSI predefined method.
	pub fn AcpiInstallInterface(InterfaceName: ACPI_STRING) -> ACPI_STATUS;
	/// Update _OSI interface strings. Used for debugging
	pub fn AcpiUpdateInterfaces(Action: u8) -> ACPI_STATUS;
	/// Remove an interface from the list of interfaces recognized by the _OSI predefined method.
	pub fn AcpiRemoveInterface(InterfaceName: ACPI_STRING) -> ACPI_STATUS;
	/// Install or remove a handler for _OSI invocations.
	pub fn AcpiInstallInterfaceHandler(Handler: ACPI_INTERFACE_HANDLER) -> ACPI_STATUS;
}