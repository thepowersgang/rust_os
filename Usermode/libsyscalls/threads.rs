//
//
//
//! Thread management system calls

#[inline]
pub unsafe fn start_thread(ip: usize, sp: usize, tlsbase: usize) -> Result<u32, u32> {
	::to_result( syscall!(CORE_STARTTHREAD, ip, sp, tlsbase) as usize )
}
#[inline]
pub fn exit_thread() -> ! {
	// SAFE: Syscall
	unsafe {
		syscall!(CORE_EXITTHREAD);
		::core::intrinsics::unreachable();
	}
}

// Object 0 : This process
/// Current process handle
pub static S_THIS_PROCESS: ThisProcess = ThisProcess;//( ::ObjectHandle(0) );

define_waits!{ ThisProcessWaits => (
	recv_obj:has_recv_obj = ::values::EV_THISPROCESS_RECVOBJ,
	recv_msg:has_recv_msg = ::values::EV_THISPROCESS_RECVMSG,
)}

/// 
pub struct ThisProcess;//(::ObjectHandle);
impl ThisProcess
{
	fn with_obj<T, F: FnOnce(&::ObjectHandle)->T>(&self, fcn: F) -> T {
		let o = ::ObjectHandle(0);
		let r = fcn(&o);
		::core::mem::forget(o);
		r
	}
	#[inline]
	/// Obtain the 'n'th unclaimed object of the specifed type
	pub fn receive_object<T: ::Object>(&self, idx: usize) -> Result<T, ()> {
		self.with_obj(|obj| 
			// SAFE: Syscall
			match super::ObjectHandle::new( unsafe { obj.call_2(::values::CORE_THISPROCESS_RECVOBJ, T::class() as usize, idx) } as usize )
			{
			Ok(v) => Ok(T::from_handle(v)),
			Err(e) => panic!("receive_object error {}", e),
			}
		)
	}

	#[inline]
	pub fn recv_msg(&self, data: &mut [u8]) -> Option<(usize, u32)> {
		// SAFE: Syscall
		let rv = unsafe { self.with_obj(|obj| obj.call_2(::values::CORE_THISPROCESS_RECVMSG, data.as_ptr() as usize, data.len())) };
		if rv == 0 {
			None
		}
		else {
			let (id, size) = ((rv >> 32) as u32, (rv & 0xFFFFFFFF) as u32);
			Some( (size as usize, id) )
		}
	}
}
impl ::Object for ThisProcess {
	const CLASS: u16 = ::values::CLASS_CORE_THISPROCESS;
	fn class() -> u16 { panic!("Cannot send/recv 'ThisProcess'"); }
	fn from_handle(_handle: ::ObjectHandle) -> Self { panic!("ThisProcess::from_handle not needed") }
	fn into_handle(self) -> ::ObjectHandle { panic!("ThisProcess::into_handle not needed") }
	fn handle(&self) -> &::ObjectHandle { panic!("ThisProcess::handle not needed") }

	type Waits = ThisProcessWaits;
	fn get_wait(&self, waits: ThisProcessWaits) -> ::values::WaitItem {
		::values::WaitItem { object: 0, flags: waits.0 }
	}
	fn check_wait(&self, wi: &::values::WaitItem) -> ThisProcessWaits {
		ThisProcessWaits(wi.flags)
	}
}

define_waits!{ ProcessWaits => (
	terminate:get_terminate = ::values::EV_PROCESS_TERMINATED,
)}
pub struct Process(::ObjectHandle);
impl Process {
	#[inline]
	pub fn terminate(&self) {
		// SAFE: Syscall
		unsafe { self.0.call_0(::values::CORE_PROCESS_KILL); }
	}
	#[inline]
	pub fn send_obj<O: ::Object>(&self, obj: O) {
		let oh = obj.into_handle().into_raw();
		// SAFE: Syscall
		unsafe { self.0.call_1(::values::CORE_PROCESS_SENDOBJ, oh as usize); }
	}
	#[inline]
	pub fn send_msg(&self, id: u32, data: &[u8]) {
		// SAFE: Syscall
		unsafe { self.0.call_3(::values::CORE_PROCESS_SENDMSG, id as usize, data.as_ptr() as usize, data.len()); }
	}

	#[inline]
	pub fn wait_terminate(&self) -> ::values::WaitItem {
		self.0.get_wait(::values::EV_PROCESS_TERMINATED)
	}
}
impl ::Object for Process {
	const CLASS: u16 = ::values::CLASS_CORE_PROCESS;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		Process(handle)
	}
	fn into_handle(self) -> ::ObjectHandle { self.0 }
	fn handle(&self) -> &::ObjectHandle { &self.0 }
	
	type Waits = ProcessWaits;
}

#[inline]
pub fn start_process(name: &str, entry: usize, stack: usize,  clone_start: usize, clone_end: usize) -> Result<Process,()> {
	// SAFE: Syscall
	let rv = unsafe { syscall!(CORE_STARTPROCESS, name.as_ptr() as usize, name.len(), entry, stack, clone_start, clone_end) };
	match ::ObjectHandle::new(rv as usize)
	{
	Ok(v) => Ok( Process(v) ),
	Err(_e) => Err( () ),
	}
}

#[inline]
pub fn exit(code: u32) -> ! {
	// SAFE: Syscall
	unsafe {
		syscall!(CORE_EXITPROCESS, code as usize);
		::core::intrinsics::unreachable();
	}
}

pub use values::WaitItem;

/// Blocks the current thread on the passed set of objects.
/// 
/// The thread is automatically woken after the passed monotonic timer value is
///  reached. (passing !0 will disable timer wakeup, passing 0 disables blocking)
///
/// Returns the number of events that caused the wakeup (zero for timeout)
#[inline]
pub fn wait(items: &mut [WaitItem], wake_time_mono: u64) -> u32 {
	// SAFE: Syscall
	unsafe {
		syscall!(CORE_WAIT, items.as_ptr() as usize, items.len(), wake_time_mono as usize) as u32
	}
}

