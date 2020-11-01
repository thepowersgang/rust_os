//
//
//
use ::std::io::Read;
use ::std::io::Write;
use ::kernel::arch::imp::threads::test_pause_thread;

extern crate core;
#[macro_use]
extern crate kernel;
extern crate syscalls;

mod fs_shim;
mod video_shim;

struct ThreadSyscallInfo
{
	process_handle: ::process_memory::ProcessHandle,
	buffers: Vec<SyscallBuffer>,
}
struct SyscallBuffer
{
	buffer: Vec<u64>,
	writeback: bool,
	writeback_addr: (usize, usize),
}
thread_local! {
	static THREAD_CURRENT_INFO: ::std::cell::RefCell<Option<ThreadSyscallInfo>> = ::std::cell::RefCell::new(None);
}

#[derive(PartialOrd,PartialEq,Debug)]
enum PreStartState {
	/// Process has spawned, but the worker hasn't started
	Spawned,
	/// Worker started, waiting 
	WaitingAck,
	/// Process is now tracked in `process_handles`
	WaitingTracked,
	/// Process has been started by parent
	Running,
}
impl Default for PreStartState { fn default() -> Self { PreStartState::Spawned } }
#[derive(Default)]
struct PreStartProcess {
	mutex: ::std::sync::Mutex<PreStartState>,
	condvar: ::std::sync::Condvar,
}
struct GlobalState {
	processes: ::std::collections::HashMap<::kernel::threads::ProcessID, ::std::sync::Arc<::std::process::Child>>,
	process_handles: ::std::collections::HashMap<::kernel::threads::ProcessID, ::kernel::threads::ProcessHandle>,
	pre_start_processes: ::std::collections::HashMap<::kernel::threads::ProcessID, ::std::sync::Arc<PreStartProcess>>,
	to_be_claimed: Option< Option<::kernel::threads::ProcessHandle> >,
	//pid0_worker: 
}
impl GlobalState
{
	fn new(proc_0: ::std::process::Child) -> GlobalState
	{
		GlobalState {
			processes: ::std::iter::once( (0, ::std::sync::Arc::new(proc_0)) ).collect(),
			process_handles: Default::default(),
			pre_start_processes: Default::default(),
			to_be_claimed: Some(None),
		}
	}
	fn add_process(&mut self, name: &str, proc: ::std::process::Child) -> ::kernel::threads::ProcessID
	{
		let handle = ::kernel::threads::ProcessHandle::new(name, 0,0);
		let pid = handle.get_pid();
		self.processes.insert(pid, ::std::sync::Arc::new(proc));
		self.to_be_claimed = Some( Some(handle) );
		self.pre_start_processes.insert(pid, Default::default());
		pid
	}
	fn claim_process(&mut self) -> Option<::kernel::threads::ProcessHandle>
	{
		self.to_be_claimed.take().expect("Nothing to be claimed?")
	}

	fn push_process(&mut self, handle: ::kernel::threads::ProcessHandle) {
		let pid = handle.get_pid();
		self.process_handles.insert(pid, handle);
	}
	fn get_process(&self, pid: ::kernel::threads::ProcessID) -> &::std::process::Child {
		&self.processes[&pid]
	}
}
impl ::std::ops::Drop for GlobalState
{
	fn drop(&mut self)
	{
		for (_pid, child) in &mut self.processes {
			match ::std::sync::Arc::get_mut(child).map(|v| v.kill())
			{
			Some(Ok(_)) => {},
			Some(Err(e)) => log_warning!("Failed to kill child {:?}: {:?}", child, e),
			None => log_warning!("Failed to kill child {:?}: In use", child),
			}
		}
	}
}
type GlobalStateRef = ::std::sync::Arc<::std::sync::Mutex<GlobalState>>;

