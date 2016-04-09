// Tifflin OS - init
// - By John Hodge (thePowersGang)
//
// First userland process started
// - Maintains daemons and manages group masters

#[macro_use]
extern crate syscalls;

extern crate loader;

fn main()
{
	kernel_log!("Hello userland!");
	
	//let daemons = Vec::new();
	//let shells = Vec::new();

	let session_root = {
		let pp = loader::new_process(b"/sysroot/bin/login", &[]).expect("Could not load login");

		pp.send_obj({
			let wingrp = syscalls::gui::Group::new("Session 1").unwrap();
			wingrp.force_active().expect("Cannot force session 1 to be active");
			wingrp
			});
		pp.start()
		};

	loop {
		let mut waits = [session_root.wait_terminate()];
		::syscalls::threads::wait(&mut waits, !0);
		drop(session_root);	// drop before panicking (leads to better reaping)
		
		// Empty wait set for ???
		::syscalls::threads::wait(&mut [], !0);

		panic!("TODO: Handle login terminating");
	}
}

