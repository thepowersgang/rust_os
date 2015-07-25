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

#[inline]
pub fn receive_object<T: ::Object>(idx: usize) -> Result<T, ()> {
	match super::ObjectHandle::new( unsafe { syscall!(CORE_RECVOBJ, idx, T::class() as usize) } as usize )
	{
	Ok(v) => Ok(T::from_handle(v)),
	Err(e) => panic!("receive_object error {}", e),
	}
}

pub struct Process(::ObjectHandle);
impl Process {
	pub fn terminate(&self) {
		unsafe { self.0.call_0(::values::CORE_PROCESS_KILL); }
	}
	pub fn send_obj<O: ::Object>(&self, obj: O) {
		let oh = obj.into_handle().into_raw();
		unsafe { self.0.call_1(::values::CORE_PROCESS_SENDOBJ, oh as usize); }
	}
	pub fn send_msg(&self, id: u32, data: &[u8]) {
		unsafe { self.0.call_3(::values::CORE_PROCESS_SENDMSG, id as usize, data.as_ptr() as usize, data.len()); }
	}
}
impl ::Object for Process {
	const CLASS: u16 = ::values::CLASS_PROCESS;
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
pub fn wait(items: &mut [WaitItem], wake_time_mono: u64) -> u32 {
	unsafe {
		syscall!(CORE_WAIT, items.as_ptr() as usize, items.len(), wake_time_mono as usize) as u32
	}
}

