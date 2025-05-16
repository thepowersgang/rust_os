use crate::server_state::ServerState;

pub struct ServerManager<'a> {
	status_window: crate::window_types::StatusWindow,
	input: &'a crate::Input,
	tabs: crate::Tabs<'a>,
	// Mapping from tab index (minus 1, for the status window) to server channels
	windows: Vec<(usize, String)>,
	servers: Vec<ServerConnection>,
	selected_server: usize,
}
struct ServerConnection {
	stream: AsyncLineStream,
	protocol: ServerState,
	endpoint: (::std::net::IpAddr, u16),
}
impl<'a> ServerManager<'a> {
	pub fn new(status_window: crate::window_types::StatusWindow, input: &'a crate::Input, tabs: crate::Tabs<'a>) -> Self {
		ServerManager {
			status_window,
			input,
			tabs,
			selected_server: 0,
			windows: Vec::new(),
			servers: Vec::new(),
		}
	}
	pub fn open_connection(&mut self, server_name: String, addr: ::std::net::IpAddr, port: u16) -> ::std::io::Result<()> {
		self.servers.push(ServerConnection {
			stream: AsyncLineStream::new(::std::net::TcpStream::connect(addr, port)?),
			protocol: ServerState::new(server_name, self.status_window.clone()),
			endpoint: (addr, port),
		});
		Ok( () )
	}
}
impl<'a> ::r#async::WaitController for ServerManager<'a> {
	fn get_count(&self) -> usize {
		self.servers.len()
	}

	fn populate(&self, cb: &mut dyn FnMut(syscalls::WaitItem)) {
		for s in &self.servers {
			cb(s.stream.wait_item())
		}
	}

	fn handle(&mut self, events: &[syscalls::WaitItem]) {
		for (s,item) in Iterator::zip(self.servers.iter_mut(), events.iter()) {
			let proto = &mut s.protocol;
			match s.stream.handle(item, |conn,line| proto.handle_line(conn, line)) {
			Ok(()) => {},
			Err(e) => {
				::syscalls::kernel_log!("Connection error: {:?}", e);
				self.status_window.print_error(s.protocol.name(), format_args!("Connection error: {:?}", e));
				// TODO: Drop connection? Trigger a reconnect?
				// - Reconnect for now... but should have a longer timeout?
				s.stream = AsyncLineStream::new(match ::std::net::TcpStream::connect(s.endpoint.0, s.endpoint.1)
					{
					Ok(v) => v,
					Err(e) => {
						::syscalls::kernel_log!("Re-connection error: {:?}", e);
						continue
					},
					});
			},
			}
		}
		if let Some(text) = self.input.take() {
			::syscalls::kernel_log!("Input {:?}", text);
			// Get the current tab
			let cur_tab = self.tabs.selected_idx();
			// Map to a server and channel
			let win = if cur_tab == 0 {
				None
			} else {
				Some(&self.windows[cur_tab-1])
			};

			if text.starts_with("/") {
				match text.split_once(" ").unwrap_or((&text,"")) {
				("/quit", _message) => {},
				("/leave", _message) => {},
				("/join", channel) => match self.command_join(channel)
					{
					Ok(()) => {},
					Err(e) => self.status_window.print_error("<input>", format_args!("/join: {}", e))
					},
				("/server", des) => {
					if des.trim() == "" {
						// Dump the current server list to the input
					}
					else {
						// If `des` parses to an integer, treat as an index
						// otherwise, search the sever names
					}
				}
				("/nick", _nickname) => {},
				("/connect",args) => match self.command_connect(args)
					{
					Ok(()) => {},
					Err(e) => self.status_window.print_error("<input>", format_args!("/connect: {}", e))
					},
				(cmd,_) => self.status_window.print_error("<input>", format_args!("Unknown command {:?}", cmd)),
				}
			}
			else {
				// Deliver message to server, and render locally
				if let Some((server_idx,channel_name)) = win {
					let s = &mut self.servers[*server_idx];
					match s.protocol.send_message( s.stream.connection.get_ref(), channel_name, &text) {
					Ok(()) => {},
					Err(e) => self.status_window.print_error(s.protocol.name(), format_args!("Network Error: {:?}", e)),
					}
				}
				else {
					// Just ignore
				}
			}
		}

		// TODO: Trigger a window redraw
	}
}
impl<'a> ServerManager<'a> {
	fn command_connect(&mut self, args: &str) -> Result<(),String> {
		let mut addr = None;
		let mut port = None;
		let mut name = None;
		let mut args = args.split(" ");
		while let Some(a) = args.next() {
			if a.starts_with("-") {
				match a {
				"-name" => name = args.next(),
				_ => {},
				}
			}
			else {
				if addr.is_none() {
					addr = Some(a);
				}
				else if port.is_none() {
					port = Some(a);
				}
				else {
				}
			}
		}
		let Some(addr) = addr else {
			return Err("Requires an address".to_owned());
		};
		let server_name = name.unwrap_or(addr);
		let addr = match addr.parse()
			{
			Ok(v) => v,
			Err(e) => return Err(format!("Malformed address: {:?}", e)),
			};
		let port: u16 = match port.unwrap_or("6667").parse()
			{
			Ok(v) => v,
			Err(e) => return Err(format!("Malformed port: {:?}", e)),
			};
		match self.open_connection(server_name.to_owned(), addr, port)
		{
		Ok(()) => {},
		Err(e) => return Err(format!("Unable to connect to server: {:?}", e)),
		}
		Ok( () )
	}