fn main()// -> Result<(), Box<dyn std::error::Error>>
{
	::kernel::threads::init();
	::kernel::memory::phys::init();
	::kernel::memory::page_cache::init();
	
	(::kernel::metadevs::storage::S_MODULE.init)();
	(::kernel::metadevs::video::S_MODULE.init)();
	(::kernel::vfs::S_MODULE.init)();
	// TODO: Add a minifb backed KB/Mouse too
	let console = video_shim::Console::new();
	::core::mem::forget( ::kernel::metadevs::video::add_output(Box::new(console.get_display())) );
	(::gui::S_MODULE.init)();

	::core::mem::forget( ::kernel::vfs::mount::DriverRegistration::new("native", &fs_shim::NativeFsDriver) );

	//(::fs_fat::S_MODULE.init)();
	//(::fs_extN::S_MODULE.init)();

	let sysdisk = "nullw";
	match ::kernel::metadevs::storage::VolumeHandle::open_named(sysdisk)
	{
	Err(e) => {
		panic!("Unable to open /system volume {}: {}", sysdisk, e);
		},
	Ok(vh) => match ::kernel::vfs::mount::mount("/system".as_ref(), vh, "native", &[])
		{
		Ok(_) => {},
		Err(e) => {
			panic!("Unable to mount /system from {}: {:?}", sysdisk, e);
			},
		},
	}
	::kernel::vfs::handle::Dir::open(::kernel::vfs::Path::new("/")).unwrap()
		.symlink("sysroot", ::kernel::vfs::Path::new("/system/Tifflin"))
		.unwrap()
		;

	let server = match ::std::net::TcpListener::bind( ("127.0.0.1", 32245) )
		{
		Ok(v) => v,
		Err(e) => panic!("bind() failed: {}", e),
		};


	let init_fh = ::kernel::vfs::handle::File::open(
			::kernel::vfs::Path::new("/sysroot/bin/init"),
			::kernel::vfs::handle::FileOpenMode::Execute
		)
		.unwrap();
	::syscalls::init(init_fh);

	#[cfg(not(windows))]
	let init_path = ".native_fs/Tifflin/bin/init";
	#[cfg(windows)]
	let init_path = ".native_fs/Tifflin/bin/init.exe";
	let gs_root = ::std::sync::Arc::new(::std::sync::Mutex::new(
			GlobalState::new( ::std::process::Command::new(init_path).spawn().expect("Failed to spawn init") )
		));

	loop
	{
		let (sock, addr) = match test_pause_thread(|| server.accept())
			{
			Ok(v) => v,
			Err(e) => panic!("accept() failed: {}", e),
			};
		log_debug!("Client connection from {:?}", addr);
		let gs = gs_root.clone();
		// NOTE: This lock must be released before the thread is started
		let h = gs_root.lock().unwrap().claim_process();
		if let Some(mut h) = h
		{
			let pid = h.get_pid();
			log_debug!("{:?} = PID {}", addr, pid);
			// NOTE: Can't have the lock held, as `start_root_thread` attempts to switch to the new thread
			h.start_root_thread(move || process_worker(gs, pid, sock, addr));

			let psp = gs_root.lock().unwrap().pre_start_processes[&pid].clone();

			// Wait until the worker starts
			{
				let mut lh = test_pause_thread(|| psp.mutex.lock().unwrap());
				while *lh < PreStartState::WaitingAck
				{
					lh = test_pause_thread(|| psp.condvar.wait(lh).expect("Process start condvar wait failed"));
				}
			}
			// Push the process to the main list	
			gs_root.lock().unwrap().push_process(h);
			// Let the worker continue (and inform a possibly-waiting sender that it's running)
			{
				let mut lh = test_pause_thread(|| psp.mutex.lock().unwrap());
				*lh = PreStartState::WaitingTracked;
				psp.condvar.notify_all();
			}
		}
		else
		{
			let pid = 0;
			log_debug!("{:?} = PID {}", addr, pid);
			::std::mem::forget( ::kernel::threads::WorkerThread::new("PID0 Rx Worker", move || process_worker(gs, pid, sock, addr)) );
		}
		fn process_worker(gs: GlobalStateRef, pid: ::kernel::threads::ProcessID, mut sock: ::std::net::TcpStream, addr: ::std::net::SocketAddr)
		{
			// Wait until the process should start
			if pid != 0 {
				let e = gs.lock().unwrap().pre_start_processes[&pid].clone();
				let mut lh = e.mutex.lock().unwrap();
				*lh = PreStartState::WaitingAck;
				e.condvar.notify_all();
				// Wait until requested to start
				while *lh < PreStartState::Running
				{
					lh = test_pause_thread(|| e.condvar.wait(lh).expect("Process start condvar wait failed"));
				}
				// Remove from the pre-start list
				gs.lock().unwrap().pre_start_processes.remove(&pid);
			}
			let buf = pid.to_le_bytes();
			match sock.write_all(&buf)
			{
			Ok(_) => {},
			Err(e) => {
				panic!("Unable to initialise process: {:?}", e);
				},
			}
			
			let pauser = ::kernel::arch::imp::threads::ThreadPauser::new();
			let mut pause_handle = pauser.pause();
			loop
			{
				#[repr(C)]
				#[derive(Default,Debug)]
				struct Msg {
					tid: u32,
					call: u32,
					args: [u64; 6],
				}
				#[repr(C)]
				#[derive(Default)]
				struct Resp {
					tid: u32,
					call: u32,
					rv: u64,
				}

				let mut req = Msg::default();
				match /*test_pause_thread(||*/ sock.read_exact( unsafe { ::std::slice::from_raw_parts_mut(&mut req as *mut _ as *mut u8, ::std::mem::size_of::<Msg>()) })/*)*/
				{
				Ok(_) => {},
				Err(e) => {
					log_error!("Failed to read syscall request from {:?}: {:?}", addr, e);
					break
					},
				}
				drop(pause_handle);
				log_log!("PID{}: request: {:x?}", pid, req);

				THREAD_CURRENT_INFO.with(|f| {
					*f.borrow_mut() = Some(ThreadSyscallInfo::new(gs.lock().unwrap().get_process(pid)));
					});
				let args_usize = [
					req.args[0] as usize,
					req.args[1] as usize,
					req.args[2] as usize,
					req.args[3] as usize,
					req.args[4] as usize,
					req.args[5] as usize,
					];

				fn error_code(value: u32) -> u64 {
					value as u64 | (1 << 31)
				}
				let res_val = match req.call
					{
					::syscalls::native_exports::values::CORE_STARTPROCESS => {
						match start_process(&gs, &args_usize[..])
						{
						Ok( rv ) => { rv as u64 },
						Err(_) => {error_code(0)},
						}
						},
					_ => unsafe { ::syscalls::syscalls_handler(req.call, args_usize.as_ptr(), args_usize.len() as u32) },
					};
				let res = THREAD_CURRENT_INFO.with(|f| {
					f.borrow_mut().take().unwrap().writeback()
				});
				if res.is_err() {
					// TODO: Change the response
				}

				let res = Resp {
					tid: req.tid,
					call: req.call,
					rv: res_val,
					};
				pause_handle = pauser.pause();
				match /*test_pause_thread(|| */sock.write(unsafe { ::std::slice::from_raw_parts(&res as *const _ as *const u8, ::std::mem::size_of::<Resp>()) })/*)*/
				{
				Ok(_) => {},
				Err(e) => { log_error!("Failed to send syscall response"); return },
				}
			}
		}
	}
}

