
pub struct Fetch<'a> {
	socket: ConnState,
	handler: Handler<'a>,
	input: &'a super::Input,
}
impl<'a> Fetch<'a> {
	pub fn new(console: &'a ::wtk_ele_console::TextConsole, input: &'a super::Input) -> Self {
		Fetch {
			socket: ConnState::NoConnection,
			handler: Handler { console, content_length: None },
			input: &input,
		}
	}
}
impl ::r#async::WaitController for Fetch<'_> {
	fn get_count(&self) -> usize {
		match &self.socket {
		ConnState::NoConnection => 0,
		ConnState::Body { .. } | ConnState::Headers(..) => 1,
		}
	}

	fn populate(&self, cb: &mut dyn FnMut(syscalls::WaitItem)) {
		match &self.socket {
		ConnState::NoConnection => {},
		ConnState::Body { socket, .. } => cb(::std::net::TcpStreamExt::wait_item(socket)),
		ConnState::Headers(v) => cb(v.wait_item()),
		}
	}

	fn handle(&mut self, events: &[syscalls::WaitItem]) {
		match &mut self.socket {
		ConnState::Headers(v) => {
			::syscalls::kernel_log!("Fetch::handle: events[0].flags={:#x}", events[0].flags);
			if events[0].flags != 0
			{
				// Read the headers first, then read based on the content-length header lengths
				let h = &mut self.handler;
				let mut body_remain = None;
				let r = v.read_line(|line| {
						let line = line.strip_suffix(b"\r").unwrap_or(line);
						if line.is_empty() {
							body_remain = Some(h.content_length.unwrap_or(0))
						}
						else if let Some(ref mut body_remain) = body_remain {
							h.handle_body_data(line);
							h.handle_body_data(b"\r\n");
							*body_remain = body_remain.saturating_sub( (line.len() + 2) as u32 );
						}
						else {
							h.handle_header_line(line);
						}
					});
				match r {
				Ok( () ) => {
					if let Some(body_remain) = body_remain {
						let ConnState::Headers(v) = ::std::mem::replace(&mut self.socket, ConnState::NoConnection) else { panic!() };
						let (socket, remain_data) = v.into_inner();
						h.handle_body_data(&remain_data);
						let body_remain = body_remain.saturating_sub(remain_data.len() as u32);
						if body_remain > 0 {
							self.socket = ConnState::Body {
								socket,
								remaining: body_remain,
							};
						}
						else {
							self.socket = ConnState::NoConnection;
						}
					}
				},
				Err(e) => {
					self.handler.io_error(e);
				}
				}
			}
			},
		ConnState::Body { socket, remaining } => {
			let mut buf = vec![0; 1024];
			let len = buf.len().min( *remaining as usize );
			use ::std::io::Read;
			match socket.read(&mut buf[..len])
			{
			Ok(len) => {
				if len == 0 {
					self.socket = ConnState::NoConnection;
				}
				else {
					self.handler.handle_body_data(&buf[..len]);
					*remaining -= len as u32;
				}
			},
			Err(e) => {
				self.handler.io_error(e);
				self.socket = ConnState::NoConnection;
				}
			}
		},
		ConnState::NoConnection => {
			if let Some(t) = self.input.take() {
				::syscalls::kernel_log!("Fetch::handle: Input {:?}", t);
				let mut v = t.split_whitespace();
				let host = v.next().unwrap();
				match (v.next(), v.remainder()) {
				(Some(method), Some(path)) => {
					match http_get_request(host, method, path)
					{
					Ok(v) => {
						self.socket = ConnState::Headers(super::LineReader::new(v))
					},
					Err(e) => {
						self.handler.io_error(e);
					},
					}
					},
				_ => {
					// TODO: parse error?
				},
				}
			}
			},
		}
	}
}

enum ConnState {
	NoConnection,
	Headers(super::LineReader),
	Body {
		socket: ::std::net::TcpStream,
		remaining: u32,
	}
}

struct Handler<'a> {
	console: &'a ::wtk_ele_console::TextConsole,
	content_length: Option<u32>,
}
impl Handler<'_> {
	fn handle_header_line(&mut self, line: &[u8]) {
		let text  = String::from_utf8_lossy(line);
		::syscalls::kernel_log!("handle_header_line({:?})", text);
		if let Some(tail) = text.strip_prefix("Content-Length: ") {
			self.content_length = Some( tail.parse().unwrap_or(0) )
		}
		self.console.new_line();
		self.console.append_text(0, &text);
	}
	fn handle_body_data(&mut self, data: &[u8]) {
		let mut it = data.split(|v| *v == b'\n' );
		let mut cur = String::from_utf8_lossy(it.next().unwrap().trim_ascii_end());
		for line in it {
			self.console.append_text(0, &cur);
			::syscalls::kernel_log!("handle_body_data: ended line={:?}", cur);
			self.console.new_line();
			cur = String::from_utf8_lossy(line.trim_ascii_end());
		}
		if !cur.is_empty() {
			::syscalls::kernel_log!("handle_body_data: tail line={:?}", cur);
			self.console.append_text(0, &cur);
		}
	}
	fn io_error(&mut self, error: ::std::io::Error) {
		::syscalls::kernel_log!("io_error: {:?}", error);
		self.console.new_line();
		self.console.append_fg_set(0, Some(::wtk::Colour::theme_text_alt()));
		self.console.append_fmt(0, format_args!("Network Error: {:?}", error));
	}
}

fn http_get_request(host_str: &str, method: &str, path: &str) -> ::std::io::Result< ::std::net::TcpStream > {
	let host: ::std::net::IpAddr = match host_str.parse()
		{
		Ok(v) => v,
		Err(e) => return Err(::std::io::Error::new_misc(format!("{:?}", e))),
		};
	let mut s = ::std::net::TcpStream::connect(host, 80)?;
	::syscalls::kernel_log!("http_get_request: connect called");
	use ::std::net::TcpStreamExt;
	::syscalls::threads::wait(&mut [s.raw().wait_conn()], !0);
	::syscalls::kernel_log!("http_get_request: connected");
	use ::std::io::Write;
	s.write_all(&format!("{method} {path} HTTP/1.1\r\nHost: {host_str}\r\n\r\n").as_bytes())?;
	::syscalls::kernel_log!("http_get_request: requested");
	Ok( s )
}