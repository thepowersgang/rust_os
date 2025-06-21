//! 8.3 ACPI Namespace Management
use super::common::*;
use super::ACPI_STATUS;
use super::Void;

// AcpiAttachData, etc
pub type ACPI_OBJECT_HANDLER = extern "C" fn (Object: ACPI_HANDLE, Data: *const Void);
// AcpiWalkNamespace
/// Interface to the user function that is invoked from AcpiWalkNamespace.
pub type ACPI_WALK_CALLBACK = extern "C" fn (Object: ACPI_HANDLE, NestingLevel: u32, Context: *const Void, ReturnValue: *mut *const Void);

extern "C" {	
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
