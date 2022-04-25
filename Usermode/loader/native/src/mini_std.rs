//! A lightweight abstraction of native OS APIs
//! 
//! Includes
//! - `Socket` - A very basic TCP socket wrapper (nullable for use in a static)
//! - `Mutex` - A non-blocking mutex

/// "Plain-old-data" - Types that contain no invalid bit patterns or undefined data
pub unsafe trait Pod: Default {}
unsafe impl Pod for u32 {}

/// Formattable wrapper around a NUL terminated C string
pub struct CStrPtr(*const libc::c_char);
impl CStrPtr {
	pub unsafe fn new(p: *const libc::c_char) -> CStrPtr {
		CStrPtr(p)
	}
}
impl ::core::fmt::Display for CStrPtr {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		use core::fmt::Write;
		// SAFE: Ensured in constructor
		unsafe {
			let mut p = self.0;
			while *p != 0 {
				f.write_char(*p as u8 as char)?;
				p = p.offset(1);
			}
		}
		Ok(())
	}
}

/// Representation of libc's errno
pub struct Errno(i32);
#[cfg(unix)]
impl Errno
{
	pub fn get() -> Errno {
		// SAFE: valid use of libc
		unsafe {
			Errno(*libc::__errno_location())
		}
	}
}
impl ::core::fmt::Display for Errno {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		// SAFE: Trusting strerror
		write!(f, "{} {}", self.0, unsafe { CStrPtr::new(libc::strerror(self.0)) })
	}
}

#[cfg(unix)]
extern crate libc;

#[cfg(unix)]
pub fn mmap_alloc(addr: *mut u8, page_count: usize) -> Result<(),Errno> {
	// SAFE: We're just allocating memory
	let rv = unsafe { libc::mmap(
		addr as *mut _,
		page_count * 0x1000,
		libc::PROT_READ | libc::PROT_WRITE,
		libc::MAP_ANONYMOUS | libc::MAP_PRIVATE | libc::MAP_FIXED | libc::MAP_FIXED_NOREPLACE,
		/*fd=*/0,
		/*offset=*/0
		) };
	if rv == libc::MAP_FAILED {
		Err(Errno::get())
	}
	else if rv != addr as *mut _ {
		todo!("MEM_ALLOCATE({:p}, {}p): failed {:p}", addr, page_count, rv);
	}
	else {
		Ok( () )
	}		
}

#[cfg(windows)]
struct WinapiError(u32);
#[cfg(windows)]
impl WinapiError {
	fn get() -> WinapiError {
		// SAFE: Function has no side-effects
		WinapiError(unsafe { ::winapi::um::errhandlingapi::GetLastError() })
	}
}
#[cfg(windows)]
impl ::core::fmt::Display for WinapiError {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "{:#x}", self.0)
	}
}
#[cfg(windows)]
pub fn mmap_alloc(addr: *mut u8, page_count: usize) -> Result<(),Errno> {
	let mut si: ::winapi::um::sysinfoapi::SYSTEM_INFO;
	// SAFE: Type is POD, valid call to GetSystemInfo
	unsafe {
		si = ::core::mem::zeroed();
		::winapi::um::sysinfoapi::GetSystemInfo(&mut si);
		println!("si.dwPageSize = {:#x}", si.dwPageSize);
		println!("si.dwAllocationGranularity = {:#x}", si.dwAllocationGranularity);
	}
	fn dump_info(addr: *mut u8) {
		// SAFE: Valid pointers
		unsafe {
			let mut b: ::winapi::um::winnt::MEMORY_BASIC_INFORMATION = ::core::mem::zeroed();
			::winapi::um::memoryapi::VirtualQuery(addr as *const _, &mut b, ::core::mem::size_of_val(&b));
			println!("MEMORY_BASIC_INFORMATION {{ BaseAddress: {:p}, AllocationBase: {:p}, AllocationProtect: {}, RegionSize: {:#x}, State: {:#x}, Protect: {}, Type: {:#x} }}",
				b.BaseAddress,
				b.AllocationBase,
				b.AllocationProtect,
				b.RegionSize,
				b.State,
				b.Protect,
				b.Type,
				);
		}
	}

	use ::winapi::um::winnt::*;
	{
		let resv_base = addr as usize & !(si.dwAllocationGranularity as usize - 1);
		let resv_extra = addr as usize - resv_base;
		let resv_bytes = resv_extra + (page_count * 0x1000);
		let resv_bytes = (resv_bytes + (si.dwAllocationGranularity as usize - 1)) & !(si.dwAllocationGranularity as usize - 1);
		let resv_count = resv_bytes / si.dwAllocationGranularity as usize;
		for i in 0 .. resv_count
		{
			let resv_base = resv_base + i * si.dwAllocationGranularity as usize;
			println!("MEM_RESERVE {:p}", resv_base as *mut u8);
			// SAFE: We're just allocating memory
			let rv = unsafe { ::winapi::um::memoryapi::VirtualAlloc(
				resv_base as *mut _,
				si.dwAllocationGranularity as usize,
				MEM_RESERVE,
				PAGE_READWRITE
				) };
			if rv == ::core::ptr::null_mut() {
				println!("MEM_RESERVE: error {}", WinapiError::get());
			}

		}
	}

	for p in 0 .. page_count
	{
		// SAFE: We're just allocating memory
		let rv = unsafe { ::winapi::um::memoryapi::VirtualAlloc(
			(addr as usize + p * 0x1000) as *mut _,
			0x1000,
			MEM_COMMIT /*| MEM_RESERVE*/,
			PAGE_READWRITE
			) };
		if rv == ::core::ptr::null_mut() {
			let e = WinapiError::get();
			dump_info((addr as usize - 0x1000) as *mut _);
			dump_info(addr);
			dump_info((addr as usize + 0x1000) as *mut _);
			todo!("mmap_alloc: error {}", e);
		}
	}
	dump_info(addr);
	Ok( () )
}

