//
//
//
//! Inter-process communication
pub use values::RpcMessage;
use crate::values as v;
use crate::syscall;

pub struct RpcChannel(::ObjectHandle);

impl ::Object for RpcChannel
{
	const CLASS: u16 = v::CLASS_IPC_RPC;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		RpcChannel(handle)
	}
	fn into_handle(self) -> ::ObjectHandle {
		self.0
	}
	fn handle(&self) -> &::ObjectHandle {
		&self.0
	}

	type Waits = RpcChannelWaits;
	fn get_wait(&self, waits: Self::Waits) -> v::WaitItem {
		v::WaitItem { object: 0, flags: waits.0 }
	}
	fn check_wait(&self, wi: &v::WaitItem) -> Self::Waits {
		RpcChannelWaits(wi.flags)
	}
}
define_waits!{ RpcChannelWaits => (
	rx:has_rx = v::EV_IPC_RPC_RECV,
)}
impl RpcChannel
{
	pub fn new_pair() -> Result< (RpcChannel, RpcChannel), NewError > {
		// SAFE: Zero-operand syscall
		let rv = unsafe { syscall(v::IPC_NEWPAIR {}) };
		if rv == !0 {
			Err( NewError(()) )
		}
		else {
			let l = super::ObjectHandle::new( (rv & 0xFFFFFFFF) as usize ).expect("RpcChannel::new_pair - left bad");
			let r = super::ObjectHandle::new( (rv >> 32) as usize ).expect("RpcChannel::new_pair - right bad");

			Ok( (RpcChannel(l), RpcChannel(r)) )
		}
	}

	pub fn send(&self, message: RpcMessage) {
		// SAFE: Syscall
		unsafe { self.0.call_m(v::IPC_RPC_SEND { msg: &message, obj: 0 }); }
	}
	pub fn send_obj<T: ::Object>(&self, message: RpcMessage, object: T) {
		// SAFE: Syscall
		unsafe { self.0.call_m(v::IPC_RPC_SEND { msg: &message, obj: object.into_handle().into_raw()}); }
	}
	pub fn try_receive(&self) -> Result< (RpcMessage, Option<::AnyObject>), RxError> {
		let mut msg: RpcMessage = Default::default();
		// SAFE: Syscall
		let rv = unsafe { self.0.call_m(v::IPC_RPC_RECV { msg: &mut msg }) };
		if rv < 0x1000 {
			Ok( (msg, if rv > 0 { Some(::AnyObject(::ObjectHandle(rv as u32))) } else { None }) )
		}
		else {
			unimplemented!()
		}
	}

	pub fn wait_rx(&self) -> ::WaitItem {
		v::WaitItem { object: self.0 .0, flags: v::EV_IPC_RPC_RECV }
	}
}

#[derive(Debug)]
pub enum RxError
{
	NoMessage,
	ConnectionClosed,
}

#[derive(Debug)]
pub struct NewError( () );

