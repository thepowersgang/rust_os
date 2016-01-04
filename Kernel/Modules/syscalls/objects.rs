// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/objects.rs
//! Userland "owned" objects
use kernel::prelude::*;

use kernel::sync::RwLock;
use stack_dst::StackDST;

use kernel::threads::get_process_local;

/// A system-call object
pub trait Object: Send + Sync + ::core::marker::Reflect
{
	/// Object class code (values::CLASS_*)
	const CLASS: u16;
	fn type_name(&self) -> &str { type_name!(Self) }
	fn as_any(&self) -> &Any;
	fn class(&self) -> u16;
	/// Return: Return value or argument error
	fn handle_syscall(&self, call: u16, args: &[usize]) -> Result<u64,super::Error>;
	/// Return: Number of wakeup events bound
	fn bind_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32;
	/// Return: Number of wakeup events fired
	fn clear_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32;
}
impl<T: Object> Object for Box<T> {
	const CLASS: u16 = 0xFFFF;
	fn type_name(&self) -> &str { (**self).type_name() }
	fn as_any(&self) -> &Any { (**self).as_any() }
	fn class(&self) -> u16 { (**self).class() }
	fn handle_syscall(&self, call: u16, args: &[usize]) -> Result<u64,super::Error> {
		(**self).handle_syscall(call, args)
	}
	fn bind_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
		(**self).bind_wait(flags, obj)
	}
	fn clear_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
		(**self).clear_wait(flags, obj)
	}
}
pub type ObjectAlloc = StackDST<Object>;

struct UserObject
{
	unclaimed: bool,
	data: ObjectAlloc,
}

impl UserObject {
	fn new<T: Object+'static>(v: T) -> Self {
		UserObject {
			unclaimed: false,
			data: match StackDST::new(v)
				{
				Ok(v) => v,
				Err(v) => {
					log_trace!("Object '{}' did not fit in StackDST {} > {}", type_name!(T), ::core::mem::size_of::<T>(), ::core::mem::size_of::<StackDST<Object>>());
					StackDST::new(Box::new(v)).ok().unwrap()
					},
				},
		}
	}
	fn sent(obj: ObjectAlloc) -> Self {
		UserObject {
			unclaimed: true,
			data: obj,
		}
	}
}

type ObjectSlot = RwLock<Option< UserObject >>;

/// Structure used as process-local list of objects
struct ProcessObjects
{
	// TODO: Use a FAR better collection for this, allowing cheap expansion of the list
	objs: Vec< ObjectSlot >,
}

impl Default for ProcessObjects {
	fn default() -> ProcessObjects {
		ProcessObjects::new()
	}
}
impl ProcessObjects {
	/// Construct the initial ProcessObjects list
	pub fn new() -> ProcessObjects {
		const MAX_OBJECTS_PER_PROC: usize = 64;
		let mut ret = ProcessObjects {
				objs: Vec::from_fn(MAX_OBJECTS_PER_PROC, |_| RwLock::new(None)),
			};
		// Object 0 is fixed to be "this process" (and is not droppable)
		*ret.objs[0].write() = Some(UserObject::new(::threads::CurProcess));
		ret
	}

	fn get(&self, idx: u32) -> Option<&ObjectSlot> {
		self.objs.get(idx as usize)
	}
	fn iter(&self) -> ::core::slice::Iter<ObjectSlot> {
		self.objs.iter()
	}

	fn with_object<O, F: FnOnce(&Object)->Result<O,super::Error>>(&self, handle: u32, fcn: F) -> Result< O, super::Error >
	{
		if let Some(h) = self.get(handle)
		{
			// Call method
			if let Some(ref obj) = *h.read() {
				fcn(&*obj.data)
			}
			else {
				Err( super::Error::NoSuchObject(handle) )
			}
		}
		else {
			Err( super::Error::NoSuchObject(handle) )
		}
	}
	fn take_object(&self, handle: u32) -> Result<ObjectAlloc, super::Error>
	{
		if let Some(h) = self.get(handle)
		{
			// Call method
			if let Some(obj) = h.write().take() {
				Ok( obj.data )
			}
			else {
				Err( super::Error::NoSuchObject(handle) )
			}
		}
		else {
			Err( super::Error::NoSuchObject(handle) )
		}
	}

	fn find_and_fill_slot<F: FnOnce()->UserObject>(&self, fcn: F) -> Result<u32, super::Error> {
		for (i,ent) in self.iter().enumerate()
		{
			// If a free slot is found,
			if ent.read().is_none() {
				// lock for writing then ensure that it is free
				let mut wh = ent.write();
				if wh.is_none() {
					*wh = Some(fcn());
					log_debug!("Object {}: {}", i, wh.as_ref().unwrap().data.type_name());
					return Ok(i as u32);
				}
			}
		}
		log_debug!("No space");
		Err(super::Error::TooManyObjects)
	}
}
impl Drop for ProcessObjects {
	fn drop(&mut self)
	{
		//self.objs.sort_by(|a,b| );
	}
}

