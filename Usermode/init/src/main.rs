// Tifflin OS - init
// - By John Hodge (thePowersGang)
//
// First userland process started
// - Maintains daemons and manages group masters

#[macro_use]
extern crate syscalls;

extern crate tifflin_process;

fn main()
{
	kernel_log!("Hello userland!");
	
	//let daemons = Vec::new();
	//let shells = Vec::new();

	//tifflin_process::Process::spawn("/sysroot/bin/login");
	let console = tifflin_process::Process::spawn("/sysroot/bin/simple_console");
    let wingrp = syscalls::gui::Group::new("Session 1").unwrap();
    console.send_obj(wingrp);
}

