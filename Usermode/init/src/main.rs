// Tifflin OS - init
// - By John Hodge (thePowersGang)
//
// First userland process started
// - Maintains daemons and manages group masters

#[macro_use]
extern crate tifflin_syscalls;

extern crate tifflin_process;

fn main()
{
	kernel_log!("Hello userland!");
	
	//let daemons = Vec::new();
	//let shells = Vec::new();

	tifflin_process::Process::spawn("/Tifflin/bin/login");
}

