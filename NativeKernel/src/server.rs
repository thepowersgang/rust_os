//! The complex logic used to manage communication with client processes

use ::kernel::arch::imp::threads::test_pause_thread;

mod start_process;

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
/// A process that is still being set up
struct PreStartProcess {
	mutex: ::std::sync::Mutex<PreStartState>,
	condvar: ::std::sync::Condvar,
}

/// Global state for the syscall interface
pub struct GlobalState
{
	/// OS handles for each process, removed when the child exits
	processes: ::std::collections::HashMap<::kernel::threads::ProcessID, ::std::process::Child>,
	/// Internal handles for each running process (pushed when the native process connects)
	/// - Kept in this list so the root thread can be started
	process_handles: ::std::collections::HashMap<::kernel::threads::ProcessID, ::kernel::threads::ProcessHandle>,
	/// Internal information for processes that haven't yet fully spawned (removed when the worker is started)
	pre_start_processes: ::std::collections::HashMap<::kernel::threads::ProcessID, ::std::sync::Arc<PreStartProcess>>,
	/// The current process that is still-to-be claimed (i.e. the most recently spawned native process)
	/// `Some(None)` is used for PID0 (which doesn't have a handle)
	to_be_claimed: Option< Option<::kernel::threads::ProcessHandle> >,
}
pub type GlobalStateRef = ::std::sync::Arc<::std::sync::Mutex<GlobalState>>;
impl GlobalState
{
	pub fn new(proc_0: ::std::process::Child) -> GlobalState
	{
		GlobalState {
			processes: ::std::iter::once( (0, proc_0) ).collect(),
			process_handles: Default::default(),
			pre_start_processes: Default::default(),
			to_be_claimed: Some(None),
		}
	}

	pub fn check_for_terminated(&mut self)
	{
		for (id, proc) in &mut self.processes {
			match proc.try_wait()
			{
			Ok(None) => {},
			Ok(Some(status)) => {
				log_error!("Process #{} exited: {}", id, status);
				todo!("Handle process exit (inform threading module)");
				},
			Err(e) => {
				panic!("Failed to poll status of process #{}: {:?}", id, e);
				},
			}
		}
	}

	/// Called when a process is created
	fn add_process(&mut self, name: &str, proc: ::std::process::Child) -> ::kernel::threads::ProcessID
	{
		let handle = ::kernel::threads::ProcessHandle::new(name, 0,0);
		let pid = handle.get_pid();
		self.processes.insert(pid, proc);
		assert!(self.to_be_claimed.is_none());
		self.to_be_claimed = Some( Some(handle) );
		self.pre_start_processes.insert(pid, Default::default());
		pid
	}
	/// Called when a new client connects
	fn claim_process(&mut self) -> Option<::kernel::threads::ProcessHandle> {
		self.to_be_claimed.take().expect("Nothing to be claimed?")
	}

	/// Called once the kernel worker for a process is started
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
			match child.kill()
			{
			Ok(_) => {},
			Err(e) => log_warning!("Failed to kill child {:?}: {:?}", child, e),
			}
		}
	}
}

pub fn main_loop(server: ::std::net::TcpListener, gs_root: GlobalStateRef)
{
	// Main thread: Wait for incoming connections
	loop
	{
		let (sock, addr) = match test_pause_thread(|| server.accept())
			{
			Ok(v) => v,
			Err(e) => panic!("accept() failed: {}", e),
			};
		sock.set_nodelay(true).expect("failed to set TCP_NODELAY");	// Used to ensure that syscall latency is low
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
			::std::mem::forget( ::kernel::threads::WorkerThread::new("PID0 Rx Worker", move || { process_worker(gs, pid, sock, addr); }) );
		}
	}
}

