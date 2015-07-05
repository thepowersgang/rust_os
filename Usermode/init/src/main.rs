// Tifflin OS - init
// - By John Hodge (thePowersGang)
//
// First userland process started
// - Maintains daemons and manages group masters

#[macro_use]
extern crate tifflin_syscalls;

fn main()
{
	kernel_log!("Hello userland!");
	
	//let daemons = Vec::new();
	//let shells = Vec::new();

	//::std::process::spawn("/Tifflin/bin/login");
}

