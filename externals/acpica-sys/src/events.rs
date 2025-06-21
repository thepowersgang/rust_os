//! 8.6 ACPI Fixed Event Management
use super::ACPI_STATUS;

pub type ACPI_EVENT_HANDLER = extern "C" fn(Context: *mut super::Void);

pub type ACPI_EVENT_STATUS = u32;

pub const ACPI_EVENT_PMTIMER     : u32 = 0;
pub const ACPI_EVENT_GLOBAL      : u32 = 1;
pub const ACPI_EVENT_POWER_BUTTON: u32 = 2;
pub const ACPI_EVENT_SLEEP_BUTTON: u32 = 3;
pub const ACPI_EVENT_RTC         : u32 = 4;

extern "C" {
	pub fn AcpiEnableEvent(Event: u32, Flags: u32) -> ACPI_STATUS;
	pub fn AcpiDisableEvent(Event: u32, Flags: u32) -> ACPI_STATUS;
	pub fn AcpiClearEvent(Event: u32) -> ACPI_STATUS;
	pub fn AcpiGetEventStatus(Event: u32, EventStatus: *mut ACPI_EVENT_STATUS) -> ACPI_STATUS;

	pub fn AcpiInstallFixedEventHandler(Event: u32, Handler: ACPI_EVENT_HANDLER, Context: *mut super::Void) -> ACPI_STATUS;
	pub fn AcpiRemoveFixedEventHandler(Event: u32, Handler: ACPI_EVENT_HANDLER) -> ACPI_STATUS;
}