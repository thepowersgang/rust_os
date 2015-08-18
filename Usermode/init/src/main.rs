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

	let session_root = tifflin_process::Process::spawn("/sysroot/bin/login");

	let wingrp = syscalls::gui::Group::new("Session 1").unwrap();
	wingrp.force_active();
	session_root.send_obj(wingrp);
	loop {
		::syscalls::threads::wait(&mut [session_root.wait_terminate()], !0);
	}
}

