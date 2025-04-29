pub struct State {
	listen_sock: ::syscalls::net::FreeSocket,
	//mac_addr: [u8; 6],
	link_local_addr: [u8; 16],
	prefix_spec: Option<PrefixSpec>,
}
#[derive(PartialEq)]
struct PrefixSpec {
	router: [u8; 16],
	addr: [u8; 16],
	prefix_len: u8,
}
impl State {
	pub fn new(addr: &::syscalls::values::NetworkAddress, _mac_addr: &[u8; 6]) -> Result<State,()>
	{
		Ok(State {
			listen_sock: ::syscalls::net::FreeSocket::create(::syscalls::values::SocketAddress {
					port_ty: ::syscalls::values::SocketPortType::Raw as _,
					addr_ty: addr.addr_ty,
					port: 58,
					addr: addr.addr,
				},
				::syscalls::values::MaskedSocketAddress::default(),
			).map_err(|_| ())?,
			link_local_addr: addr.addr,
			//mac_addr: *mac_addr,
			prefix_spec: None,
		})
	}

	pub fn get_wait(&self) -> Option<::syscalls::WaitItem> {
		Some(self.listen_sock.wait_read())
	}

	pub fn poll(&mut self, mgr: &::syscalls::net::Management, iface_idx: usize) {
		::syscalls::kernel_log!("IPv6 AutoConf: poll");
		let mut packet_data = [0; 128];
		while let Some((src, data)) = opt_rx_packet(&self.listen_sock, &mut packet_data) {
			let ty = data[0];
			let _code = data[1];
			let _cksum = u16::from_be_bytes([data[2], data[3]]);
			let tail_data = &data[4..];
			match ty {
			134 => {	// Router advertisement
				use ::std::convert::TryInto;
				let _hop_limit = tail_data[0];
				let flags = tail_data[1];
				let _flag_managed_config = flags & (1 << 0) != 0;
				let _flag_other_config = flags & (1 << 1) != 0;
				let _router_lifetime = u16::from_be_bytes([tail_data[2], tail_data[3]]);
				let _reachable_time = u32::from_be_bytes(tail_data[4..][..4].try_into().unwrap());
				let _retrans_time = u32::from_be_bytes(tail_data[8..][..4].try_into().unwrap());
				let mut options = &tail_data[12..];
				while options.len() != 0 {
					let o = options[0]; options = &options[1..];
					if o == 0 {
						continue ;
					}
					if options.len() == 0 {
						break ;
					}
					let len = options[0]; options = &options[1..];
					let len = (len as usize).min( options.len() );
					let data = &options[..len];
					match o {
					0 => unreachable!(),
					3 => {
						if data.len() >= 14+16 {
							let prefix_len = data[0];
							let flags = data[1];
							let _flag_on_link = flags & (1 << 0) != 0;
							let flag_auto_config = flags & (1 << 1) != 0;
							let _valid_leftime = &data[2..][..4];
							let _preferred_lifetime = &data[6..][..4];
							let _reserved = &data[10..][..4];
							let prefix = &data[14..][..16];

							// TODO: 
							if flag_auto_config {
								if prefix_len != 64 {
									continue ;
								}
								let ll = self.link_local_addr;
								let addr = [
									prefix[0],prefix[1],prefix[2],prefix[3],
									prefix[4],prefix[5],prefix[6],prefix[7],
									ll[0],ll[1],ll[2],ll[3],
									ll[4],ll[5],ll[6],ll[7],
								];
								let prefix_spec = PrefixSpec {
									addr,
									router: src.addr,
									prefix_len,
									};
								if self.prefix_spec.as_ref() != Some(&prefix_spec) {
									// 1. Delete the existing
									if let Some(ref v) = self.prefix_spec {
										mgr.del_address(iface_idx, ::syscalls::values::NetworkAddress {
											addr_ty: ::syscalls::values::SocketAddressType::Ipv6 as _,
											addr: v.addr,
										}, v.prefix_len);
										mgr.del_route(syscalls::values::NetworkRoute {
											network: [0; 16], mask: 0,
											gateway: v.router,
											addr_ty: syscalls::values::SocketAddressType::Ipv6 as u8,
											});
									}
									// 2. Add the new one
									mgr.add_address(iface_idx, ::syscalls::values::NetworkAddress {
										addr_ty: ::syscalls::values::SocketAddressType::Ipv6 as _,
										addr: prefix_spec.addr,
									}, prefix_spec.prefix_len);
									mgr.add_route(syscalls::values::NetworkRoute {
										network: [0; 16], mask: 0,
										gateway: src.addr,
										addr_ty: syscalls::values::SocketAddressType::Ipv6 as u8,
										});
									// Save the updated prefix
									self.prefix_spec = Some(prefix_spec);
								}
							}
						}
					}
					_ => {},
					}
				}
			},
			_ => {},
			}
		}
	}
}

fn opt_rx_packet<'a>(socket: &::syscalls::net::FreeSocket, packet_data: &'a mut [u8]) -> Option<(::syscalls::values::SocketAddress, &'a [u8])>
{
	match socket.recv_from(packet_data)
	{
	Err(::syscalls::net::Error::NoData) => None,
	Err(e) => {
		::syscalls::kernel_log!("ipv6_ra: Error reciving packet {:?}", e);
		None
	},
	Ok((len, remote)) => {
		let packet_data = &packet_data[..len];
		//let packet = DhcpPacket::from_bytes(packet_data);
		Some( (remote, packet_data, ) )
	},
	}
}