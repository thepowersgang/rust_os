//! Network management daemon
//!
//! Tasks:
//! - Configure interfaces using DHCP or other methods

fn main() {
	let net_mgr: ::syscalls::net::Management = ::syscalls::threads::S_THIS_PROCESS.receive_object("NetMgmt").unwrap();
	let mut interfaces = ::std::collections::BTreeMap::<usize,_>::new();
	loop {
		// Monitor network interfaces
		for i in 0 .. {
			match ::syscalls::net::Management::get_interface(i)
			{
			Some(Some(iface)) => {
				::syscalls::kernel_log!("IFace#{i}: {:x?}", iface.mac_addr);
				// Found.
				if let Some(v) = interfaces.insert(i, iface) {
					if v != iface {
						// Oh, this has changed!
						remove_iface(v);
						add_iface(&net_mgr, i, iface);
					}
				}
				else {
					// New interface
					add_iface(&net_mgr, i, iface);
				}
			},
			Some(None) => {
				::syscalls::kernel_log!("IFace#{i}: Empty");
				if let Some(v) = interfaces.remove(&i) {
					// Removed interface
					remove_iface(v);
				}
			},
			None => {
				::syscalls::kernel_log!("IFace#{i}: END");
				break
			},
			}
		}
		// Check interface against a configuration list
		// - Set static IPs when requested
		// - Or emit DHCP requests

		//::syscalls::threads::wait(&mut [], ::syscalls::system_ticks() + 10_000);
		::syscalls::threads::wait(&mut [], !0);
	}
}

fn remove_iface(_: ::syscalls::values::NetworkInterface) {
}

fn make_ipv4(a: u8, b: u8, c: u8, d: u8) -> ::syscalls::values::NetworkAddress {
	::syscalls::values::NetworkAddress {
		addr_ty: ::syscalls::values::SocketAddressType::Ipv4 as _,
		addr: [a,b,c,d,  0,0,0,0, 0,0,0,0, 0,0,0,0],
	}
}
fn add_iface(net_mgr: &::syscalls::net::Management, iface_idx: usize, iface_info: ::syscalls::values::NetworkInterface) {
	if iface_idx == 0 {
		net_mgr.add_address(iface_idx, make_ipv4(10,0,0,2));
		net_mgr.add_route(syscalls::values::NetworkRoute {
			addr_ty: syscalls::values::SocketAddressType::Ipv4 as u8,
			network: make_ipv4(0,0,0,0).addr,
			gateway: make_ipv4(10,0,0,2).addr,
			mask: 24,
		});
	}
	else {
		// TODO: Start a DHCP task, and send the required packets
		// - To do this, add a link-local address based on a hashed mac

		let a_frag = {
			let mut h: u16 = 0;
			for b in iface_info.mac_addr {
				h = h.wrapping_mul(12345).wrapping_add(b as u16);
			}
			h
		};

		let [a1,a2] = a_frag.to_le_bytes();
		let addr = make_ipv4(169,254,a1,a2);
		net_mgr.add_address(iface_idx, addr);
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
				return
			},
			};
		let dhcp_request_pkt = {
			let mut pkt = [0; 7*4+16+64+128];
			let mut pos = 0;
			let mut push_bytes = |data: &[u8]| {
				pkt[pos..][..data.len()].copy_from_slice(data);
				pos += data.len();
			};
			// op = REQUEST
			// HW Address type
			// HW Address lengt
			// Hop count, set to zero to start with
			push_bytes(&[1, 1, 6, 0]);
			push_bytes(&1234567u32.to_be_bytes());	// u32 Transaction ID
			push_bytes(&[0, 0]);	// u16: Seconds since start of process
			push_bytes(&[0, 0]);	// u16: flags
			push_bytes(&[0; 4]);	// [u8; 4] ciaddr
			push_bytes(&addr.addr[..4]);	// [u8; 4] yiaddr
			push_bytes(&[0; 4]);	// [u8; 4] siaddr
			push_bytes(&[0; 4]);	// [u8; 4] giaddr
			push_bytes(&iface_info.mac_addr); push_bytes(&[0; 16-6]);	// [u8; 16] chaddr
			//push_bytes()
			pkt
		};
		match s.send_to(&dhcp_request_pkt, ::syscalls::net::SocketAddress {
			port_ty: ::syscalls::values::SocketPortType::Udp as _,
			addr_ty: addr.addr_ty,
			port: 67,	// DHCP
			addr: [0xFF; 16],
		}) {
		Ok(_) => {},
		Err(e) => {
			::syscalls::kernel_log!("Error sending DHCP request: {:?}", e);
			return
		},
		}
	}
}