//
//
//
//! Inter-process communication
pub use values::RpcMessage;

pub struct RpcChannel(::ObjectHandle);

impl ::Object for RpcChannel
{
	const CLASS: u16 = ::values::CLASS_IPC_RPC;
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

	type Waits = ();
}
impl RpcChannel
{
	pub fn new_pair() -> Result< (RpcChannel, RpcChannel), NewError > {
		// SAFE: Zero-operand syscall
		let rv = unsafe { syscall!(IPC_NEWPAIR) };
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
		unimplemented!();
	}
	pub fn send_obj<T: ::Object>(&self, message: RpcMessage, object: T) {
		unimplemented!();
	}
	pub fn try_receive(&self) -> Result< (RpcMessage, Option<::AnyObject>), RxError> {
		unimplemented!()
	}

	pub fn wait_rx(&self) -> ::WaitItem {
		unimplemented!();
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

