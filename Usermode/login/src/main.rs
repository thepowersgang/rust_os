// Tifflin OS - login
// - By John Hodge (thePowersGang)
//
// GUI root process, handling user logins on a single session

extern crate wtk;

extern crate async;

#[macro_use]
extern crate syscalls;

fn main()
{
	// Obtain window group from parent
	{
		use syscalls::Object;
		use syscalls::threads::S_THIS_PROCESS;
		::syscalls::threads::wait(&mut [S_THIS_PROCESS.get_wait()], !0);
		::syscalls::gui::set_group( S_THIS_PROCESS.receive_object::<::syscalls::gui::Group>(0).unwrap() );
	}

	let mut username = ::wtk::TextInput::new();
	username.set_shadow("Username");
		
	let mut password = ::wtk::TextInput::new();
	password.set_shadow("Password");
	password.set_obscured('\u{2022}');	// Bullet

	username.bind_submit(|_uname, win| win.tabto(2));
	password.bind_submit(|password, _win| {
		let uname = username.get_content();
		let pword = password.get_content();
		kernel_log!("username = \"{}\", password = \"{}\"", uname, pword);
		if uname == "root" && pword == "password" {
			// TODO: Spawn console, and wait for it to terminate
		}
		});

	let mut fvbox = ::wtk::Box::new_vert();
	fvbox.add_fill(None);
	fvbox.add(&username, Some(16));
	fvbox.add(&password, Some(16));
	fvbox.add_fill(None);

	let mut frame = ::wtk::Frame::new();
	frame.add(&fvbox);

	let mut hbox = ::wtk::Box::new_horiz();
	hbox.add_fill(None);
	hbox.add(&frame, Some(80));
	hbox.add_fill(None);

	let mut vbox = ::wtk::Box::new_vert();
	vbox.add_fill(None);
	vbox.add(&hbox, Some(40));
	vbox.add_fill(None);

	let mut win = ::wtk::Window::new(&vbox);
	win.undecorate();
	win.maximise();

	win.taborder_add( 1, &username );
	win.taborder_add( 2, &password );

	win.focus( &username );

	win.show();

	::async::idle_loop(&mut [
		&mut win,
		]);
}
