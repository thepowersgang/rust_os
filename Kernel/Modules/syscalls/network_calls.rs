// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/network_calls.rs
//! Userland interface to the network stack
use ::syscall_values::{SocketAddress, SocketAddressType, SocketPortType};

unsafe impl crate::args::Pod for SocketAddress { }
unsafe impl crate::args::Pod for crate::values::MaskedSocketAddress { }
unsafe impl crate::args::Pod for crate::values::NetworkInterface {}
unsafe impl crate::args::Pod for crate::values::NetworkRoute {}
unsafe impl crate::args::Pod for crate::values::NetworkAddress {}

mod raw;
mod tcp;

pub fn init_handles() {
	crate::objects::push_as_unclaimed("NetMgmt", crate::objects::new_object(InterfaceManagement));
}

fn make_ipv4(addr: &[u8; 16]) -> ::network::ipv4::Address {
	::network::ipv4::Address(
		[addr[0], addr[1], addr[2], addr[3]]
		)
}
fn from_ipv4(a: ::network::ipv4::Address) -> [u8; 16] {
	[
		a.0[0], a.0[1], a.0[2], a.0[3],
		0,0,0,0, 0,0,0,0, 0,0,0,0,
	]
}

/// Open a connection-based listen server
pub fn new_server(local_address: SocketAddress) -> Result<u64, super::Error>
{
	fn inner(addr: ::network::Address, port_ty: crate::values::SocketPortType, port: u16) -> Result<u32, ::syscall_values::SocketError>
	{
		match port_ty {
		crate::values::SocketPortType::Tcp => Ok(crate::objects::new_object(tcp::TcpServer::listen(addr, port)?)),
		t => todo!("Socket type: {:?}", t),
		}
	}
	let addr = match SocketAddressType::try_from(local_address.addr_ty)
		{
		Err(_) => return Err(super::Error::BadValue),
		Ok(SocketAddressType::Ipv4) => ::network::Address::Ipv4(make_ipv4(&local_address.addr)),
		_ => todo!(""),
		};
	let port_ty = crate::values::SocketPortType::try_from(local_address.port_ty).map_err(|_| super::Error::BadValue)?;
	let o =  inner(addr, port_ty, local_address.port);
	Ok(super::from_result::<_,::syscall_values::SocketError>(o))
}

/// Open a connection-based socket
pub fn new_client(remote_address: SocketAddress) -> Result<u64, super::Error>
{
	fn inner(addr: ::network::Address, port_ty: crate::values::SocketPortType, port: u16) -> Result<u32, ::syscall_values::SocketError> {
		Ok(match port_ty
		{
		crate::values::SocketPortType::Tcp => crate::objects::new_object(tcp::TcpSocket::connect(addr, port)?),
		t => todo!("Socket type: {:?}", t),
		})
	}
	let addr = match SocketAddressType::try_from(remote_address.addr_ty)
		{
		Err(_) => return Err(super::Error::BadValue),
		Ok(SocketAddressType::Ipv4) => ::network::Address::Ipv4(make_ipv4(&remote_address.addr)),
		_ => todo!(""),
		};
	let port_ty = crate::values::SocketPortType::try_from(remote_address.port_ty).map_err(|_| super::Error::BadValue)?;
	let o = inner(addr, port_ty, remote_address.port);
	Ok(super::from_result::<_,::syscall_values::SocketError>(o))
}

/// Create a connectionless socket
pub fn new_free_socket(local_address: SocketAddress, remote_mask: crate::values::MaskedSocketAddress) -> Result<u64, super::Error>
{
	fn inner(local_address: SocketAddress, remote_mask: crate::values::MaskedSocketAddress, port_ty: SocketPortType, addr_ty: SocketAddressType) -> Result<u32, ::syscall_values::SocketError>
	{
		if local_address.port_ty != remote_mask.addr.port_ty {
			Err(crate::values::SocketError::InvalidValue)
		}
		else if local_address.addr_ty != remote_mask.addr.addr_ty {
			Err(crate::values::SocketError::InvalidValue)
		}
		else {
			// TODO: Check that the current process is allowed to use the specified combination of port/type

			match port_ty
			{
			SocketPortType::Raw => match addr_ty
				{
				SocketAddressType::Ipv4 => {
					let source = make_ipv4(&local_address.addr);
					if local_address.port > u8::MAX as u16 {
						Err(crate::values::SocketError::InvalidValue)
					}
					else {
						Ok(crate::objects::new_object(raw::RawIpv4::new(source, local_address.port as u8)?))
					}
					},
				_ => todo!("Handle other address types"),
				},
			SocketPortType::Tcp => Err(crate::values::SocketError::InvalidValue),
			SocketPortType::Udp => todo!("Handle other socket types"),
			_ => todo!("Handle other socket types"),
			}
		}
	}
	let port_ty = SocketPortType::try_from(local_address.port_ty).map_err(|_| super::Error::BadValue)?;
	let addr_ty = SocketAddressType::try_from(local_address.addr_ty).map_err(|_| super::Error::BadValue)?;
	let r = inner(local_address, remote_mask, port_ty, addr_ty);
	Ok(super::from_result::<_,::syscall_values::SocketError>(r))
}

pub(crate) struct InterfaceManagement;
impl super::objects::Object for InterfaceManagement {
	fn as_any(&self) -> &dyn core::any::Any {
		self
	}

	fn class(&self) -> u16 {
		::syscall_values::CLASS_NET_MANAGEMENT
	}

	fn try_clone(&self) -> Option<u32> {
		None
	}

