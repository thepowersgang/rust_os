// Tifflin OS - login
// - By John Hodge (thePowersGang)
//
// GUI root process, handling user logins on a single session

extern crate wtk;

extern crate async;

#[macro_use]
extern crate syscalls;

extern crate tifflin_process;

macro_rules! imgpath {
		($p:expr) => {concat!("/system/Tifflin/shared/images/",$p)};
}

fn main()
{
	const MENU_BTN_WIDTH: u32 = 16;
	const MENU_HEIGHT: u32 = 16;
	const ENTRY_FRAME_HEIGHT: u32 = 40;
	const TEXTBOX_HEIGHT: u32 = 16;

	::wtk::initialise();

	// Menu bar
	// - Two buttons: Options and power
	let options_icon = ::wtk::image::RasterMonoA::new(imgpath!("options.r8"), ::wtk::Colour::theme_text_bg()).unwrap();
	let power_icon   = ::wtk::image::RasterMonoA::new(imgpath!("power.r8"  ), ::wtk::Colour::theme_text_bg()).unwrap();
	let mut options_button = ::wtk::Button::new( ::wtk::Image::new(options_icon) );
	options_button.bind_click( |_btn,_win| () );
	let mut power_button = ::wtk::Button::new( ::wtk::Image::new(power_icon) );
	power_button.bind_click( |_btn,_win| () );
	let menubar = ::wtk::StaticBox::new_horiz( (
		::wtk::BoxEle::fixed(MENU_BTN_WIDTH, options_button),
		::wtk::BoxEle::expand( () ),
		::wtk::BoxEle::fixed(MENU_BTN_WIDTH, power_button),
		));

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
			kernel_log!("Login failed - {:?}", reason);
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

	let loginbox = ::wtk::Frame::new_fat( ::wtk::StaticBox::new_vert((
		::wtk::BoxEle::expand( () ),
		::wtk::BoxEle::fixed( TEXTBOX_HEIGHT, &username ),
		::wtk::BoxEle::fixed( 1, () ),	// <-- Padding
		::wtk::BoxEle::fixed( TEXTBOX_HEIGHT, &password ),
		::wtk::BoxEle::expand( () ),
		)) );

	let hbox = ::wtk::StaticBox::new_horiz((
		::wtk::BoxEle::expand( () ),
		::wtk::BoxEle::fixed(120, &loginbox),
		::wtk::BoxEle::expand( () ),
		));

	let vbox = ::wtk::StaticBox::new_vert((
		::wtk::BoxEle::fixed( MENU_HEIGHT, &menubar),
		::wtk::BoxEle::expand( () ),
		::wtk::BoxEle::fixed(ENTRY_FRAME_HEIGHT, &hbox),
		::wtk::BoxEle::expand( () ),
		::wtk::BoxEle::fixed( MENU_HEIGHT, () ),
		));

	let mut win = ::wtk::Window::new("Login", &vbox, ::wtk::Colour::theme_body_bg());
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
		//spawn_console_and_wait("/sysroot/bin/simple_console");
		spawn_console_and_wait("/sysroot/bin/shell");
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
	// - OR - Just have a wtk method to pass it `::wtk::share_handle(&console)`
	let console = tifflin_process::Process::spawn(path);
	console.send_obj( ::syscalls::gui::clone_group_handle() );
	::syscalls::threads::wait(&mut [console.wait_terminate()], !0);
}