/// Code for per-process worker threads
fn process_worker(
	gs: GlobalStateRef,
	pid: ::kernel::threads::ProcessID,
	mut sock: ::std::net::TcpStream,
	addr: ::std::net::SocketAddr
) -> u32
{
	use ::std::io::{Read,Write};

	// Wait until the process is cleared to start running
	// - Only applies to PIDs other than #0
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
	// Inform the client of its simulated PID (releasing it to start running)
	let buf = pid.to_le_bytes();
	match sock.write_all(&buf)
	{
	Ok(_) => {},
	Err(e) => {
		panic!("Unable to initialise process: {:?}", e);
		},
	}
	
	let pauser = ::kernel::arch::imp::threads::ThreadPauser::new();
	let mut _pause_handle = pauser.pause();
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
		match {
			let v = sock.read_exact( unsafe { ::std::slice::from_raw_parts_mut(&mut req as *mut _ as *mut u8, ::std::mem::size_of::<Msg>()) });
			drop(_pause_handle);
			v
			}
		{
		Ok(_) => {},
		Err(e) => {
			log_error!("Failed to read syscall request from {:?}: {:?}", addr, e);
			break
			},
		}
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
			// Locally handle the `CORE_STARTPROCESS` call
			::syscalls::native_exports::values::CORE_STARTPROCESS => {
				match start_process::handle_syscall(&gs, &args_usize[..])
				{
				Ok( rv ) => rv as u64,
				Err(_) => error_code(0),
				}
				},
			// Defer everything else to the syscall module
			_ => unsafe { ::syscalls::syscalls_handler(req.call, args_usize.as_ptr(), args_usize.len() as u32) },
			};
		let res = THREAD_CURRENT_INFO.with(|f| {
			f.borrow_mut().take().unwrap().writeback()
		});
		if res.is_err() {
			// TODO: Change the response
			todo!("Error in writing back syscall results");
		}

		let res = Resp {
			tid: req.tid,
			call: req.call,
			rv: res_val,
			};
		_pause_handle = pauser.pause();
		match /*test_pause_thread(|| */sock.write(unsafe { ::std::slice::from_raw_parts(&res as *const _ as *const u8, ::std::mem::size_of::<Resp>()) })/*)*/
		{
		Ok(_) => {},
		Err(e) => { log_error!("Failed to send syscall response: {:?}", e); break },
		}
	}

	
	_pause_handle = pauser.pause();

	// Drop the socket so the child will definitely quit
	drop(sock);
	// Clean up the process (wait for the child to terminate)
	match gs.lock().unwrap().processes.get_mut(&pid).map(|v| v.wait())
	{
	None => panic!("PID #{} not in the process list", pid),
	Some(Ok(status)) => {
		if status.success() {
			0
		}
		else if let Some(s) = status.code() {
			s as u32
		}
		else {
			u32::MAX
		}
	},
	Some(Err(e)) => panic!("Failed to wait for PID #{}: {:?}", pid, e),
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

	/// Register a child pointer for reading/writing
	fn register(&mut self, ptr: *const u8, len: usize, is_mut: bool) -> *const u8
	{
		use ::process_memory::CopyAddress;
		let mut buf = ::std::vec![0u64; (len + 8-1) / 8];
		// SAFE: Just reinterpeting `u64` as `u8`
		let buf_u8 = unsafe { ::std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut u8, len) };
		// Use debug hooks to read from the processes' address space
		match self.process_handle.copy_address(ptr as usize, buf_u8)
		{
		Ok( () ) => {},
		Err(_) => return ::std::ptr::null(),
		}
		//log_debug!("Read from {:p}+{}: {:?} ({:p})", ptr, len, ::kernel::lib::byte_str::ByteStr::new(buf_u8), buf_u8.as_ptr());
		let rv = buf.as_ptr() as *const u8;
		// Save the buffer, and include the writeback flag
		self.buffers.push(SyscallBuffer {
			buffer: buf,
			writeback: is_mut,
			writeback_addr: (ptr as usize, len),
			});
		rv
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

/// Called by `Kernel/Modules/syscalls/args.rs`'s `Freeze` handling
/// 
/// Copies memory from the child process, and add it to a writeback list
#[no_mangle]
pub extern "Rust" fn native_map_syscall_pointer(ptr: *const u8, len: usize, is_mut: bool) -> *const u8
{
	THREAD_CURRENT_INFO.with(|info| {
		let mut info = info.borrow_mut();
		let info = info.as_mut().expect("native_map_syscall_pointer called with no current userspace thread");
		info.register(ptr, len, is_mut)
	})
}