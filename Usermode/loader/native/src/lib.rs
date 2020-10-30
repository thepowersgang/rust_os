#![crate_type="cdylib"]
#![feature(rustc_private)]	// libc

extern crate libc;

// TODO: Put this somewhere common, can't load `loader` here
#[derive(Debug)]
pub enum Error
{
	NotFound,
	NotExecutable,
	BadFormat,
	CorruptExecutable,
	BadArguments,
}

#[allow(improper_ctypes_definitions)]
#[no_mangle]
pub extern "C" fn new_process(executable_handle: ::syscalls::vfs::File, process_name: &[u8], args: &[&[u8]]) -> Result<::syscalls::threads::ProtoProcess,Error>
{
	// Send a special syscall that prepares the process
	// - Need to hand the executable handle to the server
	// 1. Pack the arguments into a NUL separated list
	let mut args_packed = Vec::new();
	for a in args {
		args_packed.extend(a.iter().copied());
		args_packed.push(0);
	}
	let name = ::std::str::from_utf8(process_name).unwrap_or("BADSTR");

	match ::syscalls::threads::start_process(executable_handle, name, &args_packed)
	{
	Ok(h) => Ok(h),
	Err(e) => todo!("loader native new_process: error={}", e),
	}
}

#[allow(improper_ctypes_definitions)]
#[no_mangle]
pub extern "C" fn start_process(handle: ::syscalls::threads::ProtoProcess) -> ::syscalls::threads::Process
{
	handle.start(0,0)
}

static mut RUSTOS_NATIVE_SOCKET: mini_std::Socket = mini_std::Socket::null();
static mut RUSTOS_PID: u32 = 0;
const MAX_THREADS: usize = 16;

#[no_mangle]
pub unsafe extern "C" fn rustos_native_init(port: u16)
{
	// SAFE: Called once
	RUSTOS_NATIVE_SOCKET = mini_std::tcp_connect_localhost(port).unwrap();
	let pid: u32 = mini_std::tcp_recv(&RUSTOS_NATIVE_SOCKET).unwrap();
	RUSTOS_PID = pid;
}

fn get_pid() -> u32 {
	unsafe { RUSTOS_PID }
}
fn log(args: ::core::fmt::Arguments) {
	let mut buf = [0; 128];
	let mut c = ::std::io::Cursor::new(&mut buf[..]);
	::std::io::Write::write_fmt(&mut c, args);
	unsafe {
		rustos_native_syscall(::syscalls::values::CORE_LOGWRITE, &[
			c.get_ref().as_ptr() as usize,
			c.position() as usize,
		]);
	}
}
macro_rules! log {
	( $($tt:tt)* ) => {
		log(format_args!($($tt)*))
	}
}

// TODO: use this for sending syscalls
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub unsafe extern "C" fn rustos_native_syscall(id: u32, opts: &[usize]) -> u64 {
	use ::syscalls::values::*;
	match id
	{
	CORE_EXITPROCESS => {
		log!("CORE_EXITPROCESS({} {:#x})", get_pid(), opts[0]);
		::std::process::exit(opts[0] as i32)
		},
	// Memory, wrap mmap
	MEM_ALLOCATE => {
		println!("MEM_ALLOCATE({} {:#x}, {})", get_pid(), opts[0], opts[1]);
		match mini_std::mmap_alloc(opts[0] as *mut _, opts[1])
		{
		Err(e) => {
			println!("MEM_ALLOCATE errno={}", e);
			(1 << 32) | 0
			},
		Ok(_) => 0,
		}
		},
	MEM_REPROTECT => {
		let addr = opts[0];
		let mode = opts[1];
		todo!("MEM_REPROTECT({:#x}, {})", addr, mode);
		}
	MEM_DEALLOCATE => {
		let addr = opts[0];
		todo!("MEM_DEALLOCATE({:#x})", addr);
		},
	_ => {
		#[repr(C)]
		#[derive(Default)]
		struct Msg {
			tid: u32,
			call: u32,
			args: [u64; 6],
		}
		unsafe impl mini_std::Pod for Msg {}
		#[repr(C)]
		#[derive(Default)]
		struct Resp {
			tid: u32,
			call: u32,
			rv: u64,
		}
		unsafe impl mini_std::Pod for Resp {}
		#[derive(Default, Copy, Clone)]
		struct ThreadResp {
			call: u32,
			rv: u64,
		}

		let this_tid = 0;
		mini_std::tcp_send(&RUSTOS_NATIVE_SOCKET, Msg {
			tid: this_tid,
			call: id,
			args: {
				let mut it = opts.iter().map(|&v| v as u64);
				[
					it.next().unwrap_or(!0),
					it.next().unwrap_or(!0),
					it.next().unwrap_or(!0),
					it.next().unwrap_or(!0),
					it.next().unwrap_or(!0),
					it.next().unwrap_or(!0),
					]}
			}).unwrap();
		loop
		{
			static THREAD_INFO: mini_std::Mutex<[ThreadResp; MAX_THREADS]> = mini_std::Mutex::new([ ThreadResp { call: CORE_EXITPROCESS, rv: 0 }; MAX_THREADS ]);
			// Lock thread pool
			let mut lh = THREAD_INFO.lock();
			// Check if this request has been serviced (the current TID is in the list)
			// - Uses `CORE_EXITPROCESS` as a sentinel (as that's not ever sent)
			if lh[this_tid as usize].call != CORE_EXITPROCESS {
				let resp = &mut lh[this_tid as usize];
				assert!(resp.call == id);
				resp.call = 0;
				return resp.rv;
			}
			// Read a response
			let resp: Resp = mini_std::tcp_recv(&RUSTOS_NATIVE_SOCKET).unwrap();
			if resp.tid == this_tid {
				assert!(resp.call == id);
				return resp.rv;
			}
			// Place in the thread's entry in the wait queue
			todo!("Multi-threaded syscalls");
		}
		},
	}
}


