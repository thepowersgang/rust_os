// Tifflin OS - login
// - By John Hodge (thePowersGang)
//
// Graphical user session root - Provides background, taskbar and menus

extern crate wtk;
extern crate async;

macro_rules! imgpath {
		($p:expr) => {concat!("/system/Tifflin/shared/images/",$p)};
}

fn main()
{
	::wtk::initialise();


	let background = {
		// Background image is "Ferris the crab" - credit to rustacean.net
		let img = ::wtk::image::RasterRGB::new_img(imgpath!("background.r24")).expect("Cannot load background");
		img
		};
	let mut win_background = {
		let mut win = ::wtk::Window::new("Background", &background, ::wtk::Colour::from_argb32(0x01346B));	// 01346B is Ferris's background colour
		win.undecorate();
		win.maximise();
		win
		};
	let menubar = {
		let logo_button = ::wtk::Button::new( () );
		let taskbar = ();
		let clock_widget = ::wtk::Label::new("12:34", ::wtk::Colour::theme_text());
		let mut power_button = ::wtk::Button::new( ::wtk::image::RasterMonoA::new_img(imgpath!("power.r8"), ::wtk::Colour::theme_text()).unwrap() );
		power_button.bind_click(|_button, _window| {});
		::wtk::StaticBox::new_horiz((
			::wtk::BoxEle::fixed(20, logo_button),
			::wtk::BoxEle::expand(taskbar),
			::wtk::BoxEle::fixed(50, clock_widget),
			::wtk::BoxEle::fixed(20, power_button),
			))
		};
	let mut win_menu = {
		let mut win = ::wtk::Window::new("SystemBar", &menubar, ::wtk::Colour::theme_text_bg());
		win.undecorate();
		win.set_pos(0, 0);
		win.set_dims(1920,20);
		win
		};

	win_background.show();
	win_menu.show();

	::async::idle_loop(&mut [
		&mut win_background,
		&mut win_menu,
		]);

}


