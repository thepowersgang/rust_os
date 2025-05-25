#![feature(str_split_whitespace_remainder)]

mod fetch;

fn main() {
	::wtk::initialise();
	let input = Input::default();

	let console_ele = ::wtk_ele_console::TextConsole::new(1024);
	let ele_win_main = {
		use ::wtk::elements::static_layout::{Box,BoxEle};
		Box::new_vert((
			BoxEle::expand(&console_ele),
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
		let mut win = ::wtk::Window::new("HTTP", &ele_win_main, ::wtk::Colour::from_argb32(0xAAA_AAA), ()).unwrap();
		win.maximise();
		win.focus(ele_win_main.inner().1.inner());
		win
	};
	win_main.show();
	//win_main.rerender();

	let fetch = fetch::Fetch::new(&console_ele, &input);

	// Trigger an immediate fetch
	*input.0.borrow_mut() = "192.168.1.39 GET /".to_owned();

	struct OuterWrap<'eles> {
		fetch: fetch::Fetch<'eles>,
		win_main: ::wtk::Window<'eles, ()>,
	}
	impl<'e> ::r#async::WaitController for OuterWrap<'e> {
		fn get_count(&self) -> usize {
			self.fetch.get_count() + self.win_main.get_count()
		}
	
		fn populate(&self, cb: &mut dyn FnMut(syscalls::WaitItem)) {
			self.fetch.populate(cb);
			self.win_main.populate(cb);
		}
	
		fn handle(&mut self, events: &[syscalls::WaitItem]) {
			let c1 = self.fetch.get_count();
			self.fetch.handle(&events[..c1]);
			self.win_main.handle(&events[c1..]);
			self.win_main.rerender();
		}
	}

	let mut ow = OuterWrap { fetch, win_main };
	::r#async::idle_loop(&mut [ &mut ow ]);
}

struct LineReader {
	socket: ::std::net::TcpStream,
	partial_line: Vec<u8>,
}
impl LineReader {
	pub fn new(socket: ::std::net::TcpStream) -> Self {
		LineReader { socket, partial_line: Default::default() }
	}
	// Read a single `\n` terminated line
	pub fn read_line(&mut self, mut cb: impl FnMut(&[u8])) -> ::std::io::Result<()> {
		use std::io::Read;

		let mut read_buf = vec![0; 1024];
		let len = self.socket.read(&mut read_buf)?;
		let mut data = &read_buf[..len];
		::syscalls::kernel_log!("read_line: len={} data={:x?}", len, data);
		while let Some(p) = data.iter().position(|v| *v == b'\n') {
			let line = &data[..p];
			if self.partial_line.is_empty() {
				cb(line);
			}
			else {
				self.partial_line.extend_from_slice(line);
				cb(&self.partial_line);
				self.partial_line.clear();
			}
			data = &data[p+1..];
		}
		self.partial_line.extend_from_slice(data);
		Ok( () )
	}
	fn into_inner(self) -> (::std::net::TcpStream, Vec<u8>) {
		(self.socket, self.partial_line)
	}
	fn wait_item(&self) -> ::syscalls::WaitItem {
		use ::std::net::TcpStreamExt;
		self.socket.wait_item()
	}
}


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