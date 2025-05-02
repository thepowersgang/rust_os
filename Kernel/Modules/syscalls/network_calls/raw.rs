// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/network_calls/raw.rs
//! Userland interface to the network stack - Raw Sockets
use ::syscall_values::{SocketAddress,SocketAddressType};

pub struct RawIpv4
{
	source: ::network::ipv4::Address,
	proto: u8,
}
impl RawIpv4
{
	pub(crate) fn new(source: ::network::ipv4::Address, proto: u8) -> Result<Self, crate::values::SocketError>
	{
		Ok( RawIpv4 {
			source,
			proto,
			//handle: ::network::ipv4::listen_raw(source, make_ipv4(&remote_mask.addr), remote_mask.mask),
			})
	}
}
impl super::traits::FreeSocket for RawIpv4
{
	fn send_to(&self, data: &[u8], addr: &SocketAddress) -> Result<u64, crate::Error> {
		if addr.addr_ty != SocketAddressType::Ipv4 as u8 {
			return Err(crate::Error::BadValue);
		}
		let dest = super::make_ipv4(&addr.addr);
		Ok(crate::from_result::<_,::syscall_values::SocketError>(
			::kernel::futures::block_on(
				::network::ipv4::send_packet(self.source, dest, self.proto, ::network::nic::SparsePacket::new_root(&data))
				)
			.map_err(|()| ::syscall_values::SocketError::NoRoute)
			.map(|()| 0u32)
		))
	}

	fn recv_from(&self, data: &mut [u8], addr: &mut SocketAddress) -> Result<u64, crate::Error> {
		addr.addr_ty = SocketAddressType::Ipv4 as u8;
		todo!("NET_FREESOCK_RECV({:p}, {:p})", &*data, &*addr);
	}
	fn bind_wait_recv(&self, _obj: &mut kernel::threads::SleepObject) -> bool {
		todo!("bind_wait_recv")
	}
	fn unbind_wait_recv(&self, _obj: &mut kernel::threads::SleepObject) -> bool {
		todo!("unbind_wait_recv")
	}
}

pub struct RawIpv6
{
	source: ::network::ipv6::Address,
	proto: u8,
	handle: ::network::ipv6::RawListenHandle,
}
impl RawIpv6
{
	pub(crate) fn new(source: ::network::ipv6::Address, proto: u8, remote: (::network::ipv6::Address,u8)) -> Result<Self, crate::values::SocketError>
	{
		Ok( RawIpv6 {
			source,
			proto,
			handle: ::network::ipv6::RawListenHandle::new(proto, source, remote)
				.map_err(|()| crate::values::SocketError::AlreadyInUse)?,
			})
	}
}
impl super::traits::FreeSocket for RawIpv6
{
	fn send_to(&self, data: &[u8], addr: &SocketAddress) -> Result<u64, crate::Error> {
		if addr.addr_ty != SocketAddressType::Ipv6 as u8 {
			return Err(crate::Error::BadValue);
		}
		let destination = super::make_ipv6(&addr.addr);
		Ok(crate::from_result::<_,::syscall_values::SocketError>(
			::kernel::futures::block_on(
				::network::ipv6::send_packet(self.source, destination, self.proto, ::network::nic::SparsePacket::new_root(&data))
				)
			.map_err(|()| ::syscall_values::SocketError::NoRoute)
			.map(|()| 0u32)
		))
	}

	fn recv_from(&self, data: &mut [u8], addr: &mut SocketAddress) -> Result<u64, crate::Error> {
		addr.addr_ty = SocketAddressType::Ipv6 as u8;
		addr.port_ty = crate::values::SocketPortType::Raw as _;
		Ok(crate::from_result(if let Some((src, len)) = self.handle.pop(data) {
			addr.addr = super::from_ipv6(src);
			addr.port = 0;
			Ok(len as u32)
		}
		else {
			Err(crate::values::SocketError::NoData)
		}))
	}
	fn bind_wait_recv(&self, so: &mut kernel::threads::SleepObject) -> bool {
		self.handle.register_wait(so);
		self.handle.has_packet()
	}
	fn unbind_wait_recv(&self, so: &mut kernel::threads::SleepObject) -> bool {
		self.handle.clear_wait(so);
		self.handle.has_packet()
	}
}