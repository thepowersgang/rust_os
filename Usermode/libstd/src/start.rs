
use ffi::OsStr;

#[lang="start"]
fn lang_start(main: *const u8, argc: isize, argv: *const *const u8) -> isize {
	kernel_log!("lang_start(main={:p}, argc={}, argv={:p})", main, argc, argv);
	
	// SAFE: We're trusting that the pointer provied is the main function, and that it has the correct signature
	unsafe {
		let mainfcn: fn() = ::core::mem::transmute(main);
		mainfcn();
	}
	0
}

#[no_mangle]
#[linkage="external"]
#[allow(private_no_mangle_fns)]
#[allow(dead_code)]
extern "C" fn register_arguments(args: &[&OsStr]) {
	kernel_log!("register_arguments(args={:?})", args);
}

