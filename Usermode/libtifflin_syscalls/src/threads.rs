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

pub struct Process;
#[inline]
pub fn start_process(entry: usize, stack: usize,  clone_start: usize, clone_end: usize) -> Result<Process,()> {
	let rv = unsafe { syscall!(CORE_STARTPROCESS, entry, stack, clone_start, clone_end) };
	match ::to_result(rv as usize)
	{
	Ok(_v) => Ok( Process ),
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
pub fn wait(items: &[WaitItem], wake_time_mono: u64) -> u32 {
	unsafe {
		syscall!(CORE_WAIT, items.as_ptr() as usize, items.len(), wake_time_mono as usize) as u32
	}
}

