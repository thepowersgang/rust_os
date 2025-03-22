// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/network_calls/raw.rs
//! Userland interface to the network stack - Raw Sockets
use crate::args::Args;
use kernel::memory::freeze::{Freeze,FreezeMut};
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
impl crate::objects::Object for RawIpv4
{
	fn class(&self) -> u16 { crate::values::CLASS_FREESOCKET }
	fn as_any(&self) -> &dyn core::any::Any { self }
	fn try_clone(&self) -> Option<u32> {
		None
		//Some( ::objects::new_object(self.clone()) )
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64, crate::Error> {
		match call
		{
		crate::values::NET_FREESOCK_SENDTO => {
			let data: Freeze<[u8]> = args.get()?;
			let addr: Freeze<SocketAddress> = args.get()?;
			if addr.addr_ty != SocketAddressType::Ipv4 as u8 {
				return Err(crate::Error::BadValue);
			}
			let dest = super::make_ipv4(&addr.addr);
			::kernel::futures::block_on(
				::network::ipv4::send_packet(self.source, dest, self.proto, ::network::nic::SparsePacket::new_root(&data))
				);
			Ok(0)
			},
		crate::values::NET_FREESOCK_RECVFROM => {
			let data: FreezeMut<[u8]> = args.get()?;
			let mut addr: FreezeMut<SocketAddress> = args.get()?;
			addr.addr_ty = SocketAddressType::Ipv4 as u8;
			todo!("NET_FREESOCK_RECV({:p}, {:p})", &*data, &*addr);
			},
		_ => crate::objects::object_has_no_such_method_ref("network_calls::FreeSocket", call),
		}
	}
	fn handle_syscall_val(&mut self, call: u16, _args: &mut Args) -> Result<u64, crate::Error> {
		// SAFE: Valid pointer which is forgotten after call
		let _ = unsafe { ::core::ptr::read(self) };
		crate::objects::object_has_no_such_method_val("network_calls::FreeSocket", call)
	}
	fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 {
		0
	}
	fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 {
		0
	}
}