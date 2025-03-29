use crate::window_types::{StatusWindow,ChannelWindow};


pub struct ServerState {
	server_name: String,
	status_window: StatusWindow,
	windows: ::std::collections::BTreeMap<Vec<u8>,ChannelWindow>,
}
impl ServerState {
	pub fn new(server_name: String, status_window: StatusWindow) -> Self {
		ServerState {
			server_name,
			status_window,
			windows: Default::default(),
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
	pub fn handle_line(&mut self, mut line: &[u8]) {
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
						self.status_window.print_error(&self.server_name, format_args!("No window for message"));
					}
				},
				_ => {

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