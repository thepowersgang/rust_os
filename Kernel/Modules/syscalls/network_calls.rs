// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/network_calls.rs
//! Userland interface to the network stack
use ::syscall_values::{SocketAddress, SocketAddressType, SocketPortType};

unsafe impl crate::args::Pod for SocketAddress { }
unsafe impl crate::args::Pod for crate::values::MaskedSocketAddress { }

mod raw;
mod tcp;

fn make_ipv4(addr: &[u8; 16]) -> ::network::ipv4::Address {
	::network::ipv4::Address(
		[addr[0], addr[1], addr[2], addr[3]]
		)
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
