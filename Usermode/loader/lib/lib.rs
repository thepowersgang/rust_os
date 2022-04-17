// Tifflin OS - Userland loader interface
// - By John Hodge (thePowersGang)
//
// A dummy interface library that provides dynamically-linked interfaces to the loader
#![no_std]
#![crate_type="dylib"]
#![crate_name="loader"]

//extern crate std_rt;
extern crate syscalls;

use core::result::Result;

include!("../common.inc.rs");

/// Process still being initialised (not yet running)
pub struct ProtoProcess( ::syscalls::threads::ProtoProcess );

#[cfg(not(test))]
mod int {
	use core::result::Result;
	#[allow(improper_ctypes)]
	#[link(name="loader_dyn",kind="dylib")]
	extern "C"
	{
		// NOTES:
		// - Required data for spawning a new process:
		//  > Binary path
		//  > Arguments
		//  > ? Environment (could this be transferred using IPC during init?)
		//  > ? Handles (same thing really, send them over an IPC channel)
		pub fn new_process(executable_handle: ::syscalls::vfs::File, process_name: &[u8], args: &[&[u8]]) -> Result<::syscalls::threads::ProtoProcess,super::Error>;

		pub fn start_process(handle: ::syscalls::threads::ProtoProcess) -> ::syscalls::threads::Process;
	}
}
#[cfg(test)]
mod int {
	pub unsafe fn new_process(_executable_handle: ::syscalls::vfs::File, _process_name: &[u8], _args: &[&[u8]]) -> Result<::syscalls::threads::ProtoProcess,super::Error> {
		todo!("new_process");
	}
	pub unsafe fn start_process(_handle: ::syscalls::threads::ProtoProcess) -> ::syscalls::threads::Process {
		todo!("start_process");
	}
}

impl ProtoProcess
{
	pub fn from_syscall(v: ::syscalls::threads::ProtoProcess) -> ProtoProcess {
		ProtoProcess(v)
	}

	pub fn send_obj<T: ::syscalls::Object>(&self, tag: &str, obj: T) {
		self.0.send_obj( tag, obj );
	}

	pub fn start(self) -> ::syscalls::threads::Process {
		// SAFE: FFI into rust code
		unsafe {
			int::start_process(self.0)
		}
	}
}

pub fn new_process(binary_file: ::syscalls::vfs::File, binary: &[u8], args: &[&[u8]]) -> Result<ProtoProcess,Error> {
	// SAFE: Call is actually to rust
	unsafe {
		int::new_process(binary_file, binary, args).map( |v| ProtoProcess(v) )
	}
}

