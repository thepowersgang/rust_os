// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/network_calls/tcp.rs
//! Userland interface to the network stack - TCP
use crate::args::Args;
use kernel::memory::freeze::{Freeze,FreezeMut};
use ::syscall_values::SocketAddress;

fn map_tcp_err(e: ::network::tcp::ConnError) -> crate::values::SocketError {
	use ::network::tcp::ConnError as S;
	use ::syscall_values::SocketError as D;
	match e
	{
	S::NoRoute       => D::NoRoute,
	S::LocalClosed   => D::SocketClosed,
	S::RemoteRefused => D::ConnectionReset,
	S::RemoteClosed  => D::SocketClosed,
	S::RemoteReset   => D::ConnectionReset,
	S::NoPortAvailable => todo!("TCP error {:?}", e),
	}
}
fn from_tcp_result(r: Result<usize, ::network::tcp::ConnError>) -> u64 {
	crate::from_result::<_, crate::values::SocketError>(match r
		{
		Ok(v) => Ok(v as u32),
		Err(e) => Err(map_tcp_err(e)),
		})
}

pub struct TcpServer
{
	inner: ::network::tcp::ServerHandle,
}
impl TcpServer
{
	pub(crate) fn listen(_addr: ::network::Address, port: u16) -> Result<Self, ::syscall_values::SocketError> {
		Ok(TcpServer {
			inner: match ::network::tcp::ServerHandle::listen(port)
				{
				Ok(v) => v,
				Err(e) => match e {
					::network::tcp::ListenError::SocketInUse => return Err(::syscall_values::SocketError::AlreadyInUse),
					},
				}
			})
	}
}
impl crate::objects::Object for TcpServer
{
	fn class(&self) -> u16 { crate::values::CLASS_SERVER }
	fn as_any(&self) -> &dyn core::any::Any { self }
	fn try_clone(&self) -> Option<u32> {
		None
		//Some( ::objects::new_object(self.clone()) )
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64,crate::Error> {
		match call
		{
		crate::values::NET_SERVER_ACCEPT => {
			let mut addr_ptr: FreezeMut<SocketAddress> = args.get()?;
			match self.inner.accept()
			{
			Some(v) => {
				let (a,p) = v.remote_addr();
				match a {
				network::Address::Ipv4(a) => {
					addr_ptr.addr_ty = ::syscall_values::SocketAddressType::Ipv4 as _;
					addr_ptr.addr[..4].copy_from_slice(&a.0);
					},
				}
				addr_ptr.port_ty = ::syscall_values::SocketPortType::Tcp as _;
				addr_ptr.port = p;
				Ok(crate::objects::new_object(TcpSocket { inner: v }) as u64)
				},
			None => Ok(0),
			}
			},
		_ => crate::objects::object_has_no_such_method_ref("network_calls::ConnServer", call),
		}
	}
	fn handle_syscall_val(&mut self, call: u16, _args: &mut Args) -> Result<u64,crate::Error> {
		// SAFE: Valid pointer which is forgotten after call
		let _ = unsafe { ::core::ptr::read(self) };
		crate::objects::object_has_no_such_method_val("network_calls::ConnServer", call)
	}
	fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 {
		0
	}
	fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 {
		0
	}
}

pub struct TcpSocket
{
	inner: ::network::tcp::ConnectionHandle,
}
impl TcpSocket
{
	pub(crate) fn connect(addr: ::network::Address, port: u16) -> Result<Self, ::syscall_values::SocketError>
	{
		Ok(TcpSocket {
			inner: match ::network::tcp::ConnectionHandle::connect(addr, port)
				{
				Ok(v) => v,
				Err(e) => return Err(map_tcp_err(e)),
				}
			})
	}
}
impl crate::objects::Object for TcpSocket
{
	fn class(&self) -> u16 { crate::values::CLASS_SOCKET }
	fn as_any(&self) -> &dyn core::any::Any { self }
	fn try_clone(&self) -> Option<u32> {
		None
		//Some( ::objects::new_object(self.clone()) )
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64, crate::Error> {
		Ok(match call
		{
		crate::values::NET_CONNSOCK_SHUTDOWN => {
			let what = crate::values::SocketShutdownSide::try_from(args.get::<u8>()?).map_err(|_| crate::Error::BadValue)?;
			todo!("NET_CONNSOCK_SHUTDOWN({:?})", what);
			},
		crate::values::NET_CONNSOCK_SEND => {
			let data: Freeze<[u8]> = args.get()?;
			from_tcp_result(self.inner.send_data(&data))
			},
		crate::values::NET_CONNSOCK_RECV => {
			let mut data: FreezeMut<[u8]> = args.get()?;
			from_tcp_result(self.inner.recv_data(&mut data))
			},
		_ => return crate::objects::object_has_no_such_method_ref("network_calls::ConnSocket", call),
		})
	}
	fn handle_syscall_val(&mut self, call: u16, _args: &mut Args) -> Result<u64, crate::Error> {
		// SAFE: Valid pointer which is forgotten after call
		let _ = unsafe { ::core::ptr::read(self) };
		crate::objects::object_has_no_such_method_val("network_calls::ConnSocket", call)
	}
	fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 {
		0
	}
	fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 {
		0
	}
}