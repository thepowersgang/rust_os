#![feature(str_split_whitespace_remainder)]

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
	
	let mut fetch = Fetch {
		socket: None,
		handler: Handler { console: &console_ele },
		input: &input,
	};

	*input.0.borrow_mut() = "192.168.1.39 GET /".to_owned();

	::r#async::idle_loop(&mut [
		&mut fetch,
		&mut win_main,
		]);
}

struct LineReader {
	socket: ::std::net::TcpStream,
	partial_line: Vec<u8>,
}
impl LineReader {
	pub fn new(socket: ::std::net::TcpStream) -> Self {
		LineReader { socket, partial_line: Default::default() }
	}
	pub fn read_line(&mut self, mut cb: impl FnMut(&[u8])) -> ::std::io::Result<()> {
		use std::io::Read;

		let mut read_buf = vec![0; 1024];
		let len = self.socket.read(&mut read_buf)?;
		let mut data = &read_buf[..len];
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
	fn wait_item(&self) -> ::syscalls::WaitItem {
		use ::std::net::TcpStreamExt;
		self.socket.wait_item()
	}
}

struct Handler<'a> {
	console: &'a ::wtk_ele_console::TextConsole,
}

struct Fetch<'a> {
	socket: Option<LineReader>,
	handler: Handler<'a>,
	input: &'a Input,
}
impl ::r#async::WaitController for Fetch<'_> {
	fn get_count(&self) -> usize {
		if let Some(_) = &self.socket {
			1
		}
		else {
			0
		}
	}

	fn populate(&self, cb: &mut dyn FnMut(syscalls::WaitItem)) {
		if let Some(v) = &self.socket {
			cb(v.wait_item())
		}
	}

	fn handle(&mut self, events: &[syscalls::WaitItem]) {
		if let Some(v) = self.socket.as_mut() {
			if events[0].flags != 0 {
				let h = &mut self.handler;
				match v.read_line(|line| {
					h.handle_line(line);
				}) {
				Ok( () ) => {},
				Err(e) => {
					self.handler.io_error(e);
					self.socket = None;
				}
				}
			}
		}
		else {
			if let Some(t) = self.input.take() {
				let mut v = t.split_whitespace();
				let host = v.next().unwrap();
				match (v.next(), v.remainder()) {
				(Some(method), Some(path)) => {
					match http_get_request(host, method, path)
					{
					Ok(v) => {
						self.socket = Some(LineReader::new(v));
					},
					Err(e) => {
						self.handler.io_error(e);
					},
					}
					},
				_ => {
				},
				}
			}
		}
	}
}
impl Handler<'_> {
	fn handle_line(&mut self, line: &[u8]) {
		let text  = String::from_utf8_lossy(line);
		::syscalls::kernel_log!("handle_line({:?})", text);
		self.console.new_line();
		self.console.append_text(0, &text);
	}
	fn io_error(&mut self, error: ::std::io::Error) {
		self.console.new_line();
		self.console.append_fg_set(0, Some(::wtk::Colour::theme_text_alt()));
		self.console.append_fmt(0, format_args!("Network Error: {:?}", error));
	}
}

fn http_get_request(host_str: &str, method: &str, path: &str) -> ::std::io::Result< ::std::net::TcpStream > {
	let host: ::std::net::IpAddr = host_str.parse().unwrap();
	let mut s = ::std::net::TcpStream::connect(host, 80)?;
	use ::std::io::Write;
	s.write_all(&format!("{method} {path} HTTP/1.1\r\nHost: {host_str}\r\n\r\n").as_bytes())?;
	Ok( s )
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