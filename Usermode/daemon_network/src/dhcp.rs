// https://www.ietf.org/rfc/rfc2131.txt

use std::convert::TryInto;
use core::net::Ipv4Addr;

const UDP_PORT_DHCP_CLIENT: u16 = 68;
const UDP_PORT_DHCP_SERVER: u16 = 67;

mod options;
use self::options::{Opt,OptionsIter};

const MAGIC_COOKIE: [u8; 4] = [0x63, 0x82, 0x53, 0x63];

pub struct Dhcp {
	socket: ::syscalls::net::FreeSocket,
	mac_addr: [u8; 6],
	state: State,
}
enum State {
	DiscoverSent {
		start_time: u64,
		resend_time: u64,
		transaction_id: u32,
		link_local_addr: syscalls::values::NetworkAddress,
	},
	Configured {
		/// Time of the last recieved DHCPACK
		last_update_time: u64,
		/// Time for the next DHCPREQUEST to be sent
		next_update_time: u64,
		/// Our current IP address
		my_addr: [u8; 4],
		/// DHCP server address, used for transmit
		server_addr: [u8; 4],
	},
}
/// Locates the first item in an iterator that matches a pattern
macro_rules! find_match {
	($expr:expr, $pat:pat => $val:expr) => {
		($expr).filter_map(|v| match v { $pat => Some($val), _ => None }).next()
	}
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
		let socket = match ::syscalls::net::FreeSocket::create(local, remote)
			{
			Ok(s) => s,
			Err(e) => {
				::syscalls::kernel_log!("Error creating DHCP socket: {:?}", e);
				return Err(());
			},
			};
		let transaction_id = 1234567;
		let rv = Dhcp {
			socket,
			mac_addr: *mac_addr,
			state: State::DiscoverSent {
				start_time, transaction_id, resend_time: ::syscalls::system_ticks() + 5*1000,
				link_local_addr: *addr,
			}
		};
		rv.send_discover(transaction_id, start_time)?;

		::syscalls::kernel_log!("DHCP started");
		Ok(rv)
	}

	fn send_packet(&self, yiaddr: [u8; 4], siaddr: [u8; 4], transaction_id: u32, start_time: u64, options: &[Opt]) -> Result<(),::syscalls::values::SocketError>
	{
		let mut buf = DhcpPacket::empty_buf();
		let dhcp_request_pkt = DhcpPacket {
			op: BOOTREQUEST,
			transaction_id,
			seconds_since_start: ((::syscalls::system_ticks() - start_time) / 1000).try_into().unwrap_or(!0),
			flags: 0,
			ciaddr: [0; 4],
			yiaddr,
			siaddr,
			giaddr: Ipv4Addr::new(0,0,0,0),
			mac_addr: &self.mac_addr,
			server_name: b"",
			boot_file: b"",
			options: PacketOptions::Decoded(options),
		}.to_bytes(&mut buf);
		// TODO: How to ensure that this sends out the right interface?
		// - Raw IP?
		// - Socket option to restrict local?
		self.socket.send_to(dhcp_request_pkt, ::syscalls::net::SocketAddress {
			port_ty: ::syscalls::values::SocketPortType::Udp as _,
			addr_ty: ::syscalls::values::SocketAddressType::Ipv4 as _,
			port: UDP_PORT_DHCP_SERVER,
			addr: if siaddr == [0; 4] {
				[0xFF; 16]	// Wildcard address, only the first 4 bytes actually matter
			}
			else {
				super::make_v4a(siaddr)
			},
		}).map(|_| ())
	}
	fn send_discover(&self, transaction_id: u32, start_time: u64) -> Result<(),()> {
		match self.send_packet([0; 4], [0; 4], transaction_id, start_time, &[
				Opt::DhcpMessageType(MessageType::Discover as u8),	// Discover
				Opt::ClientIdentifier(&self.mac_addr),
				Opt::ParameterRequestList(&[options::codes::Routers])
				// TODO: Hostname?
			]) {
		Ok(()) => Ok(()),
		Err(e) => {
			::syscalls::kernel_log!("Failed to send DHCP Discovery request: {:?}", e);
			return Err(());
		}
		}
	}

	pub fn get_wait(&self) -> Option<::syscalls::WaitItem> {
		Some(self.socket.wait_read())
	}

	pub fn poll(&mut self, mgr: &::syscalls::net::Management, iface_idx: usize) {
		::syscalls::kernel_log!("dhcp: poll");
		match &mut self.state {
		State::DiscoverSent { start_time, transaction_id, resend_time, link_local_addr } => {
			let mut packet_data = DhcpPacket::empty_buf();
			while let Some((remote, packet)) = opt_rx_packet(&self.socket, &mut packet_data)
			{
				if packet.op == BOOTREPLY && packet.transaction_id == *transaction_id {
					let PacketOptions::Encoded(ref options) = packet.options else { panic!(); };
					::syscalls::kernel_log!("DHCP Packet: {:?}", options);
					match find_match!(options.clone(), Opt::DhcpMessageType(t) => t) {
					Some(v) if v == MessageType::Offer as u8 => {
						// We've been offered an address, formally request it from the server
						let start_time = *start_time;
						::syscalls::kernel_log!("DHCP Offer: {}", Ipv4Addr::from_octets(packet.yiaddr));
						match self.send_packet(packet.yiaddr, packet.siaddr, packet.transaction_id, start_time, &[
							Opt::DhcpMessageType(MessageType::Request as u8),
							Opt::ServerIdentifier(packet.siaddr),
							Opt::RequestedIpAddress(packet.yiaddr),
						]) {
						Ok(()) => {},
						Err(e) => {
							::syscalls::kernel_log!("Error csending DHCP packet: {:?}", e);
						}
						}
						return ;
					}
					Some(v) if v == MessageType::Ack as u8 => {
						let my_addr = packet.yiaddr;

						// Get the subnet mask and convert to a prefix length (required for `add_address`)
						let subnet_len = find_match!(options.clone(), Opt::SubnetMask(m) => m)
							.map(|m| u32::from_be_bytes(m).leading_ones() as u8)
							.unwrap_or(24)
							;
						// Remove the temporary link-local address
						mgr.del_address(iface_idx, *link_local_addr, 0);
						
						let addr = super::make_ipv4(my_addr);
						mgr.add_address(iface_idx, addr, subnet_len);

						// Fully enumerate options to get non-IP settings
						for opt in options.clone() {
							match opt {
							Opt::Routers(routers) => {
								let [router,..] = routers else { panic!("Protocol violation, must be at least one router") };
								mgr.add_route(syscalls::values::NetworkRoute {
									network: [0; 16], mask: 0,
									gateway: super::make_v4a(*router),
									addr_ty: syscalls::values::SocketAddressType::Ipv4 as u8,
									});
							},
							Opt::NameServersDns(name_servers) => {
								for _ns in name_servers {
									// TODO: Configure the local DNS server with these as upstream
								}
							},
							Opt::TimeServers(servers) => {
								for _ip in servers {
									// TODO: Configure NTP client
								}
							},
							Opt::DomainName(_dns_suffix) => {
								// TODO: Configure local DNS server
							},
							_ => {},
							}
						}

						// Update the state
						self.state = State::Configured {
							last_update_time: ::syscalls::system_ticks(),
							next_update_time: ::syscalls::system_ticks() + 60_0000,
							my_addr,
							server_addr: remote.addr[..4].try_into().unwrap(),
						};

						// Re-create the socket using the new local address
						let local = ::syscalls::net::SocketAddress {
							port_ty: ::syscalls::values::SocketPortType::Udp as _,
							addr_ty: ::syscalls::values::SocketAddressType::Ipv4 as _,
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

					},
					ty => ::syscalls::kernel_log!("DHCP: DiscoverSent: Unexpected message ty: {:?}", ty),	// Invalid message type
					}
				}
				else {
					::syscalls::kernel_log!("DHCP: DiscoverSent: Unexpected packet, op={:02x}, transaction_id={}", packet.op, packet.transaction_id);
				}
			}
			if *resend_time < ::syscalls::system_ticks() {
				*resend_time += 10_000;
				let start_time = *start_time;
				let transaction_id = *transaction_id;
				// NOTE: Error is already handled (logged) in this call
				let _ = self.send_discover(transaction_id, start_time);
			}
		}
		State::Configured { last_update_time, next_update_time, my_addr, server_addr } => {
			let mut packet_data = DhcpPacket::empty_buf();
			while let Some((_remote, packet)) = opt_rx_packet(&self.socket, &mut packet_data)
			{
				let PacketOptions::Encoded(options) = packet.options else { panic!(); };

				// Look for an ACK to a previous request
				if packet.op == BOOTREPLY && packet.transaction_id == 0 {
					match find_match!(options.clone(), Opt::DhcpMessageType(t) => t) {
					Some(v) if v == MessageType::Ack as u8 => {
						// Validate that it's an ACK for our current address, and then update the next update time
						// based on the lease duration.
					}
					_ => {},
					}
				}
			}
			if *next_update_time < ::syscalls::system_ticks() {
				let yiaddr = *my_addr;
				let siaddr = *server_addr;
				let start_time = *last_update_time;
				*last_update_time = ::syscalls::system_ticks();
				*next_update_time = ::syscalls::system_ticks() + 60_000;
				// Send a refresh request, i.e. just re-request the same IP
				match self.send_packet(yiaddr, siaddr, 0, start_time, &[
					Opt::DhcpMessageType(MessageType::Request as _)
				]) {
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

fn opt_rx_packet<'a>(socket: &::syscalls::net::FreeSocket, packet_data: &'a mut [u8]) -> Option<(::syscalls::values::SocketAddress, DhcpPacket<'a>)>
{
	match socket.recv_from(packet_data)
	{
	Err(::syscalls::net::Error::NoData) => None,
	Err(e) => {
		::syscalls::kernel_log!("dhcp: Error reciving packet {:?}", e);
		None
	},
	Ok((len, remote)) => {
		let packet_data = &packet_data[..len];
		let packet = DhcpPacket::from_bytes(packet_data);
		::syscalls::kernel_log!("DHCP Rx: {:?}", packet);
		Some( (remote, packet, ) )
	},
	}
}

const BOOTREQUEST: u8 = 1;
const BOOTREPLY: u8 = 2;

#[repr(C)]
#[allow(dead_code)]
enum MessageType {
	/// DHCPDISCOVER
	Discover = 1,
	/// DHCPOFFER
	Offer = 2,
	/// DHCPREQUEST
	Request = 3,
	/// DHCPDECLINE
	Decline = 4,
	/// DHCPACK
	Ack = 5,
	/// DHCPNAK
	Nak = 6,
	/// DHCPRELEASE
	Release = 7,
	/// DHCPINFORM
	Inform = 8,
}

#[derive(Debug)]
enum PacketOptions<'a> {
	Decoded(&'a [Opt<'a>]),
	Encoded(OptionsIter<'a>),
	NotDhcp(&'a [u8]),
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
	giaddr: ::core::net::Ipv4Addr,
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
		fn trim_nul(mut v: &[u8])->&[u8] {
			while v.last() == Some(&0) {
				v = &v[..v.len()-1];
			}
			v
		}
		Self {
			op: get::<4>(&mut pkt)[0],
			transaction_id: u32::from_be_bytes(*get(&mut pkt)),
			seconds_since_start: u16::from_be_bytes(*get(&mut pkt)),
			flags: u16::from_be_bytes(*get(&mut pkt)),
			ciaddr: *get(&mut pkt),
			yiaddr: *get(&mut pkt),
			siaddr: *get(&mut pkt),
			giaddr: ::core::net::Ipv4Addr::from_octets(*get(&mut pkt)),
			mac_addr: &get::<16>(&mut pkt)[..6],	// HACK, assume ethernet, so 6 bytes
			server_name: trim_nul(&get::<64>(&mut pkt)[..]),
			boot_file: trim_nul(&get::<128>(&mut pkt)[..]),
			options: if pkt.starts_with(&MAGIC_COOKIE) {
				PacketOptions::Encoded(OptionsIter(&pkt[4..]))
			} else {
				PacketOptions::NotDhcp(pkt)
			},
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
		p.push(&self.giaddr.octets());	// [u8; 4] giaddr
		p.push_pad::<16>(self.mac_addr);	// [u8; 16] mac addresss
		p.push_pad::<64>(self.server_name);// [u8; 64] server name
		p.push_pad::<128>(self.boot_file);	// [u8; 128] file
		let o = p.1;
		match self.options {
		PacketOptions::Decoded(opts) => {
			p.push(&MAGIC_COOKIE);
			for o in opts {
				o.encode(|op, data| {
					p.push(&[op, data.len() as u8]);
					p.push(data)
				})
			}
			p.push(&[255]);	// End
		},
		PacketOptions::Encoded(options_iter) => {
			p.push(&MAGIC_COOKIE);
			p.push(options_iter.0);
		},
		PacketOptions::NotDhcp(v) => p.push(v),
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
