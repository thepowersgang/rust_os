
use crate::args::Args;
use crate::values::SocketAddress;
use kernel::memory::freeze::{Freeze,FreezeMut};

pub trait FreeSocket: 'static + Sized + Send + Sync
{
	fn try_clone(&self) -> Option<Self> {
		None
	}
	fn send_to(&self, data: &[u8], addr: &SocketAddress) -> Result<u64, crate::Error>;
	fn recv_from(&self, data: &mut [u8], addr: &mut SocketAddress) -> Result<u64, crate::Error>;
}
pub struct FreeSocketWrapper<T>(pub T);
impl<T> crate::objects::Object for FreeSocketWrapper<T>
where
	T: FreeSocket
{
	fn class(&self) -> u16 { crate::values::CLASS_FREESOCKET }
	fn as_any(&self) -> &dyn core::any::Any { self }
	fn try_clone(&self) -> Option<u32> {
		match self.0.try_clone() {
		Some(v) => Some(crate::objects::new_object(FreeSocketWrapper(v))),
		None => None,
		}
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64, crate::Error> {
		match call
		{
		crate::values::NET_FREESOCK_SENDTO => {
			let data: Freeze<[u8]> = args.get()?;
			let addr: Freeze<SocketAddress> = args.get()?;
			self.0.send_to(&data, &addr)
			},
		crate::values::NET_FREESOCK_RECVFROM => {
			let mut data: FreezeMut<[u8]> = args.get()?;
			let mut addr: FreezeMut<SocketAddress> = args.get()?;
			self.0.recv_from(&mut data, &mut addr)
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