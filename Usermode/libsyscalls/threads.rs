//! Thread management system calls

use core::prelude::*;

#[inline]
pub unsafe fn start_thread(ip: usize, sp: usize, tlsbase: usize) -> Result<u32, u32> {
	::to_result( syscall!(CORE_STARTTHREAD, ip, sp, tlsbase) as usize )
}
#[inline]
pub fn exit_thread() -> ! {
	unsafe {
		syscall!(CORE_EXITTHREAD);
		::core::intrinsics::unreachable();
	}
}

// Object 0 : This process
/// Current process handle
pub static S_THIS_PROCESS: ThisProcess = ThisProcess;//( ::ObjectHandle(0) );

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
		match super::ObjectHandle::new( unsafe { obj.call_2(::values::CORE_THISPROCESS_RECVOBJ, T::class() as usize, idx) } as usize )
		{
		Ok(v) => Ok(T::from_handle(v)),
		Err(e) => panic!("receive_object error {}", e),
		}
		)
	}
}
impl ::Object for ThisProcess {
	const CLASS: u16 = ::values::CLASS_CORE_THISPROCESS;
	fn class() -> u16 { panic!("Cannot send/recv 'ThisProcess'"); }
	fn from_handle(handle: ::ObjectHandle) -> Self { panic!("ThisProcess::from_handle not needed") }
	fn into_handle(self) -> ::ObjectHandle { panic!("ThisProcess::into_handle not needed") }
	fn get_wait(&self) -> ::values::WaitItem {
		::values::WaitItem {
			object: 0,
			flags: ::values::EV_THISPROCESS_RECVOBJ,
		}
	}
	fn check_wait(&self, _wi: &::values::WaitItem) {
	}

}

pub struct Process(::ObjectHandle);
impl Process {
	#[inline]
	pub fn terminate(&self) {
		unsafe { self.0.call_0(::values::CORE_PROCESS_KILL); }
	}
	#[inline]
	pub fn send_obj<O: ::Object>(&self, obj: O) {
		let oh = obj.into_handle().into_raw();
		unsafe { self.0.call_1(::values::CORE_PROCESS_SENDOBJ, oh as usize); }
	}
	#[inline]
	pub fn send_msg(&self, id: u32, data: &[u8]) {
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
	fn get_wait(&self) -> ::values::WaitItem {
		panic!("TODO - Process::get_wait");
	}
	fn check_wait(&self, wi: &::values::WaitItem) {
		if wi.flags & ::values::EV_PROCESS_TERMINATED != 0 {
		}
	}
}

#[inline]
pub fn start_process(entry: usize, stack: usize,  clone_start: usize, clone_end: usize) -> Result<Process,()> {
	let rv = unsafe { syscall!(CORE_STARTPROCESS, entry, stack, clone_start, clone_end) };
	match ::ObjectHandle::new(rv as usize)
	{
	Ok(v) => Ok( Process(v) ),
	Err(_e) => Err( () ),
	}
}

#[inline]
pub fn exit(code: u32) -> ! {
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
	unsafe {
		syscall!(CORE_WAIT, items.as_ptr() as usize, items.len(), wake_time_mono as usize) as u32
	}
}