mod mini_std {
	pub unsafe trait Pod: Default {}
	unsafe impl Pod for u32 {}

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
		let rv = libc::mmap(
			addr,
			page_count * 0x1000,
			libc::PROT_READ | libc::PROT_WRITE,
			libc::MAP_ANONYMOUS | libc::MAP_PRIVATE | libc::MAP_FIXED | libc::MAP_FIXED_NOREPLACE,
			/*fd=*/0,
			/*offset=*/0
			);
		if rv == libc::MAP_FAILED {
			Err(mini_std::Errno::get())
		}
		else if rv != addr {
			todo!("MEM_ALLOCATE({:p}, {}p): failed {:p}", addr, page_count, rv);
		}
		else {
			Ok( () )
		}		
	}
	#[cfg(windows)]
	pub fn mmap_alloc(addr: *mut u8, page_count: usize) -> Result<(),Errno> {
		let mut si: ::winapi::um::sysinfoapi::SYSTEM_INFO;
		unsafe {
			si = ::core::mem::zeroed();
			::winapi::um::sysinfoapi::GetSystemInfo(&mut si);
			println!("si.dwPageSize = {:#x}", si.dwPageSize);
			println!("si.dwAllocationGranularity = {:#x}", si.dwAllocationGranularity);
		}
		fn dump_info(addr: *mut u8) {
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
				let rv = unsafe { ::winapi::um::memoryapi::VirtualAlloc(
					resv_base as *mut _,
					si.dwAllocationGranularity as usize,
					MEM_RESERVE,
					PAGE_READWRITE
					) };
				if rv == ::core::ptr::null_mut() {
					let e = unsafe { ::winapi::um::errhandlingapi::GetLastError() };
					println!("MEM_RESERVE: error {:#x}", e);
				}

			}
		}

		for p in 0 .. page_count
		{
			let rv = unsafe { ::winapi::um::memoryapi::VirtualAlloc(
				(addr as usize + p * 0x1000) as *mut _,
				0x1000,
				MEM_COMMIT /*| MEM_RESERVE*/,
				PAGE_READWRITE
				) };
			if rv == ::core::ptr::null_mut() {
				let e = unsafe { ::winapi::um::errhandlingapi::GetLastError() };
				dump_info((addr as usize - 0x1000) as *mut _);
				dump_info(addr);
				dump_info((addr as usize + 0x1000) as *mut _);
				todo!("mmap_alloc: error {:#x}", e);
			}
		}

		/*
		println!("nbytes = {:#x}", page_count * 0x1000);
		let rv = unsafe { ::winapi::um::memoryapi::VirtualAlloc(
			addr as *mut _,
			page_count * 0x1000,
			MEM_COMMIT /*| MEM_RESERVE*/,
			PAGE_READWRITE
			) };
		if rv == ::core::ptr::null_mut() {
			let e = unsafe { ::winapi::um::errhandlingapi::GetLastError() };
			dump_info((addr as usize - 0x1000) as *mut _);
			dump_info(addr);
			dump_info((addr as usize + 0x1000) as *mut _);
			todo!("mmap_alloc: error {:#x}", e);
		}
		else if rv != addr as *mut _ {
			todo!("MEM_ALLOCATE({:p}, {}p): failed {:p}", addr, page_count, rv);
		}
		else {*/
			dump_info(addr);
			Ok( () )
		//}
	}
	
	#[cfg(windows)]
	pub struct Socket(Option<::std::net::TcpStream>);
	impl Socket {
		pub const fn null() -> Socket {
			Socket(None)
		}
	}

	pub fn tcp_connect_localhost(port: u16) -> Result<Socket, &'static str> {
		Ok(Socket(Some(::std::net::TcpStream::connect( ("127.0.0.1", port) ).map_err(|_| "connect failed")?)))
	}
	pub fn tcp_recv<T: Pod>(sock: &Socket) -> Result<T,&'static str> {
		use std::io::Read;
		let mut rv = T::default();
		// SAFE: Correct pointers, data is POD
		let slice = unsafe { ::std::slice::from_raw_parts_mut(&mut rv as *mut _ as *mut u8, ::core::mem::size_of::<T>()) };
		sock.0.as_ref().unwrap().read_exact(slice).map_err(|_| "Error reported")?;
		Ok(rv)
	}
	pub fn tcp_send<T: Pod>(sock: &Socket, val: T) -> Result<(), &'static str> {
		use std::io::Write;
		// SAFE: Correct pointers, and data is POD
		let slice = unsafe { ::std::slice::from_raw_parts(&val as *const _ as *const u8, ::core::mem::size_of::<T>()) };
		sock.0.as_ref().unwrap().write_all(slice).map_err(|_| "Error reported")?;
		Ok( () )
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
}
