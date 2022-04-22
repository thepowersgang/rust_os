// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/ipc_calls.rs
//! Userland interface to IPC channels
use crate::args::Args;
use ::kernel::memory::freeze::{Freeze,FreezeMut};
use ::core::sync::atomic::{AtomicU8,Ordering};
use crate::values::RpcMessage;

struct SyncChannel {
	// TODO: NonZero?
	ptr: *const SyncChannelBack,
	side_idx: u8,
}

unsafe impl Sync for SyncChannel {}
unsafe impl Send for SyncChannel {}

impl crate::objects::Object for SyncChannel
{
	fn class(&self) -> u16 { crate::values::CLASS_IPC_RPC }
	fn as_any(&self) -> &dyn core::any::Any { self }
	fn try_clone(&self) -> Option<u32> {
		None
		//Some( ::objects::new_object( Node(self.0.clone()) ) )
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64,crate::Error> {
		match call
		{
		crate::values::IPC_RPC_SEND => {
			let data: Freeze<crate::values::RpcMessage> = args.get()?;
			let obj: u32 = args.get()?;
			todo!("IPC_RPC_SEND({:p}, {})", &*data, obj);
			},
		crate::values::IPC_RPC_RECV => {
			let _data: FreezeMut<crate::values::RpcMessage> = args.get()?;

			if let Some(msg) = self.take_message()
			{
				todo!("IPC_RPC_RECV - Message present - {:?}", msg);
			}
			else
			{
				Ok( 0x1000 )
			}
			},
		_ => crate::objects::object_has_no_such_method_ref("ipc_calls::SyncChannel", call),
		}
	}
	fn handle_syscall_val(&mut self, call: u16, _args: &mut Args) -> Result<u64,crate::Error> {
		// SAFE: Valid pointer which is forgotten after call
		let _ = unsafe { ::core::ptr::read(self) };
		crate::objects::object_has_no_such_method_val("ipc_calls::SyncChannel", call)
	}
	fn bind_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
		let mut ret = 0;
		if flags & crate::values::EV_IPC_RPC_RECV != 0 {
			self.wait_upon(obj);
			ret |= crate::values::EV_IPC_RPC_RECV;
		}
		ret
	}
	fn clear_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
		let mut ret = 0;
		if flags & crate::values::EV_IPC_RPC_RECV != 0 {
			self.clear_wait(obj);
			if self.has_message() {
				ret += 1;
			}
		}
		ret
	}
}

pub fn new_pair() -> Result< (u32,u32), () >
{
	let (a_obj, b_obj) = SyncChannel::new_pair();

	let a = crate::objects::new_object(a_obj);
	if a == !0 {
		return Err( () );
	}

	let b = crate::objects::new_object(b_obj);
	if b == !0 {
		crate::objects::drop_object(a);
		return Err( () );
	}

	Ok( (a,b) )
}

#[derive(Default)]
struct SyncChannelBack
{
	dying_refs: AtomicU8,
	dead_refs: AtomicU8,
	sides: [ SyncChannelSide; 2 ],
}
#[derive(Default)]
struct SyncChannelSide
{
	message: ::kernel::sync::Spinlock<Option<RpcMessage>>,
	queue: ::kernel::user_async::Queue,
}

impl SyncChannel
{
	fn new_pair() -> (SyncChannel, SyncChannel) {
		// SAFE: Allocation is safe?
		let ptr = unsafe { ::kernel::memory::heap::alloc( SyncChannelBack::default() ) };

		(SyncChannel { ptr: ptr, side_idx: 0 }, SyncChannel { ptr: ptr, side_idx: 1 })
	}

	fn get_side(&self) -> &SyncChannelSide {
		// SAFE: Destructor ensures that pointer is valid until both are dead
		unsafe {
			&(*self.ptr).sides[self.side_idx as usize]
		}
	}

	pub fn wait_upon(&self, waiter: &mut ::kernel::threads::SleepObject) {
		self.get_side().queue.wait_upon(waiter);
	}
	pub fn clear_wait(&self, waiter: &mut ::kernel::threads::SleepObject) {
		self.get_side().queue.clear_wait(waiter);
	}

	pub fn has_message(&self) -> bool {
		self.get_side().message.lock().is_some()
	}
	pub fn take_message(&self) -> Option<RpcMessage> {
		self.get_side().message.lock().take()
	}
}

impl ::core::ops::Drop for SyncChannel {
	fn drop(&mut self) {
		// SAFE: Pointer is valid
		let should_free = unsafe {
			if (*self.ptr).dying_refs.fetch_or(1 << self.side_idx, Ordering::SeqCst) != 0 {
				// Other side is in shutdown or dead.
			}
			else {
			}

			(*self.ptr).dead_refs.fetch_or(1 << self.side_idx, Ordering::SeqCst) != 0
			};
		if should_free {
			todo!("Deallocate SyncChannel");
		}
	}
}

