
use ffi::OsStr;

#[lang="start"]
fn start(_main: *const u8, _argc: isize, _argv: *const *const u8) -> isize {
	loop {} 
}

#[no_mangle]
#[linkage="external"]
extern "C" fn rust_start(args: &[&OsStr]) -> ! {
	kernel_log!("rust_start(args={:?})", args);
	loop {}
}

