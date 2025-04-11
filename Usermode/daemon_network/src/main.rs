//! Network management daemon
//!
//! Tasks:
//! - Configure interfaces using DHCP or other methods
use std::collections::btree_map::Entry;

mod dhcp;

struct Interface {
	info: ::syscalls::values::NetworkInterface,
	state_v4: Ipv4State,
}
enum Ipv4State {
	Unconfigured,
	StaticConfigured,

	Dhcp(dhcp::Dhcp),
}

fn main() {
	let net_mgr: ::syscalls::net::Management = ::syscalls::threads::S_THIS_PROCESS.receive_object("NetMgmt").unwrap();
	let mut interfaces = ::std::collections::BTreeMap::<usize,Interface>::new();
	loop {
		// Monitor network interfaces
		for iface_idx in 0 .. {
			match ::syscalls::net::Management::get_interface(iface_idx)
			{
			Some(Some(iface)) => {
				::syscalls::kernel_log!("IFace#{iface_idx}: {:x?}", iface.mac_addr);
				match interfaces.entry(iface_idx)
				{
				Entry::Occupied(mut exist) => {
					if exist.get().info.mac_addr != iface.mac_addr {
						// A change, wait what?
						remove_iface(exist.insert(Interface { info: iface, state_v4: Ipv4State::Unconfigured }));
						add_iface(&net_mgr, iface_idx, iface);
					}
					else {
						// No change
					}
				},
				Entry::Vacant(slot) => {
					slot.insert(Interface { info: iface, state_v4: Ipv4State::Unconfigured });
					add_iface(&net_mgr, iface_idx, iface);
				}
				}
			},
			Some(None) => {
				::syscalls::kernel_log!("IFace#{iface_idx}: Empty");
				if let Some(v) = interfaces.remove(&iface_idx) {
					// Removed interface
					remove_iface(v);
				}
			},
			None => {
				::syscalls::kernel_log!("IFace#{iface_idx}: END");
				break
			},
			}
		}
		
		for (&idx,iface) in interfaces.iter_mut()
		{
			match &mut iface.state_v4 {
			Ipv4State::Unconfigured => {
				// Re-attempt config?
			},
			Ipv4State::StaticConfigured => {},
			Ipv4State::Dhcp(dhcp_state) => dhcp_state.poll(&net_mgr, idx),
			}
		}

		let mut waits: Vec<_> = interfaces.iter().filter_map(|(_,v)| v.get_wait()).collect();
		//waits.push(net_mgr.wait_nic_update());
		//::syscalls::threads::wait(&mut waits, ::syscalls::system_ticks() + 10_000);
		::syscalls::threads::wait(&mut waits, !0);
	}
}

fn remove_iface(_: Interface) {
}

fn make_ipv4(a: u8, b: u8, c: u8, d: u8) -> ::syscalls::values::NetworkAddress {
	::syscalls::values::NetworkAddress {
		addr_ty: ::syscalls::values::SocketAddressType::Ipv4 as _,
		addr: [a,b,c,d,  0,0,0,0, 0,0,0,0, 0,0,0,0],
	}
}
fn add_iface(net_mgr: &::syscalls::net::Management, iface_idx: usize, iface_info: ::syscalls::values::NetworkInterface) -> Interface {
	let v4 = if iface_idx == 0 {
		net_mgr.add_address(iface_idx, make_ipv4(10,0,0,2), 24);
		net_mgr.add_route(syscalls::values::NetworkRoute {
			addr_ty: syscalls::values::SocketAddressType::Ipv4 as u8,
			network: make_ipv4(0,0,0,0).addr,
			gateway: make_ipv4(10,0,0,2).addr,
			mask: 24,
		});
		Ipv4State::StaticConfigured
	}
	else {
		// Start a DHCP task, and send the required packets
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
		net_mgr.add_address(iface_idx, addr, 16);

		match dhcp::Dhcp::new(&addr, &iface_info.mac_addr) {
		Ok(s) => Ipv4State::Dhcp(s),
		Err(()) => Ipv4State::Unconfigured,
		}
	};

	Interface { info: iface_info, state_v4: v4 }
}

impl Interface {
	fn get_wait(&self) -> Option<::syscalls::WaitItem> {
		match &self.state_v4 {
		Ipv4State::Unconfigured => None,
		Ipv4State::StaticConfigured => None,
		Ipv4State::Dhcp(dhcp_state) => dhcp_state.get_wait(), //dhcp_state.socket.wait_rx(),
		}
	}
}