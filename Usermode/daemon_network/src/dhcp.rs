// https://www.ietf.org/rfc/rfc2131.txt

use std::convert::TryInto;

pub struct Dhcp {
	socket: ::syscalls::net::FreeSocket,
	mac_addr: [u8; 6],
	state: State,
}
enum State {
	RequestSent {
		start_time: u64,
		resend_time: u64,
		transaction_id: u32,
	},
	Configured {
		update_time: u64,
		addr: [u8; 4],
	},
}
impl Dhcp
{
	pub fn new(addr: &::syscalls::values::NetworkAddress, mac_addr: &[u8; 6]) -> Result<Dhcp,()>
	{
		let start_time = ::syscalls::system_ticks();
		let local = ::syscalls::net::SocketAddress {
			port_ty: ::syscalls::values::SocketPortType::Udp as _,
			addr_ty: addr.addr_ty,
			port: 68,	// DHCP Client port
			addr: addr.addr,
		};
		let remote = ::syscalls::net::MaskedSocketAddress {
			addr: ::syscalls::net::SocketAddress {
				port_ty: ::syscalls::values::SocketPortType::Udp as _,
				addr_ty: addr.addr_ty,
				port: 67,	// DHCP
				addr: [0; 16],
			},
			mask: 0,
		};
		let mut s = match ::syscalls::net::FreeSocket::create(local, remote)
			{
			Ok(s) => s,
			Err(e) => {
				::syscalls::kernel_log!("Error creating DHCP socket: {:?}", e);
				return Err(());
			},
			};
		let transaction_id = 1234567;
		let mut buf = DhcpPacket::empty_buf();
		let dhcp_request_pkt = DhcpPacket {
			op: 1,
			transaction_id,
			seconds_since_start: 0,
			flags: 0,
			ciaddr: [0; 4],
			yiaddr: addr.addr[..4].try_into().unwrap(),
			siaddr: [0; 4],
			giaddr: [0; 4],
			mac_addr,
			server_name: b"",
			// TODO: Generate options - DHCP uses bootp's packet format, but has a magic cookie at the start of the options
			// TODO: Find a better format for options
			options: &[
				0x63, 0x82, 0x53, 0x63,
				// TODO: Hostname: option 12
			],
		}.to_bytes(&mut buf);
		match s.send_to(&dhcp_request_pkt, ::syscalls::net::SocketAddress {
			port_ty: ::syscalls::values::SocketPortType::Udp as _,
			addr_ty: addr.addr_ty,
			port: 67,	// DHCP
			addr: [0xFF; 16],	// Wildcard address, only the first 4 bytes actually matter
		}) {
		Ok(_) => {},
		Err(e) => {
			::syscalls::kernel_log!("Error sending DHCP request: {:?}", e);
			return Err(());
		},
		}

		Ok(Dhcp {
			socket: s,
			mac_addr: *mac_addr,
			state: State::RequestSent { start_time, transaction_id, resend_time: ::syscalls::system_ticks() + 5*1000 }
		})
	}

	pub fn get_wait(&self) -> Option<::syscalls::WaitItem> {
		None
	}

	// TODO: Options, e.g. routes.
	pub fn poll(&mut self, mgr: &::syscalls::net::Management, iface_idx: usize) {
		match &mut self.state {
		State::RequestSent { start_time, transaction_id, resend_time } => {
			let mut packet_data = DhcpPacket::empty_buf();
			match self.socket.recv_from(&mut packet_data)
			{
			Ok((len, remote)) => {
				let packet_data = &packet_data[..len];
				let packet = DhcpPacket::from_bytes(packet_data);
				if packet.op == 2 && packet.transaction_id == *transaction_id {
					let a = packet.yiaddr;
					// Iterate options, and set up the rest of the state
					let mut subnet_len = 24;
					for opt in iter_options(packet.options) {
						match opt {
						Opt::Unknown(_op, _data) => {},
						Opt::Malformed(_op, _data) => {},
						Opt::SubnetMask(m) => {
							let m = u32::from_le_bytes(m);
							subnet_len = m.leading_ones() as u8;
						},
						}
					}
					let addr = super::make_ipv4(a[0], a[1], a[2], a[3]);
					mgr.add_address(iface_idx, addr, subnet_len);
					self.state = State::Configured { update_time: ::syscalls::system_ticks() + 60*1000, addr: packet.yiaddr  };

					let local = ::syscalls::net::SocketAddress {
						port_ty: ::syscalls::values::SocketPortType::Udp as _,
						addr_ty: addr.addr_ty,
						port: 68,	// DHCP Client port
						addr: addr.addr,
					};
					let remote = ::syscalls::net::MaskedSocketAddress {
						addr: remote,
						mask: 32,
					};
					match ::syscalls::net::FreeSocket::create(local, remote)
					{
					Ok(s) => self.socket = s,
					Err(e) => {
						::syscalls::kernel_log!("Error creating new DHCP socket: {:?}", e);
					},
					}
					return ;
				}
				else {
				}
			},
			Err(::syscalls::net::Error::NoData) => {},
			Err(_) => {},
			}
			if *resend_time < ::syscalls::system_ticks() {
				// Re-send request
				let mut buf = DhcpPacket::empty_buf();
				let dhcp_request_pkt = DhcpPacket {
					op: 1,
					transaction_id: *transaction_id,
					seconds_since_start: 0,
					flags: 0,
					ciaddr: [0; 4],
					yiaddr: [0; 4],	//addr.addr[..4].try_into().unwrap(),
					siaddr: [0; 4],
					giaddr: [0; 4],
					mac_addr: &self.mac_addr,
					server_name: b"",
					// TODO: Generate options - DHCP uses bootp's packet format, but has a magic cookie at the start of the options
					// TODO: Find a better format for options
					options: &[
						0x63, 0x82, 0x53, 0x63,
						// TODO: Hostname: option 12
					],
				}.to_bytes(&mut buf);
				match self.socket.send_to(&dhcp_request_pkt, ::syscalls::net::SocketAddress {
					port_ty: ::syscalls::values::SocketPortType::Udp as _,
					addr_ty: ::syscalls::values::SocketAddressType::Ipv4 as _,
					port: 67,	// DHCP
					addr: [0xFF; 16],	// Wildcard address, only the first 4 bytes actually matter
				}) {
				Ok(_) => {},
				Err(e) => {
					::syscalls::kernel_log!("Error sending DHCP request: {:?}", e);
				},
				}
				*resend_time += 10_000;
			}
		}
		State::Configured { update_time, addr } => {
			if *update_time < ::syscalls::system_ticks() {
				// TODO: Send a refresh request
			}
		}
		}
	}
}

