//
//
//
//! Language entrypoint

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

// register_arguments is defined in std::env

