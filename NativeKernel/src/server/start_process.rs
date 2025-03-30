//! Handling of spawning new processed

use ::kernel::arch::imp::threads::test_pause_thread;
use super::{GlobalStateRef, PreStartProcess,PreStartState};

/// Handle the `CORE_STARTPROCESS` syscall with some special logic
pub fn handle_syscall(gs: &super::GlobalStateRef, mut args: &[usize]) -> Result<u32, ::syscalls::Error>
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
	// - Just use the process name for now
	let proc_name = std::str::from_utf8(&proc_name).map_err(|_| ::syscalls::Error::BadValue)?;
	if !proc_name.starts_with("/sysroot/") {
		return Err(::syscalls::Error::BadValue);
	}
	let path = &proc_name[8..];
	let path = ["../Usermode/.output/native", path].concat();
	#[cfg(windows)]
	let path = path + ".exe";

	fn byte_slice_to_osstr(v: &[u8])->&::std::ffi::OsStr {
		// Windows: Check that the string data is valid WTF-8
		#[cfg(windows)]
		let rv = match std::str::from_utf8(v)
			{
			Ok(v) => v.as_ref(),
			// TODO: Find a better way to encode on windows that handles arbitrary bytes
			Err(e) => todo!("Handle malformed UTF-8 in arguments: {:?}", e),
			};
		// UNIX: All bytes are valid
		#[cfg(unix)]
		let rv = std::os::unix::ffi::OsStrExt::from_bytes(v);
		rv
	}

	let mut lh = gs.lock().unwrap();	// Lock before spawning, so when the worker calls `claim_process` it waits for this to push
	let mut newproc = match ::std::process::Command::new(&path)
			.args(proc_args.split(|&v| v == 0)
				.map(|v| byte_slice_to_osstr(v))
				)
			.spawn()
		{
		Ok(c) => c,
		Err(e) => {
			log_error!("Unable to spawn child: {:?}", e);
			return Ok((1 << 31) | 0);
			}
		};
	// Sleep for a short period, giving the application time to start and connect to the server
	::std::thread::sleep(::std::time::Duration::from_millis(100));
	// If the process quits within the first 100ms, exit.
	if let Some(status) = newproc.try_wait().expect("Pre-start try_wait") {
		log_error!("Spawning of {:?} failed: exited {}", path, status);
		return Err(::syscalls::Error::BadValue);
	}
	return Ok( ::syscalls::native_exports::new_object(ProtoProcess {
		pid: lh.add_process(proc_name, newproc),
		gs: gs.clone(),
		}) );

	struct ProtoProcess
	{
		gs: GlobalStateRef,
		pid: ::kernel::threads::ProcessID,
	}
	impl ::std::ops::Drop for ProtoProcess
	{
		fn drop(&mut self) {
			// TODO: Mark the process handle as needing to be released
			let _psp = self.wait_until_tracked();
			self.gs.lock().unwrap().process_handles.remove(&self.pid);
			//*psp.mutex.lock().unwrap() = PreStartState::Dropped;
			todo!("ProtoProcess dropped, need to tell the worker to close");
		}
	}
	impl ProtoProcess
	{
		fn wait_until_tracked(&self) -> ::std::sync::Arc<PreStartProcess>
		{
			// Wait until the process is started (and thus in the handles pool)
			let e = self.gs.lock().unwrap().pre_start_processes.get(&self.pid).expect("Process not in start list?!").clone();

			{
				let mut lh = test_pause_thread(|| e.mutex.lock().unwrap());
				while *lh < PreStartState::WaitingTracked
				{
					log_debug!("Process state {:?}, waiting for at least WaitingTracked", *lh);
					lh = test_pause_thread(|| e.condvar.wait(lh).expect("Pre-start wait failed"));
				}
			}

			e
		}
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
				self.wait_until_tracked();
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
				let e = self.wait_until_tracked();
				let mut lh = e.mutex.lock().unwrap();
				*lh = PreStartState::Running;
				e.condvar.notify_all();

				Ok( ::syscalls::native_exports::new_object(Process {
					handle: self.gs.lock().unwrap().process_handles.remove(&self.pid).expect("Process handle not in list?"),
					// SAFE: Caller will forget `self`
					_gs: unsafe { ::core::ptr::read(&self.gs) },
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
		_gs: GlobalStateRef,
		handle: ::kernel::threads::ProcessHandle,
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
		fn handle_syscall_ref(&self, call: u16, _args: &mut ::syscalls::native_exports::Args) -> Result<u64,::syscalls::Error> {
			match call
			{
			::syscalls::native_exports::values::CORE_PROCESS_KILL => todo!("CORE_PROCESS_KILL"),
			_ => ::syscalls::native_exports::object_has_no_such_method_ref("Process", call),
			}
		}

		/// Return: Number of wakeup events bound
		fn bind_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
			let mut ret = 0;
			// Wait for child process to terminate
			if flags & ::syscalls::native_exports::values::EV_PROCESS_TERMINATED != 0 {
				self.handle.bind_wait_terminate(obj);
				ret += 1;
			}
			ret
		}
		/// Return: Number of wakeup events fired
		fn clear_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
			let mut ret = 0;
			// Wait for child process to terminate
			if flags & ::syscalls::native_exports::values::EV_PROCESS_TERMINATED != 0 {
				if self.handle.clear_wait_terminate(obj) {
					ret |= ::syscalls::native_exports::values::EV_PROCESS_TERMINATED;
				}
			}
			ret
		}
	}
}