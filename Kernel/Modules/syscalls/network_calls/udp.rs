// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/network_calls/udp.rs
//! Userland interface to the network stack - UDP Sockets
use crate::args::Args;
use kernel::memory::freeze::{Freeze,FreezeMut};
use ::syscall_values::{SocketAddress,SocketAddressType};

pub struct Udp
{
	inner: ::network::udp::Socket,
}
impl Udp
{
	pub(crate) fn new(source: ::network::ipv4::Address, port: u16) -> Result<Self, crate::values::SocketError>
	{
		Ok( RawIpv4 {
			source,
			proto,
			//handle: ::network::ipv4::listen_raw(source, make_ipv4(&remote_mask.addr), remote_mask.mask),
			})
	}
}
impl super::traits::FreeSocket for Udp {
	fn send_to(&self, data: &[u8], addr: &SocketAddress) -> Result<u64, crate::Error> {
		todo!()
	}

	fn recv_from(&self, data: &mut [u8], addr: &mut SocketAddress) -> Result<u64, crate::Error> {
		todo!()
	}
}