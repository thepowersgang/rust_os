// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/acpica/os_int.rs
//! ACPICA OS bindings
#![allow(non_snake_case)]
#![allow(dead_code,unused_variables)]
use crate::prelude::*;
use super::shim_ext::*;
use super::va_list::VaList;

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
	log_trace!("AcpiOsMapMemory({:#x}, {})", PhysicalAddress, Length);
	let phys_page = PhysicalAddress & !0xFFF;
	let ofs = (PhysicalAddress & 0xFFF) as usize;
	let npages = (ofs + Length + 0xFFF) / 0x1000;
	
	// SAFE: Trusting ACPI not to do anything stupid
	unsafe {
		let mut handle = match crate::memory::virt::map_hw_rw(phys_page, npages, module_path!())
			{
			Ok(h) => h,
			Err(e) => return 0 as *mut _,
			};
		
		let rv: *mut () = handle.as_mut::<u8>(ofs) as *mut u8 as *mut ();
		//if Length < 1024 {
		//	::logging::hex_dump( "AcpiOsMapMemory", ::core::slice::from_raw_parts(rv as *const u8, Length) );
		//}
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
		crate::memory::virt::unmap( ((LogicalAddress as usize) - ofs) as *mut (), npages );
	}
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsGetPhysicalAddress(LogicalAddress: *const (), PhysicalAddress: &mut ACPI_PHYSICAL_ADDRESS) -> ACPI_STATUS {
	unimplemented!();
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsAllocate(Size: ACPI_SIZE) -> *mut () {
	// SAFE: (called from external, trust it)
	unsafe { crate::memory::heap::malloc(Size) }
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsFree(Memory: *mut ()) {
	// SAFE: (called from external, trust it)
	unsafe { crate::memory::heap::free(Memory) }
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
	// 0 is special to ACPICA, so offset by one
	// - This is just used by ACPICA, so offsetting by one is safe
	(crate::threads::get_thread_id() + 1) as ACPI_THREAD_ID
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
		let mutex = crate::sync::Mutex::<()>::new( () );
		*OutHandle = Box::into_raw(Box::new(mutex));
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
		let sem = crate::sync::Semaphore::new(InitialUnits as isize, MaxUnits as isize);
		*OutHandle = Box::into_raw(Box::new(sem));
		AE_OK
	}
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsDeleteSemaphore(Handle: ACPI_SEMAPHORE) -> ACPI_STATUS {
	assert!( !Handle.is_null() );
	// SAFE: ACPICA should pass us a valid handle
	let boxed: Box<crate::sync::Semaphore> = unsafe { Box::from_raw(Handle as *mut _)};
	::core::mem::drop(boxed);
	AE_OK
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsWaitSemaphore(Handle: ACPI_SEMAPHORE, Units: u32, Timeout: u16) -> ACPI_STATUS {
	// SAFE: ACPICA should pass us a valid handle
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
	// SAFE: ACPICA should pass us a valid handle
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
		let mutex = crate::sync::Spinlock::<()>::new( () );
		*OutHandle = Box::into_raw(Box::new(mutex));
		AE_OK
	}
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsDeleteLock(Handle: ACPI_SPINLOCK) {
	unimplemented!();
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsAcquireLock(Handle: ACPI_SPINLOCK) -> ACPI_CPU_FLAGS {
	// SAFE: ACPICA should pass us a valid handle
	unsafe {
		(*Handle).unguarded_lock();
		0
	}
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsReleaseLock(Handle: ACPI_SPINLOCK, Flags: ACPI_CPU_FLAGS) {
	// SAFE: ACPICA should pass us a valid handle
	unsafe {
		(*Handle).unguarded_release();
	}
}

// -- Interrupt handling --
// ------------------------
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsInstallInterruptHandler(InterruptLevel: u32, Handler: ACPI_OSD_HANDLER, Context: *const ()) -> ACPI_STATUS {
	log_warning!("TODO: AcpiOsInstallInterruptHandler(InterruptLevel={}, Handler={:p}, Context={:p}",
		InterruptLevel, Handler as *const (), Context);
	// Doesn't do anything... yet. I don't have any idea what this interrupt actually is
	AE_OK
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsRemoveInterruptHandler(InterruptLevel: u32, Handler: ACPI_OSD_HANDLER) -> ACPI_STATUS {
	unimplemented!()
}

// -- Memory Access --
// -------------------
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsReadMemory(Address: ACPI_PHYSICAL_ADDRESS, Value: *mut u64, Width: u32) -> ACPI_STATUS
{
	unimplemented!()
}
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsWriteMemory(Address: ACPI_PHYSICAL_ADDRESS, Value: u64, Width: u32) -> ACPI_STATUS
{
	unimplemented!()
}

// -- Port Input / Output --
// -------------------------
#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsReadPort(Address: ACPI_IO_ADDRESS, Value: &mut u32, Width: u32) -> ACPI_STATUS
{
	assert!(Value as *mut u32 != 0 as *mut u32);
	// ACPI_IO_ADDRESS is u16
	//assert!(Address < (1<<16), "AcpiOsReadPort: Address out of range - {:#x} >= {:#x}", Address, (1<<16));
	// SAFE: We're trusting ACPICA here
	unsafe {
		match Width
		{
		 8 => {
			*Value = crate::arch::x86_io::inb(Address as u16) as u32;
			},
		16 => {
			assert!(Address % 2 == 0);
			*Value = crate::arch::x86_io::inw(Address as u16) as u32;
			},
		32 => {
			assert!(Address % 4 == 0);
			*Value = crate::arch::x86_io::inl(Address as u16);
			},
		_ => return AE_NOT_IMPLEMENTED,
		}
	};
	log_trace!("AcpiOsReadPort({:#x}, Width={}) = {:#x}", Address, Width, Value);
	AE_OK
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsWritePort(Address: ACPI_IO_ADDRESS, Value: u32, Width: u32) -> ACPI_STATUS
{
	log_trace!("AcpiOsWritePort({:#x}, Value={:#x}, Width={})", Address, Value, Width);
	// SAFE: We're trusting ACPICA here
	unsafe {
		match Width
		{
		 8 => {
			crate::arch::x86_io::outb(Address as u16, Value as u8);
			},
		16 => {
			assert!(Address % 2 == 0);
			crate::arch::x86_io::outw(Address as u16, Value as u16);
			},
		32 => {
			assert!(Address % 4 == 0);
			crate::arch::x86_io::outl(Address as u16, Value as u32);
			},
		_ => return AE_NOT_IMPLEMENTED,
		}
	}
	AE_OK
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

unsafe fn c_string_to_str<'a>(c_str: *const i8) -> &'a str {
	::core::str::from_utf8( crate::memory::c_string_as_byte_slice(c_str).unwrap_or(b"INVALID") ).unwrap_or("UTF-8")
}
fn get_uint(Args: &mut VaList, size: usize) -> u64 {
	// (uncheckable) SAFE: Could over-read from stack, returning junk
	unsafe {
		match size
		{
		0 => Args.get::<u32>() as u64,
		1 => Args.get::<u32>() as u64,
		2 => Args.get::<u64>(),
		_ => unreachable!(),
		}
	}
}
fn get_int(Args: &mut VaList, size: usize) -> i64 {
	// (uncheckable) SAFE: Could over-read from stack, returning junk
	unsafe {
		match size
		{
		0 => Args.get::<i32>() as i64,
		1 => Args.get::<i32>() as i64,
		2 => Args.get::<i64>(),
		_ => unreachable!(),
		}
	}
}

// -- Formatted Output --
// ----------------------
// NOTE: AcpiOsPrintf is handled by the acrust.h header
#[no_mangle]
#[linkage="external"]
#[allow(dead_code)]
extern "C" fn AcpiOsVprintf(Format: *const i8, mut Args: VaList)
{
	use crate::sync::mutex::LazyMutex;
	struct Buf([u8; 256]);
	impl Buf {
		fn new() -> Self {
			// SAFE: POD
			unsafe { ::core::mem::zeroed() }
		}
	}
	impl AsMut<[u8]> for Buf { fn as_mut(&mut self) -> &mut [u8] { &mut self.0 } }
	impl AsRef<[u8]> for Buf { fn as_ref(&self) -> &[u8] { &self.0 } }
	static TEMP_BUFFER: LazyMutex<crate::lib::string::FixedString<Buf>> = LazyMutex::new();

	// Acquire input and lock	
	// SAFE: Format string is valid for function
	let fmt = unsafe { c_string_to_str(Format) };
	let mut lh = TEMP_BUFFER.lock_init(|| crate::lib::string::FixedString::new(Buf::new()));
	
	// Expand format string
	let mut it = fmt.chars();
	while let Some(mut c) = it.next()
	{
		if c == '\n' {
			// Flush
			log_debug!("{}", *lh);
			lh.clear();
		}
		else if c != '%' {
			lh.push_char(c);
		}
		else {
			use core::fmt::Write;
			
			c = match it.next() { Some(v)=>v,_=>return };
			
			let mut align_left = false;
			if c == '-' {
				align_left = true;
				c = match it.next() { Some(v)=>v,_=>return };
			}
			
			let mut width = 0;
			while let Some(d) = c.to_digit(10) {
				width = width * 10 + d;
				c = match it.next() { Some(v)=>v,_=>return };
			}
			
			let mut precision = !0;
			if c == '.' {
				precision = 0;
				c = match it.next() { Some(v)=>v,_=>return };
				while let Some(d) = c.to_digit(10) {
					precision = precision * 10 + d;
					c = match it.next() { Some(v)=>v,_=>return };
				}
			}
			
			let size = if c == 'l' {
					c = match it.next() { Some(v)=>v,_=>return };
					if c == 'l' {
						c = match it.next() { Some(v)=>v,_=>return };
						2
					}
					else {
						1
					}
				}
				else {
					0
				};
			
			// TODO: Use undocumented (but public) APIs in ::core::fmt
			// to create an Arguments structure from this information
			//let spec = ::core::fmt::rt::v1::FormatSpec {
			//	fill: ' ',
			//	align: ::core::fmt::rt::v1::Alignment::Unknown,
			//	flags: 0,
			//	precision: ::core::fmt::rt::v1::Count::Is(precision),
			//	width: ::core::fmt::rt::v1::Count::Is(width),
			//	};
			let _ = align_left;
			
			match c
			{
			'x' => {
				let _ = write!(&mut *lh, "{:x}", get_uint(&mut Args, size));
				},
			'X' => {
				let val = get_uint(&mut Args, size);
				let _ = write!(&mut *lh, "{:X}", val);
				},
			'd' => {
				let val = get_int(&mut Args, size);
				let _ = write!(&mut *lh, "{}", val);
				},
			'u' => {
				let val = get_uint(&mut Args, size);
				let _ = write!(&mut *lh, "{}", val);
				},
			'p' => {
				// (uncheckable) SAFE: Could over-read from stack, returning junk
				let _ = write!(&mut *lh, "{:p}", unsafe { Args.get::<*const u8>() });
				},
			'c' => {
				// (uncheckable) SAFE: Could over-read from stack, returning junk
				let _ = write!(&mut *lh, "{}", unsafe { Args.get::<u32>() as u8 as char });
				},
			's' => {
				// SAFE: Does as much validation as possible, if ACPICA misbehaves... well, we're in trouble
				let slice = unsafe {
					let ptr = Args.get::<*const u8>();
					if precision < !0 {
						::core::str::from_utf8(::core::slice::from_raw_parts(ptr, precision as usize)).unwrap_or("")
					}
					else {
						c_string_to_str(ptr as *const i8)
					}
					};
				let _ = write!(&mut *lh, "{}", slice);
				},
			_ => {
				log_error!("AcpiOsVprintf - Unknown format code {}", c);
				},
			}
		}
	}
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
	crate::time::ticks() * 10 * 1000
}

#[no_mangle] #[linkage="external"]
extern "C" fn AcpiOsSignal(Function: u32, Info: *const ()) -> ACPI_STATUS {
	unimplemented!();
}

