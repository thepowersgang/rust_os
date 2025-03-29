use crate::server_state::ServerState;

pub struct ServerManager {
	status_window: crate::window_types::StatusWindow,
	servers: Vec<ServerConnection>,
}
struct ServerConnection {
	stream: AsyncLineStream,
	protocol: ServerState,
}
impl ServerManager {
	pub fn new(status_window: crate::window_types::StatusWindow) -> Self {
		ServerManager {
			status_window,
			servers: Vec::new(),
		}
	}
	pub fn open_connection(&mut self, server_name: String, addr: ::std::net::IpAddr, port: u16) {
		self.servers.push(ServerConnection {
			stream: AsyncLineStream::new(::std::net::TcpStream::connect(addr, port).unwrap()),
			protocol: ServerState::new(server_name, self.status_window.clone()),
		});
	}
}
impl ::r#async::WaitController for ServerManager {
	fn get_count(&self) -> usize {
		self.servers.len()
	}

	fn populate(&self, cb: &mut dyn FnMut(syscalls::WaitItem)) {
		for s in &self.servers {
			cb(s.stream.wait_item())
		}
	}

	fn handle(&mut self, events: &[syscalls::WaitItem]) {
		for (s,e) in Iterator::zip(self.servers.iter_mut(), events.iter()) {
			match s.stream.handle(e) {
			Ok(None) => {},
			Ok(Some(line)) => s.protocol.handle_line(line),
			Err(_) => {},
			}
		}
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
		//self.connection.get_ref().wait_item()
		panic!("TODO: get WaitItem from std")
	}
	fn handle(&mut self, _item: &syscalls::WaitItem) -> ::std::io::Result<Option<&[u8]>> {
		use ::std::io::Read;
		match self.connection.read(&mut self.read_buffer[self.read_ofs..]) {
		Ok(len) => {
			let o = self.read_ofs;
			self.read_ofs += len;
			if let Some(newline_pos) = self.read_buffer[o..][..len].iter().position(|&v| v == b'\n') {
				Ok(Some(&self.read_buffer[..o+newline_pos]))
			}
			else {
				Ok(None)
			}
		},
		Err(e) => Err(e),
		}
	}
}