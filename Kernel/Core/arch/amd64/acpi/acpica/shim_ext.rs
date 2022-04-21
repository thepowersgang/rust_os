// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/acpica/shim_ext.rs
//! ACPICA outbound bindings
#![allow(non_camel_case_types,non_snake_case)]	// follows C naming conventions
#![allow(dead_code)]	// API, may not be used
use core::fmt;

use crate::Void;

pub type ACPI_SIZE = usize;
pub type ACPI_PHYSICAL_ADDRESS = crate::memory::PAddr;
pub type ACPI_IO_ADDRESS = u16;

pub type ACPI_PCI_ID = u32;

pub type ACPI_CPU_FLAGS = u32;
pub type ACPI_SPINLOCK = *const crate::sync::Spinlock<()>;
pub type ACPI_MUTEX = *const crate::sync::Mutex<()>;
pub type ACPI_SEMAPHORE = *const crate::sync::Semaphore;

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
pub struct ACPI_BUFFER
{
	Length: u32, 
	Pointer: *const Void,
}

#[repr(C)]
pub struct ACPI_OBJECT
{
	Type: ACPI_OBJECT_TYPE,
	
	int_data: [u64; 2],
	//Any,
	//Integer(u64),
	//String(u32, *const i8),
	//Buffer(u32, *const u8),
	//Package(u32, *const ACPI_OBJECT),
	//Reference(ACPI_OBJECT_TYPE, ACPI_HANDLE),
	//Processor(u32, ACPI_IO_ADDRESS, u32),
	//PowerResource(u32, u32),
}
#[repr(C)]
pub struct ACPI_HANDLE(*mut Void);
#[repr(C)]
pub struct ACPI_OBJECT_LIST
{
	Count: u32,
	Pointer: *const ACPI_OBJECT,
}
pub type ACPI_OBJECT_TYPE = u32;
pub const ACPI_TYPE_ANY         : ACPI_OBJECT_TYPE = 0;
pub const ACPI_TYPE_INTEGER     : ACPI_OBJECT_TYPE = 1;
pub const ACPI_TYPE_STRING      : ACPI_OBJECT_TYPE = 2;
pub const ACPI_TYPE_BUFFER      : ACPI_OBJECT_TYPE = 3;
pub const ACPI_TYPE_PACKAGE     : ACPI_OBJECT_TYPE = 4;
pub const ACPI_TYPE_FIELD_UNIT  : ACPI_OBJECT_TYPE = 5;
pub const ACPI_TYPE_DEVICE      : ACPI_OBJECT_TYPE = 6;
pub const ACPI_TYPE_EVENT       : ACPI_OBJECT_TYPE = 7;
pub const ACPI_TYPE_METHOD      : ACPI_OBJECT_TYPE = 8;
pub const ACPI_TYPE_MUTEX       : ACPI_OBJECT_TYPE = 9;
pub const ACPI_TYPE_REGION      : ACPI_OBJECT_TYPE = 10;
pub const ACPI_TYPE_POWER       : ACPI_OBJECT_TYPE = 11;
pub const ACPI_TYPE_PROCESSOR   : ACPI_OBJECT_TYPE = 12;
pub const ACPI_TYPE_THERMAL     : ACPI_OBJECT_TYPE = 13;
pub const ACPI_TYPE_BUFFER_FIELD: ACPI_OBJECT_TYPE = 14;
pub const ACPI_TYPE_DDB_HANDLE  : ACPI_OBJECT_TYPE = 15;
pub const ACPI_TYPE_DEBUG_OBJECT: ACPI_OBJECT_TYPE = 16;


#[repr(C)]
pub struct ACPI_PREDEFINED_NAMES
{
	Name: *const u8,
	Type: i8,
	Val: *const u8,
}
pub type ACPI_STRING = *mut u8;

pub type ACPI_THREAD_ID = u64;

#[repr(C)]
#[derive(PartialEq,Eq)]
pub struct ACPI_STATUS(i32);

