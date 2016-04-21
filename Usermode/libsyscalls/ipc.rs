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
	pub fn new_pair() -> Result< (RpcChannel, RpcChannel), () > {
		unimplemented!()
	}

	pub fn send(&self, message: RpcMessage) {
		unimplemented!();
	}
	pub fn send_obj<T: ::Object>(&self, message: RpcMessage, object: T) {
		unimplemented!();
	}
	// TODO TODO TODO Use a proper type here that can be checked-casted
	pub fn try_receive(&self) -> Result< (RpcMessage, Option<::ObjectHandle>), ()> {
		unimplemented!()
	}

	pub fn wait_rx(&self) -> ::WaitItem {
		unimplemented!();
	}
}
