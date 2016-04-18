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
	pub fn receive_object<T: ::Object>(&self) -> Result<T, ()> {
		self.with_obj(|obj| 
			// SAFE: Syscall
			match super::ObjectHandle::new( unsafe { obj.call_1(::values::CORE_THISPROCESS_RECVOBJ, T::class() as usize) } as usize )
			{
			Ok(v) => Ok(T::from_handle(v)),
			Err(0) => Err( () ),
			Err(e) => panic!("receive_object error {}", e),
			}
		)
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


#[inline]
pub fn start_process(name: &str,  clone_start: usize, clone_end: usize) -> Result<ProtoProcess,()> {
	// SAFE: Syscall
	let rv = unsafe { syscall!(CORE_STARTPROCESS, name.as_ptr() as usize, name.len(),  clone_start, clone_end) };
	match ::ObjectHandle::new(rv as usize)
	{
	Ok(v) => Ok( ProtoProcess(v) ),
	Err(_e) => Err( () ),
	}
}

pub struct ProtoProcess(::ObjectHandle);
impl ::Object for ProtoProcess {
	const CLASS: u16 = ::values::CLASS_CORE_PROTOPROCESS;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		ProtoProcess(handle)
	}
	fn into_handle(self) -> ::ObjectHandle {
		self.0
	}
	fn handle(&self) -> &::ObjectHandle {
		&self.0
	}

	type Waits = ();
}
impl ProtoProcess
{
	#[inline]
	pub fn send_obj<O: ::Object>(&self, obj: O) {
		let oh = obj.into_handle().into_raw();
		// SAFE: Syscall
		unsafe { self.0.call_1(::values::CORE_PROTOPROCESS_SENDOBJ, oh as usize); }
	}
 
 	#[inline]
	pub fn start(self, entry: usize, stack: usize) -> Process {
		// SAFE: Syscall
		let rv = unsafe { self.0.call_2_v(::values::CORE_PROTOPROCESS_START, entry, stack) };
		Process( ::ObjectHandle::new(rv as usize).expect("Error erturned from CORE_PROTOPROCESS_START - unexpected") )
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