fn start_process(gs: &GlobalStateRef, mut args: &[usize]) -> Result<u32, ::syscalls::Error>
{
	use ::kernel::memory::freeze::Freeze;
	use ::syscalls::native_exports::SyscallArg;
	
	let file = ::syscalls::native_exports::get_file_handle(SyscallArg::get_arg(&mut args)?)?;
	let proc_name: Freeze<[u8]> = SyscallArg::get_arg(&mut args)?;
	let proc_args: Freeze<[u8]> = SyscallArg::get_arg(&mut args)?;

	log_notice!("CORE_STARTPROCESS: file={:?} proc_name={:?} proc_args={:?}",
		file,
		::kernel::lib::byte_str::ByteStr::new(&proc_name),
		::kernel::lib::byte_str::ByteStr::new(&proc_args),
		);

	// TODO: Get the real path from the file?
	// - Just use the process name
	let proc_name = std::str::from_utf8(&proc_name).map_err(|_| ::syscalls::Error::BadValue)?;
	if !proc_name.starts_with("/sysroot/") {
		return Err(::syscalls::Error::BadValue);
	}
	let path = &proc_name[8..];
	let path = ["../Usermode/.output/native", path].concat();
	#[cfg(windows)]
	let path = path + ".exe";

	if proc_args.len() > 0 {
		todo!("");
	}

	let newproc = match ::std::process::Command::new(&path).spawn()
		{
		Ok(c) => c,
		Err(e) => {
			log_error!("Unable to spawn child: {:?}", e);
			return Ok((1 << 31) | 0);
			}
		};
	return Ok( ::syscalls::native_exports::new_object(ProtoProcess {
		pid: gs.lock().unwrap().add_process(proc_name, newproc),
		gs: gs.clone(),
		}) );

	struct ProtoProcess
	{
		gs: GlobalStateRef,
		pid: ::kernel::threads::ProcessID,
	}
	impl ::syscalls::native_exports::Object for ProtoProcess
	{
		fn as_any(&self) -> &dyn ::core::any::Any { self }
		fn class(&self) -> u16 { 
			::syscalls::native_exports::values::CLASS_CORE_PROTOPROCESS
		}

		fn try_clone(&self) -> Option<u32> {
			None
		}

		/// Return: Return value or argument error
		fn handle_syscall_ref(&self, call: u16, args: &mut ::syscalls::native_exports::Args) -> Result<u64,::syscalls::Error> {
			match call
			{
			::syscalls::native_exports::values::CORE_PROTOPROCESS_SENDOBJ => {
				let tag: ::syscalls::native_exports::values::FixedStr8 = args.get()?;
				let handle: u32 = args.get()?;
				log_debug!("CORE_PROTOPROCESS_SENDOBJ: tag={:?} handle={}", tag, handle);
				// Wait until the process is started (and thus in the handles pool)
				let psp = self.gs.lock().unwrap().pre_start_processes.get(&self.pid).cloned();
				match psp
				{
				Some(e) => {
					let mut lh = test_pause_thread(|| e.mutex.lock().unwrap());
					while *lh < PreStartState::WaitingTracked
					{
						log_debug!("Process state {:?}, waiting for at least WaitingTracked", *lh);
						lh = test_pause_thread(|| e.condvar.wait(lh).expect("Pre-start wait failed"));
					}
					},
				None => {},
				}
				// TODO: This PID may not be in `process_handles` yet (added when main thread starts)
				let lh = self.gs.lock().unwrap();
				::syscalls::native_exports::give_object(&lh.process_handles[&self.pid], &tag, handle).map(|_| 0)
				},
			_ => ::syscalls::native_exports::object_has_no_such_method_ref("ProtoProcess", call),
			}
		}
		/// NOTE: Implementors should always move out of `self` and drop the contents (the caller will forget)
		/// Return: Return value or argument error
		fn handle_syscall_val(&mut self, call: u16, _args: &mut ::syscalls::native_exports::Args) -> Result<u64,::syscalls::Error> {
			match call
			{
			::syscalls::native_exports::values::CORE_PROTOPROCESS_START => {
				let e = self.gs.lock().unwrap().pre_start_processes.get(&self.pid).expect("Process not in start list?!").clone();
				let mut lh = e.mutex.lock().unwrap();
				while let PreStartState::Spawned = *lh
				{
					lh = test_pause_thread(|| e.condvar.wait(lh).expect("Pre-start wait failed"));
				}
				*lh = PreStartState::Running;
				e.condvar.notify_all();

				Ok( ::syscalls::native_exports::new_object(ProtoProcess {
					pid: self.pid,
					// SAFE: Caller will forget `self`
					gs: unsafe { ::core::ptr::read(&self.gs) },
					}) as u64 )
				},
			_ => ::syscalls::native_exports::object_has_no_such_method_val("ProtoProcess", call),
			}
		}

		/// Return: Number of wakeup events bound
		fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 {
			0
		}
		/// Return: Number of wakeup events fired
		fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 {
			0
		}
	}

	struct Process
	{
		gs: GlobalStateRef,
		pid: ::kernel::threads::ProcessID,
	}
	impl ::syscalls::native_exports::Object for Process
	{
		fn as_any(&self) -> &dyn ::core::any::Any { self }
		fn class(&self) -> u16 { 
			::syscalls::native_exports::values::CLASS_CORE_PROCESS
		}

		fn try_clone(&self) -> Option<u32> {
			None
		}

		/// Return: Return value or argument error
		fn handle_syscall_ref(&self, call: u16, args: &mut ::syscalls::native_exports::Args) -> Result<u64,::syscalls::Error> {
			match call
			{
			_ => ::syscalls::native_exports::object_has_no_such_method_ref("Process", call),
			}
		}

		/// Return: Number of wakeup events bound
		fn bind_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
			let mut ret = 0;
			// Wait for child process to terminate
			if flags & ::syscalls::native_exports::values::EV_PROCESS_TERMINATED != 0 {
				let lh = self.gs.lock().unwrap();
				lh.process_handles[&self.pid].bind_wait_terminate(obj);
				ret += 1;
			}
			ret
		}
		/// Return: Number of wakeup events fired
		fn clear_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
			let mut ret = 0;
			// Wait for child process to terminate
			if flags & ::syscalls::native_exports::values::EV_PROCESS_TERMINATED != 0 {
				let lh = self.gs.lock().unwrap();
				if lh.process_handles[&self.pid].clear_wait_terminate(obj) {
					ret |= ::syscalls::native_exports::values::EV_PROCESS_TERMINATED;
				}
			}
			ret
		}
	}
}

