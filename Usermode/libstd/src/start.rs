//
//
//
//! Language entrypoint
#![cfg(not(test))]

#[lang="termination"]
pub trait Termination
{
	fn report(self) -> i32;
}

#[lang="start"]
fn lang_start<T: Termination+'static>(main: fn()->T, argc: isize, argv: *const *const u8) -> isize {
	#[cfg(arch="native")]
	{
		::syscalls::raw::native_init(32245);
		// SAFE: This is single-threaded, and trusts its inputs
		unsafe {
			let args_ptr = core::slice::from_raw_parts(argv, argc as usize);
			crate::env::register_arguments_native(args_ptr);
		}
	}
	kernel_log!("lang_start(main={:p}, argc={}, argv={:p})", main, argc, argv);
	
	main().report() as isize
}

impl Termination for ()
{
	fn report(self) -> i32 { 0 }
}
impl<T,E> Termination for Result<T,E>
where
	T: Termination//,
	//E: ::error::Error
{
	fn report(self) -> i32
	{
		match self
		{
		Ok(v) => v.report(),
		Err(_e) => 1,
		}
	}
}

// register_arguments is defined in std::env

