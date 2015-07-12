// Tifflin OS - Userland loader
// - By John Hodge (thePowersGang)
//
// interface.rs
// - Exposed process spawning interface

// Import the interface crate
extern crate loader;

#[no_mangle]
pub extern "C" fn new_process(binary: &[u8], args: &[&[u8]]) -> Result<::tifflin_syscalls::Process,loader::Error>
{
	extern "C" {
		static BASE: [u8; 0];
		static LIMIT: [u8; 0];
		static init_stack_end: [u8; 0];
	}
	kernel_log!("new_process('{:?}', ...)", ::std::ffi::OsStr::new(binary));
	// Lock loader until after 'start_process', allowing global memory to be used as buffer for binary and arguments
	//let lh = S_BUFFER_LOCK.lock();
	match ::tifflin_syscalls::start_process(new_process_entry as usize, init_stack_end.as_ptr() as usize, BASE.as_ptr() as usize, LIMIT.as_ptr() as usize)
	{
	Ok(v) => Ok( v ),
	Err(e) => panic!("TODO: new_process - Error '{:?}'", e),
	}
}

/// Entrypoint for new processes, runs with a clean stack
fn new_process_entry() -> !
{
	kernel_log!("new_process_entry");
	loop {}
}

