// Tifflin OS - login
// - By John Hodge (thePowersGang)
//
// Graphical user session root

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
		let mut win = ::wtk::Window::new(&background, ::wtk::Colour::from_argb32(0x01346B));	// 01346B is Ferris's background colour
		win.undecorate();
		win.maximise();
		win
		};
	let menubar = {
		let logo_button = ();
		let taskbar = ();
		let clock_widget = ::wtk::Label::new("12:34", ::wtk::Colour::theme_text());
		let power_button = ::wtk::Button::new( ::wtk::image::RasterMonoA::new_img(imgpath!("power.r8"), ::wtk::Colour::theme_text()).unwrap() );
		::wtk::StaticBox::new_horiz((
			::wtk::BoxEle::fixed(logo_button, 20),
			::wtk::BoxEle::expand(taskbar),
			::wtk::BoxEle::fixed(clock_widget, 50),
			::wtk::BoxEle::fixed(power_button, 20),
			))
		};
	let mut win_menu = {
		let mut win = ::wtk::Window::new(&menubar, ::wtk::Colour::theme_text_bg());
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


