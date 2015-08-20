// Tifflin OS - login
// - By John Hodge (thePowersGang)
//
// GUI root process, handling user logins on a single session

extern crate wtk;

extern crate async;

#[macro_use]
extern crate syscalls;

extern crate tifflin_process;

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
	let options_icon = ::wtk::Colour::theme_text_bg();	//::wtk::image::RasterMonoA::new("/Tifflin/shared/images/options.r32");
	let power_icon   = ::wtk::Colour::theme_text_bg();	//::wtk::image::RasterMonoA::new("/Tifflin/shared/images/power.r32");
	let mut options_button = ::wtk::Button::new( ::wtk::Image::new(options_icon) );
	options_button.bind_click( |_btn,_win| () );
	let mut power_button = ::wtk::Button::new( ::wtk::Image::new(power_icon) );
	power_button.bind_click( |_btn,_win| () );
	let mut menubar = ::wtk::Box::new_horiz();
	menubar.add(&options_button, Some(MENU_BTN_WIDTH));
	menubar.add_fill(None);
	menubar.add(&power_button, Some(MENU_BTN_WIDTH));

	// Login box (vertially stacked, centered)
	let mut username = ::wtk::TextInput::new();
	username.set_shadow("Username");
	
	let mut password = ::wtk::TextInput::new();
	password.set_shadow("Password");
	password.set_obscured('\u{2022}');	// Bullet

	username.bind_submit(|_uname, win| win.tabto(2));
	password.bind_submit(|password, win| {
		//win.hide();
		if let Err(reason) = try_login(&username.get_content(), &password.get_content()) {
			// TODO: Print error to the screen, as an overlay
			//win.show_message("Login Failed", reason);
		}
		else {
			// try_login also spawns and waits for the shell
		}
		// - Clear username/password and tab back to the username field
		username.clear();
		password.clear();
		win.tabto(1);
		//win.show();
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

fn try_login(username: &str, password: &str) -> Result<(), &'static str>
{
	kernel_log!("username = \"{}\", password = \"{}\"", username, password);
	// TODO: Use a proper auth infrastructure
	if username == "root" && password == "password"
	{
		// Spawn console, and wait for it to terminate
		spawn_console_and_wait("/sysroot/bin/simple_console");
		Ok( () )
	}
	else
	{
		Err( "Invalid username or password" )
	}
}

fn spawn_console_and_wait(path: &str)
{
	// TODO: I need something more elegant than this.
	// - Needs to automatically pass the WGH
	let console = tifflin_process::Process::spawn("/sysroot/bin/simple_console");
	console.send_obj( ::syscalls::gui::clone_group_handle() );
	::syscalls::threads::wait(&mut [console.wait_terminate()], !0);
}

