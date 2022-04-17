#![crate_type="cdylib"]
#![feature(rustc_private)]	// libc

extern crate libc;

mod mini_std;

include!("../../common.inc.rs");

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
	Ok(h) => {
		h.send_obj( "ro:/", ::syscalls::vfs::root().clone() );
		Ok(h)
		},
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
	RUSTOS_NATIVE_SOCKET = mini_std::Socket::connect_localhost(port).unwrap();
	let pid: u32 = RUSTOS_NATIVE_SOCKET.recv().unwrap();
	RUSTOS_PID = pid;
}

#[no_mangle]
pub extern "C" fn rustos_native_panic() -> !
{
	panic!("NativeKernel user panic")
}

fn get_pid() -> u32 {
	// SAFE: Written once
	unsafe { RUSTOS_PID }
}
fn log(args: ::core::fmt::Arguments) {
	let mut buf = [0; 128];
	let mut c = ::std::io::Cursor::new(&mut buf[..]);
	let _ = ::std::io::Write::write_fmt(&mut c, args);
	// SAFE: Valid syscall arguments
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
	// User logging messages, avoids the mess from IPC for each log message
	CORE_LOGWRITE => {
		let addr = opts[0];
		let size = opts[1];
		let msg = std::slice::from_raw_parts(addr as *const u8, size);
		let msg = std::str::from_utf8(msg).unwrap_or("BADTEXT");
		println!("USER{}> {}", get_pid(), msg);
		0
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
		RUSTOS_NATIVE_SOCKET.send(Msg {
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
			let resp: Resp = RUSTOS_NATIVE_SOCKET.recv().unwrap();
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

