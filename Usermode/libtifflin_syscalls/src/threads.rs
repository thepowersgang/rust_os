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


