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
	const MENU_BTN_WIDTH: u32 = 16;
	const MENU_HEIGHT: u32 = 16;
	const ENTRY_FRAME_HEIGHT: u32 = 40;
	const TEXTBOX_HEIGHT: u32 = 16;

	// Obtain window group from parent
	{
		use syscalls::Object;
		use syscalls::threads::S_THIS_PROCESS;
		::syscalls::threads::wait(&mut [S_THIS_PROCESS.get_wait()], !0);
		::syscalls::gui::set_group( S_THIS_PROCESS.receive_object::<::syscalls::gui::Group>(0).unwrap() );
	}

	// Menu bar
	// - Two buttons: Options and power
	// TODO: At the moment, the image widget does nothing. Need to decide where images live, and what format.
	let mut options_button = ::wtk::Button::new( ::wtk::Image::new("/Tifflin/shared/images/options.r32") );
	options_button.bind_click( |_btn,_win| () );
	let mut power_button = ::wtk::Button::new( ::wtk::Image::new("/Tifflin/shared/images/power.r32") );
	power_button.bind_click( |_btn,_win| () );
	let mut menubar = ::wtk::Box::new_horiz();
	menubar.add(&options_button, Some(MENU_BTN_WIDTH));
	menubar.add_fill(None);
	menubar.add(&power_button, Some(MENU_BTN_WIDTH));

	// Login box (vertially staked, centered)
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
		// TODO: Use a proper auth infrastructure
		if &*uname == "root" && &*pword == "password" {
			// TODO: Spawn console, and wait for it to terminate
		}
		});

	let mut loginbox = ::wtk::Frame::new( ::wtk::Box::new_vert() );
	loginbox.inner_mut().add_fill(None);
	loginbox.inner_mut().add(&username, Some(TEXTBOX_HEIGHT));
	loginbox.inner_mut().add(&password, Some(TEXTBOX_HEIGHT));
	loginbox.inner_mut().add_fill(None);

	let mut hbox = ::wtk::Box::new_horiz();
	hbox.add_fill(None);
	hbox.add(&loginbox, Some(80));
	hbox.add_fill(None);

	let mut vbox = ::wtk::Box::new_vert();
	vbox.add(&menubar, Some(MENU_HEIGHT));
	vbox.add_fill(None);
	vbox.add(&hbox, Some(ENTRY_FRAME_HEIGHT));
	vbox.add_fill(None);
	vbox.add_fill(Some(MENU_HEIGHT));

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
