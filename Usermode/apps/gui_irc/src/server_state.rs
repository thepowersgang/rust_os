use crate::window_types::{StatusWindow,ChannelWindow};


pub struct ServerState {
	server_name: String,
	status_window: StatusWindow,
	windows: ::std::collections::BTreeMap<Vec<u8>,ChannelWindow>,
	is_connected: bool,
}
impl ServerState {
	pub fn new(server_name: String, status_window: StatusWindow) -> Self {
		ServerState {
			server_name,
			status_window,
			windows: Default::default(),
			is_connected: false,
		}
	}
	pub fn name(&self) -> &str {
		&self.server_name
	}

	fn get_window(&mut self, name: &[u8]) -> Option<&ChannelWindow> {
		self.windows.get(name)
	}
	pub fn send_message(&mut self, mut conn: &::std::net::TcpStream,  channel_name: &str, message: &str) -> ::std::io::Result<()> {
		if let Some(v) = self.get_window(channel_name.as_bytes()) {
			v.append_message(b"-ME-", message.as_bytes());
			::std::io::Write::write_all(&mut conn, format!("PRIVMSG {} :{}\r\n", channel_name, message).as_bytes())?
		}
		Ok( () )
	}
	pub fn join_channel(&mut self, mut conn: &::std::net::TcpStream,  channel_name: &str, window: ChannelWindow) -> ::std::io::Result<()> {
		let old_w = self.windows.insert(channel_name.as_bytes().to_owned(), window);
		assert!(old_w.is_none(), "BUG: Re-opened same channel?");
		::std::io::Write::write_all(&mut conn, format!("JOIN {}\r\n", channel_name).as_bytes())?;
		Ok( () )
	}
	pub fn handle_line(&mut self, mut conn: &::std::net::TcpStream, mut line: &[u8]) -> ::std::io::Result<()> {
		::syscalls::kernel_log!("handle_line: {:?}", ::std::str::from_utf8(line).unwrap_or("?BADUTF?"));
		if line.starts_with(b":") {
			// A message (either from a user or from a server)

			line = &line[1..];
			let source = fetch_word(&mut line);
			let cmd = fetch_word(&mut line);

			if cmd.len() == 3 && cmd.iter().all(|v| v.is_ascii_digit()) {
				// Numeric commands
				let num = (cmd[0] - b'0') as u32 * 100
					+ (cmd[1] - b'0') as u32 * 10
					+ (cmd[2] - b'0') as u32 * 1
					;
				match num {
				332 => {	// RPL_TOPIC
					let channel = fetch_word(&mut line);
					let topic = fetch_word(&mut line);
					if let Some(w) = self.get_window(channel) {
						w.set_topic(topic);
					}
					else {
						self.status_window.print_error(&self.server_name, format_args!("No window for message"));
					}
				},
				_ => {},
				}
			}
			else {
				match cmd {
				b"PRIVMSG" => {
					let channel = fetch_word(&mut line);
					let message = fetch_word(&mut line);
					if let Some(w) = self.get_window(channel) {
						w.append_message(source, message);
					}
					else {
						// TODO: If this is not to a channel, then create a PM window
						self.status_window.orphaned_message(&self.server_name, channel, source, message);
					}
				},
				b"NOTICE" => {
					let channel = fetch_word(&mut line);
					let message = fetch_word(&mut line);
					if channel == b"*" {
						self.status_window.server_message(&self.server_name, source, message);
					}
					else if let Some(w) = self.get_window(channel) {
						w.append_message(source, message);
					}
					else {
						self.status_window.orphaned_message(&self.server_name, channel, source, message);
					}
				},
				_ => {
					self.status_window.print_error(&self.server_name, format_args!("Unknown message type: {:?}", String::from_utf8_lossy(cmd)));
				},
				}
			}
		}
		else {
			// A command
			let cmd = fetch_word(&mut line);
			match cmd {
			b"PING" => {
				},
			_ => {},
			}
		}

		// If this was the first message from the server, then send the initial login messages
		if ! ::core::mem::replace(&mut self.is_connected, true) {
			::std::io::Write::write_all(&mut conn, format!("USER {uname} {hname} {addr} :{realname}\r\n",
				uname="rust_os", hname="rust_os", addr=self.server_name, realname="Unspecified").as_bytes()
				)?;
			::std::io::Write::write_all(&mut conn, format!("NICK {nickname}\r\n",
				nickname="rust_os").as_bytes()
				)?;
		}
		Ok( () )
	}
}

fn fetch_word<'a>(line: &mut &'a [u8]) -> &'a [u8] {
	if line.is_empty() {
		b""
	}
	else if line[0] == b':' {
		let rv = &line[1..];
		*line = b"";
		rv
	}
	else {
		let l = line.iter().position(|v | *v == b' ').unwrap_or(line.len());
		let rv = &line[..l];
		*line = &line[l..];
		while line.starts_with(b" ") {
			*line = &line[1..];
		}
		rv
	}
}