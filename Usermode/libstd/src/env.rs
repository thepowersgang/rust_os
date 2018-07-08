//
//
//
use ::alloc::vec::Vec;
use ::ffi::OsString;

static mut S_ARGUMENTS: &'static [OsString] = &[];

#[no_mangle]
#[linkage="external"]
#[allow(private_no_mangle_fns)]
#[allow(dead_code)]
extern "C" fn register_arguments(args: &[&::ffi::OsStr]) {
	kernel_log!("register_arguments(args={:?})", args);
	let args: Vec<_> = args.iter().map(|&a| OsString::from(a)).collect();
	// SAFE: Runs in a single-threaded context
	unsafe {
		//S_ARGUMENTS = args.into_static();
		S_ARGUMENTS = &*(&args[..] as *const [_]);
		::core::mem::forget(args);
	}
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