const AE_CODE_ENVIRONMENTAL: i32 = 0x0000;
const AE_CODE_PROGRAMMER   : i32 = 0x1000;

pub const AE_OK             : ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL| 0);
pub const AE_ERROR          : ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL| 1);
pub const AE_NO_ACPI_TABLES : ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL| 2);
pub const AE_NO_MEMORY      : ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL| 4);
pub const AE_NOT_FOUND      : ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL| 5);
pub const AE_NOT_IMPLEMENTED: ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL|14);
pub const AE_BAD_PARAMETER: ACPI_STATUS = ACPI_STATUS(AE_CODE_PROGRAMMER|1);

#[repr(C)]
pub struct ACPI_TABLE_DESC(crate::Void);

pub type ACPI_TABLE_HEADER = super::super::SDTHeader;

// AcpiInstallInitializationHandler
pub type ACPI_INIT_HANDLER = extern "C" fn(Object: ACPI_HANDLE, Function: u32) -> ACPI_STATUS;
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
// AcpiInstallInterfaceHandler
pub type ACPI_INTERFACE_HANDLER = extern "C" fn(InterfaceName: ACPI_STRING, Supported: u32) -> u32;
// AcpiInstallTableHandler
pub type ACPI_TABLE_HANDLER = extern "C" fn (Event: u32, Table: *const Void, Context: *const Void) -> ACPI_STATUS;
// AcpiGetObjectInfo
#[repr(C)]
pub struct ACPI_DEVICE_INFO
{
	InfoSize: u32,
	Name: u32,
	Type: ACPI_OBJECT_TYPE,
	ParamCount: u8,
	Valid: u8,
	Flags: u8,
	HighestDstates: [u8; 4],
	LowestDstates: [u8; 5],
	CurrentStatus: u32,
	Address: u64,
	HardwareId: ACPI_PNP_DEVICE_ID,
	UniqueId: ACPI_PNP_DEVICE_ID,
	SubsystemId: ACPI_PNP_DEVICE_ID,
	CompatibleIdList: ACPI_PNP_DEVICE_ID_LIST,
}
#[repr(C)]
pub struct ACPI_PNP_DEVICE_ID
{
	Length: u32,
	String: *const i8,
}
#[repr(C)]
pub struct ACPI_PNP_DEVICE_ID_LIST
{
	Count: u32,
	ListSize: u32,
	Ids: [ACPI_PNP_DEVICE_ID; 0],
}
// AcpiAttachData, etc
pub type ACPI_OBJECT_HANDLER = extern "C" fn (Object: ACPI_HANDLE, Data: *const Void);
// AcpiWalkNamespace
/// Interface to the user function that is invoked from AcpiWalkNamespace.
pub type ACPI_WALK_CALLBACK = extern "C" fn (Object: ACPI_HANDLE, NestingLevel: u32, Context: *const Void, ReturnValue: *mut *const Void);

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

	// 8.2 ACPI Table Management
	/// Initialize the ACPICA table manager.
	pub fn AcpiInitializeTables(InitialStorage: *mut ACPI_TABLE_DESC, InitialTableCount: u32, AllowResize: bool) -> ACPI_STATUS;
	/// Copy the root ACPI information table into dynamic memory.
	pub fn AcpiReallocateRootTable() -> ACPI_STATUS;
}
#[allow(improper_ctypes)]
extern "C" {
	/// Locate the RSDP via memory scan (IA-32).
	pub fn AcpiFindRootPointer(TableAddress: *mut ACPI_SIZE) -> ACPI_STATUS;
}
extern "C" {
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
	
	// 8.3 ACPI Namespace Management
	/// Evaluate an ACPI namespace object and return the result.
	pub fn AcpiEvaluateObject(Object: ACPI_HANDLE, Pathname: ACPI_STRING, MethodParams: *const ACPI_OBJECT_LIST, ReturnBuffer: *mut ACPI_BUFFER) -> ACPI_STATUS;
	/// Evaluate an ACPI namespace object and return the type-validated result.
	pub fn AcpiEvaluateObjectTyped(Object: ACPI_HANDLE, Pathname: ACPI_STRING, MethodParams: *const ACPI_OBJECT_LIST, ReturnBuffer: *mut ACPI_BUFFER, ReturnType: ACPI_OBJECT_TYPE) -> ACPI_STATUS;
	/// Get information about an ACPI namespace object.
	pub fn AcpiGetObjectInfo(Object: ACPI_HANDLE, OutBuffer: *mut *const ACPI_DEVICE_INFO) -> ACPI_STATUS;
	/// Get a handle to the next child ACPI object of a parent object.
	pub fn AcpiGetNextObject(Type: ACPI_OBJECT_TYPE, Parent: ACPI_HANDLE, Child: ACPI_HANDLE, OutHandle: *mut ACPI_HANDLE) -> ACPI_STATUS;
	/// Get a handle to the parent object of an ACPI object.
	pub fn AcpiGetParent(Child: ACPI_HANDLE, OutParent: *mut ACPI_HANDLE) -> ACPI_STATUS;
	/// Get the type of an ACPI object.
	pub fn AcpiGetType(Object: ACPI_HANDLE, OutType: *mut ACPI_OBJECT_TYPE) -> ACPI_STATUS;
	/// Get the object handle associated with an ACPI name.
	pub fn AcpiGetHandle(Parent: ACPI_HANDLE, Pathname: ACPI_STRING, OutHandle: *mut ACPI_HANDLE) -> ACPI_STATUS;
	/// Walk the ACPI namespace to find all objects of type Device.
	pub fn AcpiGetDevices(HID: *const u8, UserFunction: ACPI_WALK_CALLBACK, UserContext: *const  Void, ReturnValue: *mut *const  Void) -> ACPI_STATUS;
	/// Attach user data to an ACPI namespace object.
	pub fn AcpiAttachData(Object: ACPI_HANDLE, Handler: ACPI_OBJECT_HANDLER, Data: *const  Void) -> ACPI_STATUS;
	/// Remove a data attachment to a namespace object.
	pub fn AcpiDetachData(Object: ACPI_HANDLE, Handler: ACPI_OBJECT_HANDLER) -> ACPI_STATUS;
	/// Retrieve data that was associated with a namespace object.
	pub fn AcpiGetData(Object: ACPI_HANDLE, Handler: ACPI_OBJECT_HANDLER, Data: *mut *const  Void) -> ACPI_STATUS;
	/// Install a single control method into the namespace.
	pub fn AcpiInstallMethod(TableBuffer: *const u8) -> ACPI_STATUS;
	/// Traverse a portion of the ACPI namespace to find objects of a given type.
	pub fn AcpiWalkNamespace(Type: ACPI_OBJECT_TYPE, StartObject: ACPI_HANDLE, MaxDepth: u32, DescendingCallback: ACPI_WALK_CALLBACK, AscendingCallback: ACPI_WALK_CALLBACK, UserContext: *const  Void, ReturnValue: *mut *const  Void) -> ACPI_STATUS;
	/// Acquire an AML Mutex object.
	pub fn AcpiAcquireMutex(Parent: ACPI_HANDLE, Pathname: ACPI_STRING, Timeout: u16) -> ACPI_STATUS;
	/// Release an AML Mutex object.
	pub fn AcpiReleaseMutex(Parent: ACPI_HANDLE, Pathname: ACPI_STRING) -> ACPI_STATUS;
	
	// 8.4 ACPI Hardware Management
	/// Put the system into ACPI mode.
	pub fn AcpiEnable() -> ACPI_STATUS;
	/// Take the system out of ACPI mode.
	pub fn AcpiDisable() -> ACPI_STATUS;
	// ...	

	// 8.5 ACPI Sleep/Wake Support
	// 8.6 ACPI Fixed Event Management
	// 8.7 ACPI General Purpose Event (GPE) Management
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

