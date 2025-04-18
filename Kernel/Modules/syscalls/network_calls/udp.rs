// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/network_calls/udp.rs
//! Userland interface to the network stack - UDP Sockets
use ::syscall_values::SocketAddress;

fn map_err(e: ::network::udp::Error) -> crate::values::SocketError {
	match e {
	network::udp::Error::AddressInUse => todo!(),
	network::udp::Error::UnboundSocket => todo!(),
	network::udp::Error::InvalidRemote => todo!(),
	network::udp::Error::IncompatibleAddresses => todo!(),
	}
}

pub struct Udp
{
	inner: ::network::udp::SocketHandle,
}
impl Udp
{
	pub(crate) fn new(
		local_addr: Option<::network::Address>, local_port: u16,
		remote_net: ::network::Address, remote_mask: u8, remote_port: u16,
	) -> Result<Self, crate::values::SocketError>
	{
		Ok( Udp { inner: ::network::udp::SocketHandle::new(
			local_addr, local_port,
			(remote_net,remote_mask), if remote_port == 0 { None } else { Some(remote_port) }
		).map_err(map_err)? })
	}
}
impl super::traits::FreeSocket for Udp {
	fn send_to(&self, buf: &[u8], addr: &SocketAddress) -> Result<u64, crate::Error> {
		if addr.port_ty != crate::values::SocketPortType::Udp as _ {
			return Err(crate::Error::BadValue);
		}
		let port = addr.port;
		let addr = super::addr_from_socket(addr)?;
		let buf = ::network::nic::SparsePacket::new_root(buf);
		Ok(crate::from_result(self.inner.send_to(addr, port, buf).map(|_| 0u32).map_err(map_err)))
	}

	fn recv_from(&self, data: &mut [u8], out_addr: &mut SocketAddress) -> Result<u64, crate::Error> {
		Ok(crate::from_result(match self.inner.try_recv_from(data)
		{
		Some((len,addr,port)) => {
			out_addr.addr_ty = super::from_addr(&mut out_addr.addr, addr);
			out_addr.port_ty = crate::values::SocketPortType::Udp as _;
			out_addr.port = port;
			Ok(len as u32)
		},
		None => Err(crate::values::SocketError::NoData),
		}))
	}
}