impl ThreadSyscallInfo
{
	fn new(process: &::std::process::Child) -> ThreadSyscallInfo
	{
		use ::process_memory::TryIntoProcessHandle;
		ThreadSyscallInfo {
			process_handle: process.try_into_process_handle().expect("Unable to get process handle from child"),
			buffers: Vec::new(),
		}
	}
	fn writeback(self) -> Result<(),()>
	{
		for b in self.buffers
		{
			if b.writeback
			{
				use ::process_memory::PutAddress;
				// SAFE: Just reintepreting u64 as u8
				let slice = unsafe { ::std::slice::from_raw_parts(b.buffer.as_ptr() as *const u8, b.writeback_addr.1) };
				log_debug!("Writing back data to {:#x}+{}: {:?}",
					b.writeback_addr.0, b.writeback_addr.1,
					::kernel::lib::byte_str::ByteStr::new(slice)
					);
				match self.process_handle.put_address(b.writeback_addr.0, slice)
				{
				Ok(_) => {},
				Err(e) => {
					log_error!("Failed to write-back syscall data to {:#x}+{}: {:?}", b.writeback_addr.0, b.writeback_addr.1, e);
					return Err(());
					},
				}
			}
		}
		Ok(())
	}
}

#[no_mangle]
pub extern "Rust" fn native_map_syscall_pointer(ptr: *const u8, len: usize, is_mut: bool) -> *const u8
{
	use ::process_memory::CopyAddress;
	THREAD_CURRENT_INFO.with(|info| {
		let mut info = info.borrow_mut();
		let info = info.as_mut().unwrap();
		let mut buf = ::std::vec![0u64; (len + 8-1) / 8];
		// Use debug hooks to read from the processes' address space
		let buf_u8 = unsafe { ::std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, len) };
		match info.process_handle.copy_address(ptr as usize, buf_u8)
		{
		Ok( () ) => {},
		Err(_) => return ::std::ptr::null(),
		}
		//log_debug!("Read from {:p}+{}: {:?} ({:p})", ptr, len, ::kernel::lib::byte_str::ByteStr::new(buf_u8), buf_u8.as_ptr());
		// Save for writeback if `is_mut` is true
		let rv = buf.as_ptr() as *const u8;
		info.buffers.push(SyscallBuffer {
			buffer: buf,
			writeback: is_mut,
			writeback_addr: (ptr as usize, len),
			});
		rv
	})
}
