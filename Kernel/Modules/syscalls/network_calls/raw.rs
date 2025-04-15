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
		::kernel::futures::block_on(
			::network::ipv4::send_packet(self.source, dest, self.proto, ::network::nic::SparsePacket::new_root(&data))
			);
		Ok(0)
	}

	fn recv_from(&self, data: &mut [u8], addr: &mut SocketAddress) -> Result<u64, crate::Error> {
		addr.addr_ty = SocketAddressType::Ipv4 as u8;
		todo!("NET_FREESOCK_RECV({:p}, {:p})", &*data, &*addr);
	}
}