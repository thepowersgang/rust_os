// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/objects.rs
/// Userland "owned" objects
use prelude::*;

use sync::RwLock;
use stack_dst::StackDST;

/// A system-call object
pub trait Object: Send + Sync
{
	fn handle_syscall(&self, call: u16, args: &[usize]) -> u64;
}

type UserObject = RwLock<Option< StackDST<Object> >>;

/// Structure used as process-local list of objects
struct ProcessObjects
{
	// TODO: Use a FAR better collection for this, allowing cheap expansion of the list
	objs: Vec< UserObject >,
}

impl Default for ProcessObjects {
	fn default() -> ProcessObjects {
		const MAX_OBJECTS_PER_PROC: usize = 64;
		ProcessObjects {
			objs: Vec::from_fn(MAX_OBJECTS_PER_PROC, |_| RwLock::new(None)),
		}
	}
}
impl ProcessObjects {
	fn get(&self, idx: u32) -> Option<&UserObject> {
		self.objs.get(idx as usize)
	}
	fn iter(&self) -> ::core::slice::Iter<UserObject> {
		self.objs.iter()
	}
}

pub fn new_object<T: Object+'static>(val: T) -> u32
{
	let objs = ::threads::get_process_local::<ProcessObjects>();
	// Search unsynchronised through the list of objects
	for (i,ent) in objs.iter().enumerate()
	{
		// If a free slot is found,
		if ent.read().is_none() {
			// lock for writing then ensure that it is free
			let mut wh = ent.write();
			if wh.is_none() {
				*wh = Some(StackDST::new(val).expect("Object did not fit"));
				return i as u32;
			}
		}
	}
	!0
}

pub fn call_object(handle: u32, call: u16, args: &[usize]) -> u64
{
	let objs = ::threads::get_process_local::<ProcessObjects>();
	// Obtain reference/borrow to object (individually locked)
	if let Some(h) = objs.get(handle)
	{
		if let Some(ref obj) = *h.read()
		{
			return obj.handle_syscall(call, args);
		}
	}
	// Call method
	todo!("call_object(handle={},call={},args=...)", handle, call);
}

pub fn drop_object(handle: u32)
{
	todo!("drop_object(handle={})", handle);
}