#[derive(Debug)]
struct DhcpPacket<'a> {
	op: u8,
	transaction_id: u32,
	seconds_since_start: u16,
	flags: u16,
	/// Client address - only populated when renewing
	ciaddr: [u8; 4],
	/// New IPv4 address for the client to use
	yiaddr: [u8; 4],
	/// Server address
	siaddr: [u8; 4],
	/// Relay address
	giaddr: [u8; 4],
	mac_addr: &'a [u8],	// up to 16
	server_name: &'a [u8],	// up to 128
	options: &'a [u8],
}
type PktBuf = [u8; 576];
impl<'a> DhcpPacket<'a> {
	fn empty_buf() -> PktBuf {
		[0; 576]
	}
	fn from_bytes(mut pkt: &'a [u8]) -> Self {
		Self {
			op: get::<4>(&mut pkt)[0],
			transaction_id: u32::from_be_bytes(*get(&mut pkt)),
			seconds_since_start: u16::from_be_bytes(*get(&mut pkt)),
			flags: u16::from_be_bytes(*get(&mut pkt)),
			ciaddr: *get(&mut pkt),
			yiaddr: *get(&mut pkt),
			siaddr: *get(&mut pkt),
			giaddr: *get(&mut pkt),
			mac_addr: &get::<10>(&mut pkt)[..6],
			server_name: &get::<128>(&mut pkt)[..],
			options: pkt,
		}
	}
	fn to_bytes(self, pkt: &mut PktBuf) -> &[u8] {
		let mut pos = 0;
		let mut push_bytes = |data: &[u8]| {
			pkt[pos..][..data.len()].copy_from_slice(data);
			pos += data.len();
		};
		// op = REQUEST
		// HW Address type
		// HW Address length
		// Hop count, set to zero to start with
		push_bytes(&[self.op, 1, 6, 0]);
		push_bytes(&self.transaction_id.to_be_bytes());	// u32 Transaction ID
		push_bytes(&self.seconds_since_start.to_be_bytes());	// u16: Seconds since start of process
		push_bytes(&self.flags.to_be_bytes());	// u16: flags
		push_bytes(&self.ciaddr);	// [u8; 4] ciaddr
		push_bytes(&self.yiaddr);	// [u8; 4] yiaddru32
		push_bytes(&self.siaddr);	// [u8; 4] siaddr
		push_bytes(&self.giaddr);	// [u8; 4] giaddr
		push_bytes(self.mac_addr);
		push_bytes(&[0; 16][self.mac_addr.len()..]);	// [u8; 16] chaddr
		push_bytes(self.server_name);
		push_bytes(&[0; 128][self.server_name.len()..]);	// [u8; 128] sname
		push_bytes(self.options);
		push_bytes(&[0; 312][self.server_name.len()..]);	// [u8; 312...] options
		&pkt[..pos]
	}
}

fn get<'a, const N: usize>(src: &mut &'a [u8]) -> &'a [u8; N] {
	let v = src.split_at(N);
	*src = v.1;
	v.0.try_into().unwrap()
}

enum Opt<'a> {
	Malformed(u8, &'a [u8]),
	Unknown(u8, &'a [u8]),
	SubnetMask([u8; 4]),
}
fn iter_options<'a>(opts: &'a [u8]) -> impl Iterator<Item=Opt<'a>> + 'a {
	return if opts.starts_with(b"") {
		Iter(b"")
	}
	else {
		Iter(&opts[4..])
	};
	struct Iter<'a>(&'a [u8]);
	impl<'a> Iterator for Iter<'a> {
		type Item = Opt<'a>;
		fn next(&mut self) -> Option<Self::Item> {
			match self.0 {
			[] => None,
			[0, tail @ ..] => {
				self.0 = tail;
				self.next()
			},
			[_] => {
				self.0 = &[];
				None
			},
			&[code, len, ref tail @ ..] => {
				let Some( (data,tail) ) = tail.split_at_checked(len as usize) else {
					self.0 = &[];
					return None;
				};
				self.0 = tail;
				Some(match code {
					1 => match data {
						&[a,b,c,d,] => Opt::SubnetMask([a,b,c,d,]),
						_ => Opt::Malformed(code, data),
						},
					_ => Opt::Unknown(code, data)
					})
			}
			}
		}
	}
}