// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/acpica/shim_out.rs
//! ACPICA inbound bindings
#![allow(non_snake_case)]
#![allow(private_no_mangle_fns)]
#![allow(dead_code)]
use _common::*;
use super::shim_ext::*;

struct va_list(*const ());

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsInitialize() -> ACPI_STATUS {
	AE_OK
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsTerminate() -> ACPI_STATUS {
	AE_OK
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsGetRootPointer() -> ACPI_PHYSICAL_ADDRESS {
	//if( gACPI_RSDPOverride )
	//	return gACPI_RSDPOverride;	

	let mut val = 0;
	// SAFE: Called from within ACPI init context
	match unsafe { AcpiFindRootPointer(&mut val) }
	{
	AE_OK => {},
	e @ _ => {
		log_notice!("Failed to find ACPI root pointer : {}", e);
		return 0;
		},
	}
	
	val as ACPI_PHYSICAL_ADDRESS
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsPredefinedOverride(_PredefinedObject: *const ACPI_PREDEFINED_NAMES, NewValue: &mut ACPI_STRING) -> ACPI_STATUS {
	*NewValue = 0 as *mut _;
	AE_OK
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsTableOverride(_ExisitingTable: *mut ACPI_TABLE_HEADER, NewTable: &mut *const ACPI_TABLE_HEADER) -> ACPI_STATUS {
	*NewTable = 0 as *const _;
	AE_OK
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsPhysicalTableOverride(_ExisitingTable: *mut ACPI_TABLE_HEADER, NewAddress: &mut ACPI_PHYSICAL_ADDRESS, _NewTableLength: &mut u32) -> ACPI_STATUS {
	*NewAddress = 0;
	AE_OK
}

// -- Memory Management ---
// ------------------------
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsMapMemory(PhysicalAddress: ACPI_PHYSICAL_ADDRESS, Length: ACPI_SIZE) -> *mut () {
	let phys_page = PhysicalAddress & !0xFFF;
	let ofs = (PhysicalAddress & 0xFFF) as usize;
	let npages = (ofs + Length + 0xFFF) / 0x1000;
	
	// SAFE: Trusting ACPI not to do anything stupid
	unsafe {
		let mut handle = match ::memory::virt::map_hw_rw(phys_page, npages, module_path!())
			{
			Ok(h) => h,
			Err(e) => return 0 as *mut _,
			};
		
		let rv: *mut () = handle.as_mut::<u8>(ofs) as *mut u8 as *mut ();
		::core::mem::forget(handle);
		rv
	}
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsUnmapMemory(LogicalAddress: *mut (), Length: ACPI_SIZE) {
	let ofs = (LogicalAddress as usize) & 0xFFF;
	let npages = (ofs + Length + 0xFFF) / 0x1000;
	// SAFE: Trusting ACPICA not to pass us a bad pointer
	unsafe {
		::memory::virt::unmap( ((LogicalAddress as usize) - ofs) as *mut (), npages );
	}
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsGetPhysicalAddress(LogicalAddress: *const (), PhysicalAddress: &mut ACPI_PHYSICAL_ADDRESS) -> ACPI_STATUS {
	unimplemented!();
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsAllocate(Size: ACPI_SIZE) -> *mut () {
	// SAFE: (called from external, trust it)
	unsafe { ::memory::heap::malloc(Size) }
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsFree(Memory: *mut ()) {
	// SAFE: (called from external, trust it)
	unsafe { ::memory::heap::free(Memory) }
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
	::threads::get_thread_id() as ACPI_THREAD_ID
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
	// SAFE: Transmutes Box to *mut to forget the box. Will be recreated to drop
	unsafe {
		let mutex = ::sync::Mutex::<()>::new( () );
		*OutHandle = ::core::mem::transmute( Box::new(mutex) );
		AE_OK
	}
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

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsCreateSemaphore(MaxUnits: u32, InitialUnits: u32, OutHandle: *mut ACPI_SEMAPHORE) -> ACPI_STATUS {
	// SAFE: Transmutes Box to *mut to forget the box. Will be recreated to drop
	unsafe {
		let sem = ::sync::Semaphore::new(InitialUnits as isize, MaxUnits as isize);
		*OutHandle = ::core::mem::transmute(Box::new( sem ));
		AE_OK
	}
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsDeleteSemaphore(Handle: ACPI_SEMAPHORE) -> ACPI_STATUS {
	assert!( !Handle.is_null() );
	let boxed: Box<::sync::Semaphore> = unsafe { ::core::mem::transmute(Handle) };
	::core::mem::drop(boxed);
	AE_OK
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsWaitSemaphore(Handle: ACPI_SEMAPHORE, Units: u32, Timeout: u16) -> ACPI_STATUS {
	unsafe {
		if Units != 1 {
			todo!("AcpiOsWaitSemaphore - Support multiple unit acquire");
		}
		else {
			(*Handle).acquire();
		}
		AE_OK
	}
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsSignalSemaphore(Handle: ACPI_SEMAPHORE, Units: u32) -> ACPI_STATUS {
	unsafe {
		if Units != 1 {
			todo!("AcpiOsWaitSemaphore - Support multiple unit release");
		}
		else {
			(*Handle).release();
		}
		AE_OK
	}
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsCreateLock(OutHandle: *mut ACPI_SPINLOCK) -> ACPI_STATUS {
	// SAFE: Transmutes Box to *mut to forget the box. Will be recreated to drop
	unsafe {
		let mutex = ::sync::Spinlock::<()>::new( () );
		*OutHandle = Box::new(mutex).into_ptr();
		AE_OK
	}
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsDeleteLock(Handle: ACPI_SPINLOCK) {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsAcquireLock(Handle: ACPI_SPINLOCK) -> ACPI_CPU_FLAGS {
	unsafe {
		(*Handle).unguarded_lock();
		0
	}
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsReleaseLock(Handle: ACPI_SPINLOCK, Flags: ACPI_CPU_FLAGS) {
	unsafe {
		(*Handle).unguarded_release();
	}
}

// -- Interrupt handling --
// ------------------------
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsInstallInterruptHandler(InterruptLevel: u32, Handler: ACPI_OSD_HANDLER, Context: *const ()) -> ACPI_STATUS {
	unimplemented!()
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsRemoveInterruptHandler(InterruptLevel: u32, Handler: ACPI_OSD_HANDLER) -> ACPI_STATUS {
	unimplemented!()
}

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
// NOTE: AcpiOsPrintf is handled by the acrust.h header
#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsVprintf(Format: *const i8, Args: va_list)
{
	let fmt = ::core::str::from_utf8( ::memory::c_string_as_byte_slice(Format).unwrap_or(b"INVALID") ).unwrap_or("UTF-8");
	log_trace!("AcpiOsVprintf: Format='{}'", fmt);
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

