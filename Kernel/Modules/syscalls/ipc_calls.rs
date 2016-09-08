// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/ipc_calls.rs
//! Userland interface to IPC channels
use args::Args;
use kernel::memory::freeze::{Freeze,FreezeMut};

struct SyncChannel( () );
impl ::objects::Object for SyncChannel
{
	const CLASS: u16 = ::values::CLASS_IPC_RPC;
	fn class(&self) -> u16 { Self::CLASS }
	fn as_any(&self) -> &::core::any::Any { self }
	fn try_clone(&self) -> Option<u32> {
		None
		//Some( ::objects::new_object( Node(self.0.clone()) ) )
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64,::Error> {
		match call
		{
		::values::IPC_RPC_SEND => {
			let data: Freeze<::values::RpcMessage> = try!(args.get());
			let obj: u32 = try!(args.get());
			todo!("IPC_RPC_SENDBLOB({:p}, {})", &*data, obj);
			},
		::values::IPC_RPC_RECV => {
			let _data: FreezeMut<::values::RpcMessage> = try!(args.get());
			//todo!("IPC_RPC_RECV({:p})", &*data);
			Ok( 0x1000 )
			},
		_ => ::objects::object_has_no_such_method_ref("ipc_calls::SyncChannel", call),
		}
	}
	fn handle_syscall_val(&mut self, call: u16, _args: &mut Args) -> Result<u64,::Error> {
		// SAFE: Valid pointer which is forgotten after call
		let _ = unsafe { ::core::ptr::read(self) };
		::objects::object_has_no_such_method_val("ipc_calls::SyncChannel", call)
	}
	fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
	fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
}

pub fn new_pair() -> Result< (u32,u32), () >
{
	let a = ::objects::new_object( SyncChannel( () ) );
	if a == !0 {
		return Err( () );
	}

	let b = ::objects::new_object( SyncChannel( () ) );
	if b == !0 {
		::objects::drop_object(a);
		return Err( () );
	}

	Ok( (a,b) )
}