//pub fn new_object<T: Object+'static>(val: T) -> Result<u32, super::Error>
pub fn new_object<T: Object+'static>(val: T) -> u32
{
	//log_debug!("new_object<{}>", type_name!(T));
	get_process_local::<ProcessObjects>().find_and_fill_slot(|| UserObject::new(val)).unwrap_or(!0)
}

/// Grab the 'n'th unclaimed object of the specified class
pub fn get_unclaimed(class: u16, idx: usize) -> u64
{
	let objs = get_process_local::<ProcessObjects>();

	let mut cur_idx = 0;
	for (i, ent) in objs.iter().enumerate()
	{
		let found = if let Some(ref v) = *ent.read()
			{
				if v.data.class() == class && v.unclaimed {
					if cur_idx == idx {
						true
					}
					else {
						cur_idx += 1;
						false
					}
				}
				else {
					false
				}
			}
			else {
				false
			};
		if found
		{
			if let Some(ref mut v) = *ent.write()
			{
				if v.data.class() == class && v.unclaimed {
					v.unclaimed = false;
					return super::from_result::<u32,u32>( Ok(i as u32) );
				}
			}
			break;
		}
	}
	super::from_result::<u32,u32>( Err(0) )
}

#[inline(never)]
pub fn call_object(handle: u32, call: u16, args: &[usize]) -> Result<u64,super::Error>
{
	// Obtain reference/borrow to object (individually locked), and call the syscall on it
	get_process_local::<ProcessObjects>().with_object(handle, |obj| {
		log_trace!("#{} {} Call {} - args=[{:#x}]", handle, obj.type_name(), call, ::kernel::lib::FmtSlice(args));
		obj.handle_syscall(call, args)
		})
}

pub fn wait_on_object(handle: u32, mask: u32, sleeper: &mut ::kernel::threads::SleepObject) -> Result<u32,super::Error> {
	get_process_local::<ProcessObjects>().with_object(handle, |obj| {
		Ok( obj.bind_wait(mask, sleeper) )
		})
}
pub fn clear_wait(handle: u32, mask: u32, sleeper: &mut ::kernel::threads::SleepObject) -> Result<u32,super::Error> {
	get_process_local::<ProcessObjects>().with_object(handle, |obj| {
		Ok( obj.clear_wait(mask, sleeper) )
		})
}

/// Queue of objects to be delivered to a process
#[derive(Default)]
struct ObjectQueue
{
	event: ::kernel::async::event::Source,
	//event: ::kernel::async::Semaphore,
}

/// Give the target process the object specified by `handle`
pub fn give_object(target: &::kernel::threads::ProcessHandle, handle: u32) -> Result<(),super::Error> {
	let target_queue = target.get_process_local::<ObjectQueue>().expect("TODO: Handle no queue");
	let target_objs = target.get_process_local::<ProcessObjects>().expect("TODO: Handle no queue");
	if ! target_objs.iter().any(|x| x.read().is_none()) {
		return Err(super::Error::TooManyObjects);
	}
	log_debug!("give_object(target={:?}, handle={:?})", target, handle);
	let obj = try!(get_process_local::<ProcessObjects>().take_object(handle));
	try!( target_objs.find_and_fill_slot(|| UserObject::sent(obj)) );
	target_queue.event.trigger();
	Ok( () )
}
pub fn wait_for_obj(obj: &mut ::kernel::threads::SleepObject) {
	get_process_local::<ObjectQueue>().event.wait_upon(obj);
}
pub fn clear_wait_for_obj(obj: &mut ::kernel::threads::SleepObject) {
	get_process_local::<ObjectQueue>().event.clear_wait(obj);
}

pub fn take_object<T: Object+'static>(handle: u32) -> Result<T,super::Error> {
	let obj = try!(get_process_local::<ProcessObjects>().take_object(handle));
	// SAFE: ptr::read is called on a pointer to a value that is subsequently forgotten
	unsafe {
		let rv = {
			let r = obj.as_any().downcast_ref::<T>().expect("Object was not expected type (TODO: Proper error)");
			//let r = obj.downcast_ref::<T>().expect("Object was not expected type (TODO: Proper error)");
			::core::ptr::read(r)
			};
		::core::mem::forget(obj);
		Ok(rv)
	}
}

#[inline(never)]
pub fn drop_object(handle: u32)
{
	if handle == 0 {
		// Ignore, it's the "this process" object
	}
	else {
		::core::mem::drop( get_process_local::<ProcessObjects>().take_object(handle) );
	}
}


