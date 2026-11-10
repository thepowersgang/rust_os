use crate::values as v;

#[inline]
/// Write a string to the kernel's log
pub fn log_write<S: ?Sized+AsRef<[u8]>>(msg: &S) {
	let msg = msg.as_ref();
	// SAFE: Syscall
	unsafe { crate::syscall(v::CORE_LOGWRITE { msg }); }
}
pub fn debug_value<S: ?Sized+AsRef<[u8]>>(msg: &S, v: usize) {
	let msg = msg.as_ref();
	// SAFE: Syscall
	unsafe { crate::syscall(v::CORE_DBGVALUE { msg, value: v }); }
}

/// Get the current system tick count
#[inline]
pub fn system_ticks() -> u64 {
	// SAFE: No arguments to call
	unsafe { crate::syscall(v::CORE_SYSTEM_TICKS {}) }
}

#[inline]
/// Obtain a string from the kernel
/// 
/// Accepts a buffer and returns a string slice from that buffer.
pub fn get_text_info(unit: v::TextInfo, id: u32, buf: &mut [u8]) -> &str {
	// SAFE: Syscall
	let len: usize = unsafe { crate::syscall(v::CORE_TEXTINFO { group: unit as u32, value: id, dst: buf }) } as usize;
	::core::str::from_utf8(&buf[..len]).expect("TODO: get_text_info handle error")
}