// Tifflin OS - login
// - By John Hodge (thePowersGang)
//
// Graphical user session root - Provides background, taskbar and menus

extern crate wtk;
extern crate r#async;
//#[macro_use]
//extern crate syscalls;

fn main()
{
	::wtk::initialise();
	
	let ele_win_main = {
		use ::wtk::elements::static_layout::{Box,BoxEle};
		Box::new_vert((
			BoxEle::expand( TextConsole::new(1024)),
			BoxEle::fixed(32, ::wtk::elements::input::TextInput::new()),
			//BoxEle::fixed(32, ::wtk::elements::controls::TabBar::new()),
			))
		};
	let log_ele = ele_win_main.inner().0.inner();
	
	log_ele.new_line();
	log_ele.append_text(0, None, None, "Hello!");

	let mut win_main = {
		let mut win = ::wtk::Window::new("IRC", &ele_win_main, ::wtk::Colour::from_argb32(0xAAA_AAA), ()).unwrap();
		win.focus(ele_win_main.inner().1.inner());
		win
	};
	
	// TODO: Create a server connection

	win_main.show();

	::r#async::idle_loop(&mut [
		&mut win_main,
		]);
}

mod rich_text_ele;
use self::rich_text_ele::TextConsole;