/// A nullable TCP socket
pub struct Socket(Option<::std::net::TcpStream>);
impl Socket {
	pub const fn null() -> Socket {
		Socket(None)
	}

	/// Connect to IPv4 localhost on the specified port
	pub fn connect_localhost(port: u16) -> Result<Socket, &'static str> {
		let sock = ::std::net::TcpStream::connect( ("127.0.0.1", port) ).map_err(|_| "connect failed")?;
		sock.set_nodelay(true).expect("failed to set TCP_NODELAY");	// Used to ensure that syscall latency is low
		Ok(Socket(Some(sock)))
	}
	/// Reeive a POD structure
	pub fn recv<T: Pod>(&self) -> Result<T,&'static str> {
		use std::io::Read;
		let mut rv = T::default();
		// SAFE: Correct pointers, data is POD
		let slice = unsafe { ::std::slice::from_raw_parts_mut(&mut rv as *mut _ as *mut u8, ::core::mem::size_of::<T>()) };
		self.0.as_ref().unwrap().read_exact(slice).map_err(|_| "Error reported")?;
		Ok(rv)
	}
	/// Send a POD structure
	pub fn send<T: Pod>(&self, val: T) -> Result<(), &'static str> {
		use std::io::Write;
		// SAFE: Correct pointers, and data is POD
		let slice = unsafe { ::std::slice::from_raw_parts(&val as *const _ as *const u8, ::core::mem::size_of::<T>()) };
		self.0.as_ref().unwrap().write_all(slice).map_err(|_| "Error reported")?;
		Ok( () )
	}
}

pub struct Mutex<T>
{
	locked: ::core::sync::atomic::AtomicBool,
	data: ::core::cell::UnsafeCell<T>,
}
unsafe impl<T> Send for Mutex<T> where T: Send {}
unsafe impl<T> Sync for Mutex<T> where T: Sync {}
impl<T> Mutex<T>
{
	pub const fn new(v: T) -> Mutex<T> {
		Mutex {
			locked: ::core::sync::atomic::AtomicBool::new(false),
			data: ::core::cell::UnsafeCell::new(v),
		}
	}

	pub fn lock(&self) -> HeldMutex<T> {
		assert!(!self.locked.swap(true, ::core::sync::atomic::Ordering::Acquire));
		HeldMutex(self)
	}
}
pub struct HeldMutex<'a, T: 'a>(&'a Mutex<T>);
impl<'a, T: 'a> ::core::ops::Drop for HeldMutex<'a, T>
{
	fn drop(&mut self)
	{
		assert!( self.0.locked.swap(false, ::core::sync::atomic::Ordering::Acquire) );
	}
}
impl<'a, T: 'a> ::core::ops::Deref for HeldMutex<'a, T>
{
	type Target = T;
	fn deref(&self) -> &T
	{
		// SAFE: Locked
		unsafe {
			&*self.0.data.get()
		}
	}
}
impl<'a, T: 'a> ::core::ops::DerefMut for HeldMutex<'a, T>
{
	fn deref_mut(&mut self) -> &mut T
	{
		// SAFE: Locked
		unsafe {
			&mut *self.0.data.get()
		}
	}
}
