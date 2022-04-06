
// MOST OSes have a 4K page size
pub const PAGE_SIZE: usize = 0x1000;

#[cfg_attr(not(feature="native_nolink"), link(name="loader_dyn",kind="dylib"))]
#[allow(improper_ctypes)]
extern "C" {
	fn rustos_native_init(port: u16);
	fn rustos_native_panic() -> !;
	fn rustos_native_syscall(id: u32, opts: &[usize]) -> u64;
}
#[cfg(not(windows))]
#[link(name="gcc_s")]
extern "C" {
}

extern crate libc;

pub fn native_init(port: u16) {
	// SAFE: Called once
	unsafe {
		rustos_native_init(port);
	}
}

pub fn trigger_panic() -> ! {
	// SAFE: Safe call
	unsafe {
		rustos_native_panic()
	}
}

unsafe fn syscall(id: u32, opts: &[usize]) -> u64 {
	rustos_native_syscall(id, opts)
}

// SAVE rdi, rsi, rdx, r10, r8, r9
#[inline]
pub unsafe fn syscall_0(id: u32) -> u64 {
	syscall(id, &[])
}
#[inline]
pub unsafe fn syscall_1(id: u32, a1: usize) -> u64 {
	syscall(id, &[a1])
}
#[inline]
pub unsafe fn syscall_2(id: u32, a1: usize, a2: usize) -> u64 {
	syscall(id, &[a1, a2])
}
#[inline]
pub unsafe fn syscall_3(id: u32, a1: usize, a2: usize, a3: usize) -> u64 {
	syscall(id, &[a1, a2, a3])
}
#[inline]
pub unsafe fn syscall_4(id: u32, a1: usize, a2: usize, a3: usize, a4: usize) -> u64 {
	syscall(id, &[a1, a2, a3, a4])
}
#[inline]
pub unsafe fn syscall_5(id: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> u64 {
	syscall(id, &[a1, a2, a3, a4, a5])
}
#[inline]
pub unsafe fn syscall_6(id: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize) -> u64 {
	syscall(id, &[a1, a2, a3, a4, a5, a6])
}