	fn handle_syscall_ref(&self, call: u16, args: &mut crate::args::Args) -> Result<u64,crate::Error> {
		Ok(match call {
		::syscall_values::NET_MGMT_GET_INTERFACE => {
			let index = args.get()?;
			let mut out: crate::FreezeMut<::syscall_values::NetworkInterface> = args.get()?;
			if index >= ::network::nic::count_interfaces() {
				!0
			}
			else if let Some(ii) = ::network::nic::interface_info(index) {
				out.mac_addr = ii.mac;
				0
			}
			else {
				1
			}
		},
		// --- Addresses ---
		::syscall_values::NET_MGMT_ADD_ADDRESS => {
			let iface_idx: usize = args.get()?;
			let addr: crate::Freeze<::syscall_values::NetworkAddress> = args.get()?;
			let mask_bits: u8 = args.get()?;
			log_debug!("NET_MGMT_ADD_ADDRESS({iface_idx}, {:?} / {mask_bits})", addr.addr);

			let Some(ii) = ::network::nic::interface_info(iface_idx) else {
				return Err(crate::Error::BadValue);
			};

			match match ::syscall_values::SocketAddressType::try_from(addr.addr_ty)
				{
				Ok(v) => v,
				Err(_) => return Err(crate::Error::BadValue),
				}
			{
			::syscall_values::SocketAddressType::Mac => todo!(),
			::syscall_values::SocketAddressType::Ipv4 => {
				::network::ipv4::add_interface(ii.mac, make_ipv4(&addr.addr), mask_bits);
				0
				}
			::syscall_values::SocketAddressType::Ipv6 => todo!(),
			}
		},
		//::syscall_values::NET_MGMT_DEL_ADDRESS => {},
		/*
		::syscall_values::NET_MGMT_GET_ADDRESS => {
			let index: usize = args.get()?;
			let mut data: ::kernel::memory::freeze::FreezeMut<::syscall_values::NetworkRoute> = args.get()?;
			let addr_ty = match ::syscall_values::SocketAddressType::try_from(data.addr_ty)
				{
				Ok(v) => v,
				Err(_) => return Err(crate::Error::BadValue),
				};
			match addr_ty {
			SocketAddressType::Mac => todo!(),
			SocketAddressType::Ipv4 => {
				//::network::ipv4::
				todo!();
			}
			SocketAddressType::Ipv6 => todo!(),
			}
		},
		*/
		// --- Routes ---
		::syscall_values::NET_MGMT_ADD_ROUTE => {
			let route: crate::Freeze<::syscall_values::NetworkRoute> = args.get()?;
			match match ::syscall_values::SocketAddressType::try_from(route.addr_ty)
				{
				Ok(v) => v,
				Err(_) => return Err(crate::Error::BadValue),
				}
			{
			::syscall_values::SocketAddressType::Mac => return Err(crate::Error::BadValue),
			::syscall_values::SocketAddressType::Ipv4 => {
				let rv = ::network::ipv4::route_add(::network::ipv4::Route {
					network: make_ipv4(&route.network),
					mask: route.mask,
					next_hop: make_ipv4(&route.gateway),
				});
				if rv { 0 } else { 1 }
				}
			::syscall_values::SocketAddressType::Ipv6 => todo!(),
			}
		},
		::syscall_values::NET_MGMT_DEL_ROUTE => {
			let route: crate::Freeze<::syscall_values::NetworkRoute> = args.get()?;
			match match ::syscall_values::SocketAddressType::try_from(route.addr_ty)
				{
				Ok(v) => v,
				Err(_) => return Err(crate::Error::BadValue),
				}
			{
			::syscall_values::SocketAddressType::Mac => return Err(crate::Error::BadValue),
			::syscall_values::SocketAddressType::Ipv4 => {
				let rv =::network::ipv4::route_del(::network::ipv4::Route {
					network: make_ipv4(&route.network),
					mask: route.mask,
					next_hop: make_ipv4(&route.gateway),
				});
				if rv { 0 } else { 1 }
				}
			::syscall_values::SocketAddressType::Ipv6 => todo!(),
			}
		},
		::syscall_values::NET_MGMT_GET_ROUTE => {
			let index: usize = args.get()?;
			let mut data: ::kernel::memory::freeze::FreezeMut<::syscall_values::NetworkRoute> = args.get()?;
			let addr_ty = match ::syscall_values::SocketAddressType::try_from(data.addr_ty)
				{
				Ok(v) => v,
				Err(_) => return Err(crate::Error::BadValue),
				};
			match addr_ty {
			SocketAddressType::Mac => todo!(),
			SocketAddressType::Ipv4 => {
				let (maxlen, rv) = ::network::ipv4::route_enumerate(index);
				if index >= maxlen {
					!0
				}
				else if let Some(rv) = rv {
					*data = ::syscall_values::NetworkRoute {
						network: from_ipv4(rv.network),
						gateway: from_ipv4(rv.next_hop),
						addr_ty: addr_ty as u8,
						mask: rv.mask,
						//interface: 0,
					};
					1
				}
				else {
					0
				}
			},
			SocketAddressType::Ipv6 => todo!(),
			}
		},
		_ => return Err(crate::Error::UnknownCall),
		})
	}

	fn bind_wait(&self, flags: u32, _obj: &mut kernel::threads::SleepObject) -> u32 {
		let mut rv = 0;
		if flags & ::syscall_values::EV_NET_MGMT_INTERFACE != 0 {
			rv += 1;
		}
		rv
	}

	fn clear_wait(&self, flags: u32, _obj: &mut kernel::threads::SleepObject) -> u32 {
		let mut rv = 0;
		if flags & ::syscall_values::EV_NET_MGMT_INTERFACE != 0 {
			rv += 1;
		}
		rv
	}
}
