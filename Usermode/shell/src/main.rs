// Tifflin OS - login
// - By John Hodge (thePowersGang)
//
// Graphical user session root - Provides background, taskbar and menus

extern crate wtk;
extern crate async;
#[macro_use]
extern crate syscalls;

macro_rules! imgpath {
		($p:expr) => {concat!("/system/Tifflin/shared/images/",$p)};
}

fn main()
{
	::wtk::initialise();


	let power_menu = {
		use wtk::menu::{Menu,Entry,Spacer};
		Menu::new("Power Menu", (
			Entry::new("Lock", 0, "", || {}),
			Entry::new("Logout", 1, "", || {}),
			Spacer,
			Entry::new("Restart", 0, "", || {}),
			Entry::new("Shut Down", 0, "", || {}),
			))
		};
	//power_menu.set_pos( Pos::new(0,20) );
	let system_menu = {
		use wtk::menu::{Menu,Entry,Spacer};
		Menu::new("System Menu", (
			Entry::new("CLI", 0, "Win-T", || start_app(&["/sysroot/bin/simple_console", "--windowed"])),
			Spacer,
			Entry::new("Filesystem", 0, "Win-E", || start_app(&["/sysroot/bin/filebrowser", "--windowed"])),
			Entry::new("Text Editor", 5, "", || kernel_log!("TODO: Spawn text editor")),
			))
		};
	system_menu.set_pos( ::wtk::geom::Pos::new(0,20) );
	
	let background = {
		// Background image is "Ferris the crab" - credit to rustacean.net
		let img = ::wtk::image::RasterRGB::new_img(imgpath!("background.r24")).expect("Cannot load background");
		img
		};
	let mut win_background = {
		let mut win = ::wtk::Window::new("Background", &background, ::wtk::Colour::from_argb32(0x01346B), ()).unwrap();	// 01346B is Ferris's background colour
		win.maximise();
		win
		};
	
	let menubar = {
		let logo_button = ::wtk::Button::new(
			::wtk::image::RasterMonoA::new_img(imgpath!("menu.r8"), ::wtk::Colour::theme_text()).expect("Error loading menu icon"),
			|_,_| system_menu.show()
			);
		let taskbar = ();
		let clock_widget = ::wtk::Label::new("12:34", ::wtk::Colour::theme_text());
		let power_button = ::wtk::Button::new(
			::wtk::image::RasterMonoA::new_img(imgpath!("power.r8"), ::wtk::Colour::theme_text()).expect("Error loading power icon"),
			|_button, _window| power_menu.show()
			);
		::wtk::StaticBox::new_horiz((
			::wtk::BoxEle::fixed(20, logo_button),
			::wtk::BoxEle::expand(taskbar),
			::wtk::BoxEle::fixed(50, clock_widget),
			::wtk::BoxEle::fixed(20, power_button),
			))
		};
	let mut win_menu = {
		let mut win = ::wtk::Window::new("SystemBar", &menubar, ::wtk::Colour::theme_text_bg(), ()).unwrap();
		win.set_pos(0, 0);
		win.set_dims(1920,20);
		//win.taborder_add(0, &menubar.inner().0);
		//win.taborder_add(1, &menubar.inner().3);
		win
		};
	
	win_menu.add_shortcut_1( ::syscalls::gui::KeyCode::LeftGui, || system_menu.show() );
	win_menu.add_shortcut_1( ::syscalls::gui::KeyCode::RightGui, || system_menu.show() );

	win_background.show();
	win_menu.show();

	::async::idle_loop(&mut [
		&mut win_background,
		&mut win_menu,
		&mut system_menu.waiter(),
		&mut power_menu.waiter(),
		]);

}

fn start_app(args: &[&str]) {
	extern crate loader;
	kernel_log!("start_app(args={:?})", args);
	// SAFE: &str and &[u8] have the same representation
	let byte_args: &[&[u8]] = unsafe { ::std::mem::transmute(&args[1..]) };
	match loader::new_process(args[0].as_bytes(), byte_args)
	{
	Ok(app) => {
		app.send_obj( ::syscalls::gui::clone_group_handle() );
		},
	Err(_e) => {},
	}
}


