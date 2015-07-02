// Tifflin OS - init
// - By John Hodge (thePowersGang)
//
// First userland process started

#[macro_use]
extern crate tifflin_syscalls;

fn main()
{
	kernel_log!("Hello userland!");
}

