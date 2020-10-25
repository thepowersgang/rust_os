

static mut RUSTOS_NATIVE_SOCKET: i32 = 3;
const MAX_THREADS: usize = 16;

pub fn native_init(port: u16) {
	// SAFE: Called once
	unsafe {
		RUSTOS_NATIVE_SOCKET = mini_std::tcp_connect_localhost(port).unwrap();
		let _v: u32 = mini_std::tcp_recv(RUSTOS_NATIVE_SOCKET).unwrap();
	}
}

unsafe fn syscall(id: u32, opts: &[usize]) -> u64 {
	use crate::values::*;
	match id
	{
	// Process-related core functions
	CORE_LOGWRITE => {
		let ptr = ::core::slice::from_raw_parts(opts[0] as *const u8, opts[1]);
		mini_std::write_stdout(b"LOGWRITE: ");
		mini_std::write_stdout(ptr);
		mini_std::write_stdout(b"\n");
		0
		},
	/*
	CORE_DBGVALUE => {
		let ptr = ::std::str::from_raw_parts(opts[0] as *const u8, opts[1]);
		let val = opts[2];
		println!("{}: {:#x}", ptr, val);
		0
		},
	*/
	CORE_EXITPROCESS => {
		mini_std::exit(opts[0] as i32)
		},
	// Memory, wrap mmap
	MEM_ALLOCATE => todo!("MEM_ALLOCATE"),
	MEM_REPROTECT => todo!("MEM_REPROTECT"),
	MEM_DEALLOCATE => todo!("MEM_DEALLOCATE"),
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
		},
	}
}

// SAVE rdi, rsi, rdx, r10, r8, r9
#[inline]
pub unsafe fn syscall_0(id: u32) -> u64 {
	syscall(id, &[])
}
#[inline]
pub unsafe fn syscall_1(id: u32, a1: usize) -> u64 {
	syscall(id, &[a1])
}
#[inline]
pub unsafe fn syscall_2(id: u32, a1: usize, a2: usize) -> u64 {
	syscall(id, &[a1, a2])
}
#[inline]
pub unsafe fn syscall_3(id: u32, a1: usize, a2: usize, a3: usize) -> u64 {
	syscall(id, &[a1, a2, a3])
}
#[inline]
pub unsafe fn syscall_4(id: u32, a1: usize, a2: usize, a3: usize, a4: usize) -> u64 {
	syscall(id, &[a1, a2, a3, a4])
}
#[inline]
pub unsafe fn syscall_5(id: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> u64 {
	syscall(id, &[a1, a2, a3, a4, a5])
}
#[inline]
pub unsafe fn syscall_6(id: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize) -> u64 {
	syscall(id, &[a1, a2, a3, a4, a5, a6])
}

//#[link(name="native_wrapper")]
//extern "C"
//{
//}

mod mini_std {
	pub unsafe trait Pod: Default {}
	unsafe impl Pod for u32 {}

	//extern crate std;
	mod imp {
		#[link(name="gcc_s")]
		extern "C" {
		}
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