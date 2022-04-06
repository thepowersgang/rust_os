//! Placeholder library for libloader_dyn (the symbols exported by the loader host)
#![no_std]

#[no_mangle] pub extern "C" fn new_process() { loop{} }
#[no_mangle] pub extern "C" fn start_process() { loop{} }

// libloader_dyn is an actual library in native mode, and exposes the syscalls too
#[cfg(arch="native")]
pub mod _foo {
	#[no_mangle] pub extern "C" fn rustos_native_init() { loop {} }
	#[no_mangle] pub extern "C" fn rustos_native_syscall() { loop {} }
	#[no_mangle] pub extern "C" fn rustos_native_panic() -> ! { loop {} }
}

#[panic_handler]
fn panic_handler(_: &::core::panic::PanicInfo) -> ! {
	loop {}
}

#[cfg(windows)]
#[no_mangle]
pub extern "system" fn _DllMainCRTStartup() {
}
