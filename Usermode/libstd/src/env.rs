//
//
//
use ::alloc::vec::Vec;
use ::ffi::OsString;

static mut S_ARGUMENTS: &'static [OsString] = &[];

#[no_mangle]
#[linkage="external"]
#[allow(dead_code)]
#[allow(improper_ctypes_definitions)]	//< The target is rust code too
extern "C" fn register_arguments(args: &[&::ffi::OsStr]) {
	kernel_log!("register_arguments(args={:?})", args);
	let args: Vec<_> = args.iter().map(|&a| OsString::from(a)).collect();
	// SAFE: Runs in a single-threaded context
	unsafe {
		S_ARGUMENTS = ::alloc::boxed::Box::leak(args.into_boxed_slice());
	}
}

#[cfg(arch="native")]
pub(crate) unsafe fn register_arguments_native(args: &[*const u8]) {
	extern "C" {
		fn memchr(buf: *const u8, val: u8, size: usize) -> *const u8;
	}
	kernel_log!("register_arguments_native(args={:?})", args);
	let args: Vec<_> = args.iter()
		.map(|&ptr| (ptr, memchr(ptr, 0, usize::MAX)))
		.map(|(ptr,end)| ::core::slice::from_raw_parts(ptr, end.offset_from(ptr) as usize))
		.map(|v| crate::ffi::OsStr::new(v))
		.map(|a| OsString::from(a))
		.collect();
	kernel_log!("- register_arguments_native: {:?}", args);
	S_ARGUMENTS = ::alloc::boxed::Box::leak(args.into_boxed_slice());
}

pub struct ArgsOs(usize);
pub fn args_os() -> ArgsOs {
	ArgsOs(0)
}
impl Iterator for ArgsOs {
	type Item = OsString;
	fn next(&mut self) -> Option<OsString> {
		// SAFE: The S_ARGUMENTS array is only ever altered at startup
		unsafe {
			if self.0 == S_ARGUMENTS.len() {
				None
			}
			else {
				self.0 += 1;
				Some( S_ARGUMENTS[self.0 - 1].clone() )
			}
		}
	}
}
