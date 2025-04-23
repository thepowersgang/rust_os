// https://www.ietf.org/rfc/rfc2131.txt

use std::convert::TryInto;

const UDP_PORT_DHCP_CLIENT: u16 = 68;
const UDP_PORT_DHCP_SERVER: u16 = 67;

mod options;
use self::options::{Opt,OptionsIter};

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
		last_update_time: u64,
		my_addr: [u8; 4],
		server_addr: [u8; 4],
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
			port: UDP_PORT_DHCP_CLIENT,	// DHCP Client port
			addr: addr.addr,
		};
		let remote = ::syscalls::net::MaskedSocketAddress {
			addr: ::syscalls::net::SocketAddress {
				port_ty: ::syscalls::values::SocketPortType::Udp as _,
				addr_ty: addr.addr_ty,
				port: UDP_PORT_DHCP_SERVER,	// DHCP
				addr: [0; 16],
			},
			mask: 0,
		};
		// TODO: Since the interface will have an address of `0.0.0.0` (maybe?) need to specify the interface number
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
			boot_file: b"",
			options: PacketOptions::Decoded(&[
				Opt::DhcpMessageType(1),	// Discover
				Opt::ClientIdentifier(mac_addr),
				Opt::ParameterRequestList(&[options::codes::Routers])
				// TODO: Hostname?
			]),
		}.to_bytes(&mut buf);
		// TODO: How to ensure that this sends out the right interface?
		// - Raw IP?
		// - Socket option to restrict local?
		match s.send_to(dhcp_request_pkt, ::syscalls::net::SocketAddress {
			port_ty: ::syscalls::values::SocketPortType::Udp as _,
			addr_ty: addr.addr_ty,
			port: UDP_PORT_DHCP_SERVER,
			addr: [0xFF; 16],	// Wildcard address, only the first 4 bytes actually matter
		}) {
		Ok(_) => {},
		Err(e) => {
			::syscalls::kernel_log!("Error sending DHCP request: {:?}", e);
			return Err(());
		},
		}

		::syscalls::kernel_log!("DHCP started");
		Ok(Dhcp {
			socket: s,
			mac_addr: *mac_addr,
			state: State::RequestSent { start_time, transaction_id, resend_time: ::syscalls::system_ticks() + 5*1000 }
		})
	}

	pub fn get_wait(&self) -> Option<::syscalls::WaitItem> {
		Some(self.socket.wait_read())
	}

	pub fn poll(&mut self, mgr: &::syscalls::net::Management, iface_idx: usize) {
		::syscalls::kernel_log!("dhcp: poll");
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
					let PacketOptions::Encoded(options) = packet.options else { panic!(); };
					for opt in options {
						match opt {
						Opt::Unknown(_op, _data) => {},
						Opt::Malformed(_op, _data) => {},
						Opt::SubnetMask(m) => {
							let m = u32::from_le_bytes(m);
							subnet_len = m.leading_ones() as u8;
						},
						_ => {},
						}
					}
					let addr = super::make_ipv4(a[0], a[1], a[2], a[3]);
					mgr.add_address(iface_idx, addr, subnet_len);
					self.state = State::Configured {
						last_update_time: ::syscalls::system_ticks(),
						my_addr: packet.yiaddr,
						server_addr: remote.addr[..4].try_into().unwrap(),
					};

					let local = ::syscalls::net::SocketAddress {
						port_ty: ::syscalls::values::SocketPortType::Udp as _,
						addr_ty: addr.addr_ty,
						port: UDP_PORT_DHCP_CLIENT,
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
					seconds_since_start: ((::syscalls::system_ticks() - *start_time) / 1000).try_into().unwrap_or(!0),
					flags: 0,
					ciaddr: [0; 4],
					yiaddr: [0; 4],	//addr.addr[..4].try_into().unwrap(),
					siaddr: [0; 4],
					giaddr: [0; 4],
					mac_addr: &self.mac_addr,
					server_name: b"",
					boot_file: b"",
					options: PacketOptions::Decoded(&[
						// TODO: Hostname: option 12
					]),
				}.to_bytes(&mut buf);
				match self.socket.send_to(dhcp_request_pkt, ::syscalls::net::SocketAddress {
					port_ty: ::syscalls::values::SocketPortType::Udp as _,
					addr_ty: ::syscalls::values::SocketAddressType::Ipv4 as _,
					port: UDP_PORT_DHCP_SERVER,
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
		State::Configured { last_update_time, my_addr, server_addr } => {
			if *last_update_time + 60_1000 < ::syscalls::system_ticks() {
				// TODO: Send a refresh request
				let mut buf = DhcpPacket::empty_buf();
				let dhcp_request_pkt = DhcpPacket {
					op: 1,
					transaction_id: 0,
					seconds_since_start: ((::syscalls::system_ticks() - *last_update_time) / 1000).try_into().unwrap_or(!0),
					flags: 0,
					ciaddr: [0; 4],
					yiaddr: *my_addr,
					siaddr: *server_addr,
					giaddr: [0; 4],
					mac_addr: &self.mac_addr,
					server_name: b"",
					boot_file: b"",
					options: PacketOptions::Decoded(&[
						// TODO: Hostname: option 12
					]),
				}.to_bytes(&mut buf);
				match self.socket.send_to(dhcp_request_pkt, ::syscalls::net::SocketAddress {
					port_ty: ::syscalls::values::SocketPortType::Udp as _,
					addr_ty: ::syscalls::values::SocketAddressType::Ipv4 as _,
					port: UDP_PORT_DHCP_SERVER,
					addr: [server_addr[0], server_addr[1],server_addr[2],server_addr[3], 0,0,0,0, 0,0,0,0,0,0,0,0],	// Wildcard address, only the first 4 bytes actually matter
				}) {
				Ok(_) => {},
				Err(e) => {
					::syscalls::kernel_log!("Error sending DHCP request: {:?}", e);
				},
				}
			}
		}
		}
	}
}

