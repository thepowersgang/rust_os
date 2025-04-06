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
			match net_mgr.get_interface(i)
			{
			Some(Some(iface)) => {
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
				if let Some(v) = interfaces.remove(&i) {
					// Removed interface
					remove_iface(v);
				}
			},
			None => break,
			}
		}
		// Check interface against a configuration list
		// - Set static IPs when requested
		// - Or emit DHCP requests

		::syscalls::threads::wait(&mut [], ::syscalls::system_ticks() + 10_000);
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
		//net_mgr.add_route()
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
		net_mgr.add_address(iface_idx, make_ipv4(169,254,a1,a2));
		//::syscalls::net::FreeSocket::create(local, remote)
	}
}