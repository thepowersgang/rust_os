// NativeKernel - the rust_os/tifflin kernel running as a userland process
//
// See `README.md` for more details

extern crate core;
#[macro_use]
extern crate kernel;
extern crate syscalls;
extern crate vfs;

mod fs_shim;
mod video_shim;
#[cfg(feature="network")]
mod net_shim;

mod server;


fn main()// -> Result<(), Box<dyn std::error::Error>>
{
	// Initialise the kernel (very similar to logic in `Kernel/main/main.rs`)
	::kernel::threads::init();
	::kernel::memory::phys::init();
	::kernel::memory::page_cache::init();
	
	(::kernel::metadevs::storage::S_MODULE.init)();
	(::kernel::metadevs::video::S_MODULE.init)();
	(::vfs::S_MODULE.init)();
	(::network::S_MODULE.init)();

	// Simulated Keyboard/Mouse (started before GUI, but after video metadev)
	let console = video_shim::Console::new();
	::core::mem::forget( ::kernel::metadevs::video::add_output(Box::new(console.get_display())) );

	(::gui::S_MODULE.init)();

	// Native filesystem shim
	::core::mem::forget( ::vfs::mount::DriverRegistration::new("native", &fs_shim::NativeFsDriver) );
	// Network card shim
	#[cfg(feature="network")]
	{
		let mac_addr = [0xAA,0xBB,0xCC,0x00,0x00,0x01];
		::core::mem::forget( ::network::nic::register(mac_addr, net_shim::Nic::new(mac_addr)) );
		//let _ = ::network::ipv4::add_interface(mac_addr, ::network::ipv4::Address([192,168,1,3]), 24);
	}

	// TODO: Also load actual filesystem drivers?
	//(::fs_fat::S_MODULE.init)();
	//(::fs_extN::S_MODULE.init)();

	let sysdisk = "nullw";
	match ::kernel::metadevs::storage::VolumeHandle::open_named(sysdisk)
	{
	Err(e) => {
		panic!("Unable to open /system volume {}: {}", sysdisk, e);
		},
	Ok(vh) => match ::vfs::mount::mount("/system".as_ref(), vh, "native", &[])
		{
		Ok(_) => {},
		Err(e) => {
			panic!("Unable to mount /system from {}: {:?}", sysdisk, e);
			},
		},
	}
	::vfs::handle::Dir::open(::vfs::Path::new("/")).unwrap()
		.symlink("sysroot", ::vfs::Path::new("/system/Tifflin"))
		.unwrap()
		;

	let server = match ::std::net::TcpListener::bind( ("127.0.0.1", 32245) )
		{
		Ok(v) => v,
		Err(e) => panic!("bind() failed: {}", e),
		};

	let init = "/bin/init";
	let init_args: Vec<_> = ::std::env::args().skip(1).collect();
	
	let init_fh = ::vfs::handle::File::open(
			::vfs::Path::new(&format!("/sysroot{init}")),
			::vfs::handle::FileOpenMode::Execute
		)
		.unwrap();
	::syscalls::init(init_fh);

	let init_path = format!(".native_fs/Tifflin{init}{suf}", suf=if cfg!(windows) {".exe"} else {""});
	let init_proc = ::std::process::Command::new(init_path)
		.args(init_args)
		.spawn().expect("Failed to spawn init");

	let gs_root = ::std::sync::Arc::new(::std::sync::Mutex::new(
			server::GlobalState::new( init_proc )
		));

	// Run a thread that monitors for closed tasks.
	if false
	{
		let gs_root = gs_root.clone();
		::std::thread::spawn(move || {
			loop
			{
				gs_root.lock().unwrap().check_for_terminated();
				
				// Sleep for a short period before checking for exit again
				std::thread::sleep(std::time::Duration::from_millis(100));
			}
		});
	}
	
	server::main_loop(server, gs_root);
}