	fn command_join(&mut self, channel: &str) -> Result<(),String>
	{
		// Create a window, then tell the client logic to issue a join command
		// - But... which server?
		let cur_tab = self.tabs.selected_idx();
		let server_idx = if cur_tab != 0 {
				self.windows[cur_tab-1].0
			}
			else {
				self.selected_server
			};
		if self.windows.iter().any(|e| e.0 == server_idx && e.1 == channel) {
			return Err("Already joined?".to_owned())
		}
		let server = &mut self.servers[server_idx];
		// Add the tab to the tab bar
		let window = self.tabs.add_tab(server.protocol.name(), channel);
		// Add to the mapping between tabs and server-channel
		self.windows.push((server_idx, channel.to_owned()));
		// And get the protocol to issue a `JOIN` request
		match server.protocol.join_channel(server.stream.connection.get_ref(), channel.trim(), window) {
		Ok(()) => {},
		Err(e) => return Err(format!("Network error: {:?}", e)),
		}
		Ok( () )
	}
}

struct AsyncLineStream {
	connection: ::std::io::BufReader<::std::net::TcpStream>,
	read_ofs: usize,
	read_buffer: Vec<u8>,
}

impl AsyncLineStream {
	fn new(inner: ::std::net::TcpStream) -> Self {
		AsyncLineStream {
			connection: ::std::io::BufReader::new(inner),
			read_ofs: 0,
			read_buffer: vec![0; 1024],
			}
	}
	fn wait_item(&self) -> syscalls::WaitItem {
		use ::std::net::TcpStreamExt;
		self.connection.get_ref().wait_item()
	}
	fn handle<F>(&mut self, _item: &syscalls::WaitItem, mut cb: F) -> ::std::io::Result<()>
	where
		F: FnMut(&::std::net::TcpStream, &[u8]) -> ::std::io::Result<()>
	{
		use ::std::io::Read;
		::syscalls::kernel_log!("AsyncLineStream: handle");
		let len = self.connection.read(&mut self.read_buffer[self.read_ofs..])?;
		::syscalls::kernel_log!("AsyncLineStream::handle: len={}", len);
		let o = self.read_ofs;
		self.read_ofs += len;

		if let Some(newline_pos) = self.read_buffer[o..][..len].iter().position(|&v| v == b'\n') {
			cb( self.connection.get_ref(), &self.read_buffer[..o+newline_pos] )?;
			let mut len = len - (newline_pos+1);
			let mut o = o + (newline_pos+1);
			// Handle multiple lines
			while let Some(newline_pos) = self.read_buffer[o..][..len].iter().position(|&v| v == b'\n') {
				cb( self.connection.get_ref(), &self.read_buffer[o..o+newline_pos] )?;
				len = len - (newline_pos+1);
				o = o + (newline_pos+1);
			}
			// Once done, remove the consumed data
			self.read_buffer.copy_within(o .. o+len, 0);
			self.read_ofs = len;
		}
		
		Ok(())
	}
}