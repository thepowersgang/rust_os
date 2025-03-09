// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/network_calls.rs
//! Userland interface to the network stack
use crate::args::Args;
use kernel::memory::freeze::{Freeze,FreezeMut};

unsafe impl crate::args::Pod for crate::values::SocketAddress { }
unsafe impl crate::args::Pod for crate::values::MaskedSocketAddress { }

fn from_tcp_result(r: Result<usize, ::network::tcp::ConnError>) -> u64 {
	crate::from_result::<_, crate::values::SocketError>(match r
		{
		Ok(v) => Ok(v as u32),
		Err(e) => todo!("Convert error: {:?}", e),
		})
}

pub fn new_server(local_address: crate::values::SocketAddress) -> Result<u32, crate::values::SocketError>
{
	Ok(crate::objects::new_object(ConnServer {
		inner: if local_address.addr == [0; 16] {
				match ::network::tcp::ServerHandle::listen(local_address.port)
				{
				Ok(v) => v,
				Err(e) => match e {
					::network::tcp::ListenError::SocketInUse => return Err(::syscall_values::SocketError::AlreadyInUse),
					},
				}
			}
			else {
				todo!("new_server({:?}", local_address)
			}
		}))
}
pub fn new_client(remote_address: crate::values::SocketAddress) -> Result<u64, super::Error>
{
	let a = match crate::values::SocketAddressType::try_from(remote_address.addr_ty) {
		Err(_) => return Err(super::Error::BadValue),
		Ok(crate::values::SocketAddressType::Ipv4) => ::network::Address::Ipv4(::network::ipv4::Address(
			[remote_address.addr[0], remote_address.addr[1], remote_address.addr[2], remote_address.addr[3]]
			)),
		_ => todo!(""),
		};
	let o = match crate::values::SocketPortType::try_from(remote_address.port_ty).map_err(|_| super::Error::BadValue)?
		{
		crate::values::SocketPortType::Tcp => {
			crate::objects::new_object(ConnSocket {
				inner: match ::network::tcp::ConnectionHandle::connect(a, remote_address.port)
					{
					Ok(v) => v,
					Err(e) => todo!("{:?}", e),
					}
				})
			},
		t => todo!("Socket type: {:?}", t),
		};
	Ok(super::from_result::<_,::syscall_values::SocketError>(Ok( o )))
}

/// Create a UDP socket
pub fn new_free_socket(local_address: crate::values::SocketAddress, remote_mask: crate::values::MaskedSocketAddress) -> Result<u32, crate::values::SocketError>
{
	if local_address.port_ty != remote_mask.addr.port_ty {
		return Err(crate::values::SocketError::InvalidValue);
	}
	if local_address.addr_ty != remote_mask.addr.addr_ty {
		return Err(crate::values::SocketError::InvalidValue);
	}
	// TODO: Check that the current process is allowed to use the specified combination of port/type
	Ok(crate::objects::new_object(FreeSocket {
		inner: todo!("new_free_socket"),
		}))
}

struct ConnServer
{
	inner: ::network::tcp::ServerHandle,
}
impl crate::objects::Object for ConnServer
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
			let mut addr_ptr: FreezeMut<crate::values::SocketAddress> = args.get()?;
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
				Ok(super::objects::new_object(ConnSocket { inner: v }) as u64)
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

struct ConnSocket
{
	inner: ::network::tcp::ConnectionHandle,
}
impl crate::objects::Object for ConnSocket
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

struct FreeSocket
{
	inner: (),
}

impl crate::objects::Object for FreeSocket
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
		crate::values::NET_FREESOCK_SEND => {
			let data: Freeze<[u8]> = args.get()?;
			todo!("NET_FREESOCK_SEND({:p})", &*data);
			},
		crate::values::NET_FREESOCK_RECV => {
			let data: FreezeMut<[u8]> = args.get()?;
			todo!("NET_FREESOCK_RECV({:p})", &*data);
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

