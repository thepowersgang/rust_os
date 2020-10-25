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

static mut RUSTOS_NATIVE_SOCKET: i32 = 3;
const MAX_THREADS: usize = 16;

#[no_mangle]
pub unsafe extern "C" fn rustos_native_init(port: u16)
{
	// SAFE: Called once
	unsafe {
		RUSTOS_NATIVE_SOCKET = mini_std::tcp_connect_localhost(port).unwrap();
		let _v: u32 = mini_std::tcp_recv(RUSTOS_NATIVE_SOCKET).unwrap();
	}
}

// TODO: use this for sending syscalls
#[no_mangle]
#[allow(improper_ctypes)]
pub unsafe extern "C" fn rustos_native_syscall(id: u32, opts: &[usize]) -> u64 {
	use ::syscalls::values::*;
	match id
	{
	// Process-related core functions
	//CORE_LOGWRITE => {
	//	let ptr = ::core::slice::from_raw_parts(opts[0] as *const u8, opts[1]);
	//	println!("LOGWRITE: {}", String::from_utf8_lossy(ptr));
	//	0
	//	},
	//CORE_DBGVALUE => {
	//	let ptr = ::core::slice::from_raw_parts(opts[0] as *const u8, opts[1]);
	//	let val = opts[2];
	//	println!("LOGWRITE: {}: {:#x}", String::from_utf8_lossy(ptr), val);
	//	0
	//	},
	CORE_EXITPROCESS => {
		::std::process::exit(opts[0] as i32)
		},
	// Memory, wrap mmap
	MEM_ALLOCATE => {
		let addr = opts[0] as *mut _;
		let page_count = opts[1];
		let rv = libc::mmap(
			addr,
			page_count * 0x1000,
			libc::PROT_READ | libc::PROT_WRITE,
			libc::MAP_ANONYMOUS | libc::MAP_PRIVATE | libc::MAP_FIXED | libc::MAP_FIXED_NOREPLACE,
			/*fd=*/0,
			/*offset=*/0
			);
		if rv == libc::MAP_FAILED {
			println!("errno={}", mini_std::Errno::get());
			(1 << 32) | 0
		}
		else if rv != addr {
			todo!("MEM_ALLOCATE({:p}, {}p): failed {:p}", addr, page_count, rv);
		}
		else {
			0
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

		//mini_std::write_stdout(b"Sending syscall req\n");
		let this_tid = 0;
		mini_std::tcp_send(RUSTOS_NATIVE_SOCKET, Msg {
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
			let resp: Resp = mini_std::tcp_recv(RUSTOS_NATIVE_SOCKET).unwrap();
			if resp.tid == this_tid {
				assert!(resp.call == id);
				return resp.rv;
			}
			// Place in the thread's entry in the wait queue
			todo!("Multi-threaded syscalls");
		}
		0
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

	//extern crate std;
	mod imp {
		#[link(name="c")]
		extern "C" {
			pub fn write(fd: i32, buf: *const u8, len: usize) -> i32;

			pub fn recv(fd: i32, buf: *mut u8, len: usize, flags: u32) -> i32;
			pub fn send(fd: i32, buf: *const u8, len: usize, flags: u32) -> i32;
			pub fn exit(val: i32) -> !;
		}
	}
	extern crate libc;

	pub fn tcp_connect_localhost(port: u16) -> Result<i32, &'static str> {
		// SAFE: Valid libc calls
		unsafe {	
			let sock = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
			if sock < 0 {
				return Err("socket failed");
			}
			
			let addr = libc::sockaddr_in {
				sin_family: libc::AF_INET as u16,
				sin_port: port.swap_bytes(),
				sin_addr: libc::in_addr { s_addr: 0x7F_00_00_01u32.swap_bytes(), },
				sin_zero: [0; 8],
				};
			let rv = libc::connect(sock, &addr as *const _ as *const _, ::core::mem::size_of::<libc::sockaddr>() as u32);
			if rv < 0 {
				return Err("connect failed");
			}

			Ok(sock)
		}
	}
	pub fn tcp_recv<T: Pod>(sock: i32) -> Result<T,&'static str> {
		let mut rv = T::default();
		// SAFE: Correct pointers, an invalid socket will error, and data is POD
		let resp = unsafe { imp::recv(sock, &mut rv as *mut _ as *mut u8, ::core::mem::size_of::<T>(), 0) };
		if resp < 0 {
			Err("Error reported")
		}
		else if resp == 0 {
			Err("EOF")
		}
		else if resp as usize != ::core::mem::size_of::<T>() {
			Err("Incomplete")
		}
		else {
			Ok(rv)
		}
	}
	pub fn tcp_send<T: Pod>(sock: i32, val: T) -> Result<(), &'static str> {
		// SAFE: Correct pointers, an invalid socket will error, and data is POD
		let resp = unsafe { imp::send(sock, &val as *const _ as *const u8, ::core::mem::size_of::<T>(), 0) };
		if resp < 0 {
			Err("Error reported")
		}
		else if resp as usize != ::core::mem::size_of::<T>() {
			Err("Incomplete")
		}
		else {
			Ok( () )
		}
	}

	pub fn exit(val: i32) -> ! {
		// SAFE: Diverging
		unsafe { imp::exit(val) }
	}
	pub fn write_stdout(s: &[u8]) {
		// SAFE: Valid pointer
		unsafe { imp::write(1, s.as_ptr(), s.len()); }
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
