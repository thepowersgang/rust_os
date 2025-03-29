// Tifflin OS - IRC Client
// - By John Hodge (thePowersGang)
//
// TODO:
// - How should different channels/views be handled?

extern crate wtk;
extern crate r#async;

fn main()
{
	::wtk::initialise();
	
	let (status_window,status_window_ele) = window_types::StatusWindow::new();
	let ele_win_main = {
		use ::wtk::elements::static_layout::{Box,BoxEle};
		Box::new_vert((
			BoxEle::expand( status_window_ele ),
			BoxEle::fixed(32, ::wtk::elements::input::TextInput::new()),
			//BoxEle::fixed(32, ::wtk::elements::controls::TabBar::new()),
			))
		};

	let mut win_main = {
		let mut win = ::wtk::Window::new("IRC", &ele_win_main, ::wtk::Colour::from_argb32(0xAAA_AAA), ()).unwrap();
		win.focus(ele_win_main.inner().1.inner());
		win
	};
	
	let mut sm = server_manager::ServerManager::new(status_window);
	if false {
		sm.open_connection("Local".to_string(), ::std::net::Ipv4Addr::from_octets(10,0,0,1).into(), 6667);
	}

	win_main.show();

	::r#async::idle_loop(&mut [
		&mut win_main,
		&mut sm,
		]);
}

mod window_types;

mod rich_text_ele;
mod server_manager;
mod server_state;
