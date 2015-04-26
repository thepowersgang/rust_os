// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/acpica/shim_out.rs
//! ACPICA inbound bindings
#![allow(non_snake_case)]
#![allow(private_no_mangle_fns)]
#![allow(dead_code)]
use super::shim_ext::*;

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsInitialize() -> ACPI_STATUS
{
	AE_OK
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsTerminate() -> ACPI_STATUS
{
	AE_OK
}

// -- Memory Management ---
// ------------------------
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsMapMemory(PhysicalAddress: ACPI_PHYSICAL_ADDRESS, Length: ACPI_SIZE) -> *mut () {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsUnmapMemory(LogicalAddress: *mut (), Length: ACPI_SIZE) {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsGetPhysicalAddress(LogicalAddress: *const (), PhysicalAddress: &mut ACPI_PHYSICAL_ADDRESS) -> ACPI_STATUS {
	unimplemented!();
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsAllocate(Size: ACPI_SIZE) -> *mut () {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsFree(Memory: *mut ()) {
	unimplemented!();
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsReadable(Memory: *const (), Length: ACPI_SIZE) -> bool {
	unimplemented!();
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsWritable(Memory: *const (), Length: ACPI_SIZE) -> bool {
	unimplemented!();
}

// -- Thread Managment --
// ----------------------
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsGetThreadId() -> ACPI_THREAD_ID {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsExecute(Type: ACPI_EXECUTE_TYPE, Function: ACPI_OSD_EXEC_CALLBACK, Context: *const ()) -> ACPI_STATUS {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsSleep(Milliseconds: u64) {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsStall(Microseconds: u32) {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsWaitEventsComplete() {
	unimplemented!();
}


// --- Mutexes etc ---
// -------------------
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsCreateMutex(OutHandle: *mut ACPI_MUTEX) -> ACPI_STATUS {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsDeleteMutex(Handle: ACPI_MUTEX) {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsAcquireMutex(Handle: ACPI_MUTEX, Timeout: u16) -> ACPI_STATUS {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsReleaseMutex(Handle: ACPI_MUTEX) {
	unimplemented!();
}

#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsCreateSemaphore(MaxUnits: u32, InitialUnits: u32, OutHandle: *mut ACPI_SEMAPHORE) -> ACPI_STATUS {
	unimplemented!();
}
#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsDeleteSemaphore(Handle: ACPI_SEMAPHORE) -> ACPI_STATUS {
	unimplemented!();
}
#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsWaitSemaphore(Handle: ACPI_SEMAPHORE, Units: u32, Timeout: u16) -> ACPI_STATUS {
	unimplemented!();
}
#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsSignalSemaphore(Handle: ACPI_SEMAPHORE, Units: u32) -> ACPI_STATUS {
	unimplemented!();
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsCreateLock(OutHandle: *mut ACPI_SPINLOCK) -> ACPI_STATUS {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsDeleteLock(Handle: ACPI_SPINLOCK) {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsAcquireLock(Handle: ACPI_SPINLOCK) -> ACPI_CPU_FLAGS {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsReleaseLock(Handle: ACPI_SPINLOCK, Flags: ACPI_CPU_FLAGS) {
	unimplemented!();
}

// -- Interrupt handling --
// ------------------------

// -- Memory Access --
// -------------------
#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsReadMemory(Address: ACPI_PHYSICAL_ADDRESS, Value: *mut u64, Width: u32) -> ACPI_STATUS
{
	unimplemented!()
}
#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsWriteMemory(Address: ACPI_PHYSICAL_ADDRESS, Value: u64, Width: u32) -> ACPI_STATUS
{
	unimplemented!()
}

// -- Port Input / Output --
// -------------------------
#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsReadPort(Address: ACPI_IO_ADDRESS, Value: *mut u32, Width: u32) -> ACPI_STATUS
{
	unimplemented!()
}

#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsWritePort(Address: ACPI_IO_ADDRESS, Value: u32, Width: u32) -> ACPI_STATUS
{
	unimplemented!()
}


// -- PCI Configuration Space Access --
// ------------------------------------
#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsReadPciConfiguration(PciId: ACPI_PCI_ID, Register: u32, Value: *mut u64, Width: u32) -> ACPI_STATUS 
{
	unimplemented!();
}

#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsWritePciConfiguration(PciId: ACPI_PCI_ID, Register: u32, Value: u64, Width: u32) -> ACPI_STATUS 
{
	unimplemented!();
}

// -- Formatted Output --
// ----------------------
#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsPrintf(Format: *const u8/*, ...*/)
{
	unimplemented!()
}

#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsVprintf(Format: *const u8, Args: () /* va_list */)
{
	unimplemented!()
}

#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsRedirectOutput(Destination: *const ())
{
}

// -- Miscellaneous --
// -------------------
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsGetTimer() -> u64 {
	::time::ticks() * 10 * 1000
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsSignal(Function: u32, Info: *const ()) -> ACPI_STATUS {
	unimplemented!();
}



// -- C Library --
// ---------------
#[no_mangle] #[linkage="external"]
unsafe extern "C" fn strcpy(mut dst: *mut u8, mut src: *const u8) -> *mut u8 {
	let rv = dst;
	while *src != 0 {
		*dst = *src;
		dst = dst.offset(1);
		src = src.offset(1);
	}
	dst
}
