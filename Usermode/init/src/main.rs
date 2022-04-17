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
	let root_app = "/sysroot/bin/login";
	let root_args = [];

	kernel_log!("Tifflin (rust_os) userland started");

	let rw_root: ::syscalls::vfs::Dir = get_handle("RW VFS Root", "RwRoot");
	
	//let daemons = Vec::new();
	//let shells = Vec::new();

	let session_root = {
		let pp = loader::new_process(open_exec(root_app), root_app.as_bytes(), &root_args)
			.expect("Could not start root process");

		pp.send_obj("guigrp", {
			let wingrp = syscalls::gui::Group::new("Session 1").unwrap();
			wingrp.force_active().expect("Cannot force session 1 to be active");
			wingrp
			});
		pp.send_obj("RwRoot", rw_root.clone() );
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

fn get_handle<T: ::syscalls::Object>(desc: &str, tag: &str) -> T
{
	match ::syscalls::threads::S_THIS_PROCESS.receive_object(tag)
	{
	Ok(v) => v,
	Err(e) => panic!("Failed to receive {} - {:?}", desc, e),
	}
}

fn open_exec(path: &str) -> ::syscalls::vfs::File
{
	match ::syscalls::vfs::root().open_child_path(path.as_bytes())
	{
	Ok(v) => match v.into_file(::syscalls::vfs::FileOpenMode::Execute)
		{
		Ok(v) => v,
		Err(e) => panic!("Couldn't open '{}' as an executable file - {:?}", path, e),
		},
	Err(e) => panic!("Couldn't open executable '{}' - {:?}", path, e),
	}
}

