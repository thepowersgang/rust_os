// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/network_calls.rs
//! Userland interface to the network stack
use args::Args;
use kernel::memory::freeze::{Freeze,FreezeMut};

unsafe impl ::args::Pod for ::values::SocketAddress { }
unsafe impl ::args::Pod for ::values::MaskedSocketAddress { }

pub fn new_server(local_address: ::values::SocketAddress) -> Result<u32, ::values::SocketError>
{
	todo!("new_server");
}

pub fn new_free_socket(local_address: ::values::SocketAddress, remote_mask: ::values::MaskedSocketAddress) -> Result<u32, ::values::SocketError>
{
	if local_address.port_ty != remote_mask.addr.port_ty {
		return Err(::values::SocketError::InvalidValue);
	}
	if local_address.addr_ty != remote_mask.addr.addr_ty {
		return Err(::values::SocketError::InvalidValue);
	}
	// TODO: Check that the current process is allowed to use the specified combination of port/type
	todo!("new_free_socket");
}

struct ConnServer
{
}
impl ::objects::Object for ConnServer
{
	fn class(&self) -> u16 { ::values::CLASS_SERVER }
	fn as_any(&self) -> &::core::any::Any { self }
	fn try_clone(&self) -> Option<u32> {
		None
		//Some( ::objects::new_object(self.clone()) )
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64,::Error> {
		match call
		{
		::values::NET_SERVER_ACCEPT => {
			let addr_ptr: FreezeMut<::values::SocketAddress> = try!(args.get());
			todo!("NET_SERVER_ACCEPT({:p})", &*addr_ptr);
			},
		_ => ::objects::object_has_no_such_method_ref("network_calls::ConnServer", call),
		}
	}
	fn handle_syscall_val(&mut self, call: u16, _args: &mut Args) -> Result<u64,::Error> {
		// SAFE: Valid pointer which is forgotten after call
		let _ = unsafe { ::core::ptr::read(self) };
		::objects::object_has_no_such_method_val("network_calls::ConnServer", call)
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
}
impl ::objects::Object for ConnSocket
{
	fn class(&self) -> u16 { ::values::CLASS_SOCKET }
	fn as_any(&self) -> &::core::any::Any { self }
	fn try_clone(&self) -> Option<u32> {
		None
		//Some( ::objects::new_object(self.clone()) )
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64,::Error> {
		match call
		{
		::values::NET_CONNSOCK_SHUTDOWN => {
			let what = ::values::SocketShutdownSide::try_from(args.get::<u8>()?).map_err(|_| ::Error::BadValue)?;
			todo!("NET_CONNSOCK_SHUTDOWN({:?})", what);
			},
		::values::NET_CONNSOCK_SEND => {
			let data: Freeze<[u8]> = try!(args.get());
			todo!("NET_CONNSOCK_SEND({:p})", &*data);
			},
		::values::NET_CONNSOCK_RECV => {
			let data: FreezeMut<[u8]> = try!(args.get());
			todo!("NET_CONNSOCK_RECV({:p})", &*data);
			},
		_ => ::objects::object_has_no_such_method_ref("network_calls::ConnSocket", call),
		}
	}
	fn handle_syscall_val(&mut self, call: u16, _args: &mut Args) -> Result<u64,::Error> {
		// SAFE: Valid pointer which is forgotten after call
		let _ = unsafe { ::core::ptr::read(self) };
		::objects::object_has_no_such_method_val("network_calls::ConnSocket", call)
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
}

impl ::objects::Object for FreeSocket
{
	fn class(&self) -> u16 { ::values::CLASS_FREESOCKET }
	fn as_any(&self) -> &::core::any::Any { self }
	fn try_clone(&self) -> Option<u32> {
		None
		//Some( ::objects::new_object(self.clone()) )
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64,::Error> {
		match call
		{
		::values::NET_FREESOCK_SEND => {
			let data: Freeze<[u8]> = try!(args.get());
			todo!("NET_FREESOCK_SEND({:p})", &*data);
			},
		::values::NET_FREESOCK_RECV => {
			let data: FreezeMut<[u8]> = try!(args.get());
			todo!("NET_FREESOCK_RECV({:p})", &*data);
			},
		_ => ::objects::object_has_no_such_method_ref("network_calls::FreeSocket", call),
		}
	}
	fn handle_syscall_val(&mut self, call: u16, _args: &mut Args) -> Result<u64,::Error> {
		// SAFE: Valid pointer which is forgotten after call
		let _ = unsafe { ::core::ptr::read(self) };
		::objects::object_has_no_such_method_val("network_calls::FreeSocket", call)
	}
	fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 {
		0
	}
	fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 {
		0
	}
}