#[derive(Debug)]
enum PacketOptions<'a> {
	Decoded(&'a [Opt<'a>]),
	Encoded(OptionsIter<'a>),
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
	server_name: &'a [u8],	// up to 64
	boot_file: &'a [u8],	// up to 128
	options: PacketOptions<'a>,
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
			server_name: &get::<64>(&mut pkt)[..],
			boot_file: &get::<128>(&mut pkt)[..],
			options: PacketOptions::Encoded(OptionsIter(pkt)),
		}
	}
	fn to_bytes(self, pkt: &mut PktBuf) -> &[u8] {
		struct P<'a>(&'a mut PktBuf, usize);
		impl<'a> P<'a> {
			fn push(&mut self, data: &[u8]) {
				self.0[self.1..][..data.len()].copy_from_slice(data);
				self.1 += data.len();
			}
			fn push_pad<const N: usize>(&mut self, data: &[u8]) {
				assert!(data.len() <= N);
				self.push(data);
				self.push(&[0; N][data.len()..]);
			}
		}
		let mut p = P(pkt, 0);
		// HW Address type = 1 for Ethernet
		// HW Address length = 6 for Ethernet
		// Hop count, set to zero
		p.push(&[self.op, 1, 6, 0]);
		p.push(&self.transaction_id.to_be_bytes());	// u32 Transaction ID
		p.push(&self.seconds_since_start.to_be_bytes());	// u16: Seconds since start of process
		p.push(&self.flags.to_be_bytes());	// u16: flags
		p.push(&self.ciaddr);	// [u8; 4] ciaddr
		p.push(&self.yiaddr);	// [u8; 4] yiaddru32
		p.push(&self.siaddr);	// [u8; 4] siaddr
		p.push(&self.giaddr);	// [u8; 4] giaddr
		p.push_pad::<16>(self.mac_addr);	// [u8; 16] mac addresss
		p.push_pad::<64>(self.server_name);// [u8; 64] server name
		p.push_pad::<128>(self.boot_file);	// [u8; 128] file
		let o = p.1;
		p.push(&[0x63, 0x82, 0x53, 0x63]);
		match self.options {
		PacketOptions::Decoded(opts) => {
			for o in opts {
				o.encode(|op, data| {
					p.push(&[op, data.len() as u8]);
					p.push(data)
				})
			}
			p.push(&[255]);	// End
		},
		PacketOptions::Encoded(options_iter) => {
			p.push(options_iter.0);
		},
		}
		let options_len = p.1 - o;
		if let Some(data) = [0; 312].get(options_len..) {
			p.push(data);	// [u8; 312...] options
		}
		let len = p.1;
		&pkt[..len]
	}
}

fn get<'a, const N: usize>(src: &mut &'a [u8]) -> &'a [u8; N] {
	let v = src.split_at(N);
	*src = v.1;
	v.0.try_into().unwrap()
}
