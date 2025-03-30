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
	
	let input = Input::default();

	let (status_window,status_window_ele) = window_types::StatusWindow::new();
	let tabs = ::std::cell::RefCell::new({
		let mut e = ::wtk::elements::controls::TabView::new_below();
		e.add_tab("(status)", status_window_ele);
		e
		});
	let ele_win_main = {
		use ::wtk::elements::static_layout::{Box,BoxEle};
		Box::new_vert((
			BoxEle::expand(&tabs),
			BoxEle::fixed(20, {
				let mut e = ::wtk::elements::input::TextInput::new();
				e.bind_submit(|input_ele,_win| {
					*input.0.borrow_mut() = input_ele.get_content().to_owned();
					input_ele.clear();
				});
				e
			}),
			))
		};

	let mut win_main = {
		let mut win = ::wtk::Window::new("IRC", &ele_win_main, ::wtk::Colour::from_argb32(0xAAA_AAA), ()).unwrap();
		win.maximise();
		win.focus(ele_win_main.inner().1.inner());
		win
	};
	
	let mut sm = server_manager::ServerManager::new(status_window, &input, Tabs(&tabs));

	win_main.show();
	//win_main.rerender();
	// TODO: How can this tell the window to redraw when ServerManager updates contents?
	// - Could make `ServerManager` own the window, so it can request the redraw
	struct M<'a, 'sm, 'w> {
		win: &'a mut ::wtk::Window<'w, ()>,
		sm: &'a mut server_manager::ServerManager<'sm>,
	}
	impl<'a, 'sm, 'w> ::r#async::WaitController for M<'a, 'sm, 'w> {
		fn get_count(&self) -> usize {
			self.win.get_count() + self.sm.get_count()
		}
	
		fn populate(&self, cb: &mut dyn FnMut(syscalls::WaitItem)) {
			self.win.populate(cb);
			self.sm.populate(cb);
		}
	
		fn handle(&mut self, events: &[syscalls::WaitItem]) {
			let (a,b) = events.split_at(self.win.get_count());
			self.win.handle(a);
			self.sm.handle(b);
			self.win.rerender();
		}
	}
	::r#async::idle_loop(&mut [
		//&mut win_main,
		//&mut sm,
		&mut M { win: &mut win_main, sm: &mut sm },
		]);
}

mod window_types;

mod rich_text_ele;
mod server_manager;
mod server_state;

#[derive(Default)]
struct Input( ::std::cell::RefCell<String> );
impl Input {
	fn take(&self) -> Option<String> {
		let v = self.0.take();
		if v.is_empty() {
			None
		}
		else {
			Some(v)
		}
	}
}

struct Tabs<'a>(&'a ::std::cell::RefCell< ::wtk::elements::controls::TabView>);
impl<'a> Tabs<'a> {
	fn selected_idx(&self) -> usize {
		self.0.borrow().selected_idx()
	}
	fn add_tab(&self, server_name: &str, channel_name: &str) -> window_types::ChannelWindow {
		let (cw,ele) = window_types::ChannelWindow::new(channel_name.as_bytes());
		let tab_name = format!("{} [{}]", channel_name, server_name);
		self.0.borrow_mut().add_tab(tab_name, ele);
		cw
	}
}
