// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/objects.rs
//! Userland "owned" objects
use kernel::prelude::*;

use kernel::sync::{RwLock,Mutex};
use crate::args::Args;
use crate::values::FixedStr6;

use kernel::threads::get_process_local;

//pub type WaitHandle<'a> = ::stack_dst::ValueA<dyn ::core::future::Future<Output=u32> + 'a, [usize; 4]>;

/// A system-call object
pub trait Object: Send + Sync + ::core::any::Any
{
	fn type_name(&self) -> &str { type_name!(Self) }
	fn as_any(&self) -> &dyn Any;
	/// Object class code (values::CLASS_*)
	fn class(&self) -> u16;

	fn try_clone(&self) -> Option<u32>;

	/// Return: Return value or argument error
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64,super::Error>;
	/// NOTE: Implementors should always move out of `self` and drop the contents (the caller will forget)
	/// Return: Return value or argument error
	//fn handle_syscall_val(self, call: u16, args: &mut Args) -> Result<u64,super::Error>;
	fn handle_syscall_val(&mut self, call: u16, _args: &mut Args) -> Result<u64,super::Error> {
		crate::objects::object_has_no_such_method_val(self.type_name(), call)
	}

	/// Return: Number of wakeup events bound
	fn bind_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32;
	/// Return: Number of wakeup events fired
	fn clear_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32;

	// fn wait(&self, flags: u32) -> WaitHandle<'_> {
	// 	crate::objects::WaitHandle::new(async move { 0 }).map_err(|_|()).unwrap()
	// }
}
impl<T: Object> Object for Box<T> {
	fn type_name(&self) -> &str { (**self).type_name() }
	fn as_any(&self) -> &dyn Any { (**self).as_any() }
	fn class(&self) -> u16 { (**self).class() }
	fn try_clone(&self) -> Option<u32> {
		(**self).try_clone()
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64,super::Error> {
		(**self).handle_syscall_ref(call, args)
	}
	fn handle_syscall_val(&mut self, call: u16, args: &mut Args) -> Result<u64,super::Error> {
		// SAFE: Valid pointer, forgotten by caller
		let mut this: Box<T> = unsafe { ::core::ptr::read(&mut *self) };
		let rv = (*this).handle_syscall_val(call, args);
		//Box::shallow_drop(this);
		::core::mem::forget(*this);
		rv
	}
	fn bind_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
		(**self).bind_wait(flags, obj)
	}
	fn clear_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
		(**self).clear_wait(flags, obj)
	}
}
pub type ObjectAlloc = ::stack_dst::ValueA<dyn Object, [usize; 8]>;

struct UserObject
{
	data: ObjectAlloc,
}

impl UserObject {
	fn new<T: Object+'static>(v: T) -> Self {
		UserObject {
			data: match ::stack_dst::ValueA::new(v)
				{
				Ok(v) => v,
				Err(v) => {
					log_trace!("Object '{}' did not fit in StackDST {} > {}", type_name!(T), ::core::mem::size_of::<T>(), ::core::mem::size_of::<ObjectAlloc>());
					::stack_dst::ValueA::new(Box::new(v)).ok().unwrap()
					},
				},
		}
	}
}

type ObjectSlot = RwLock<Option< UserObject >>;

/// Structure used as process-local list of objects
struct ProcessObjects
{
	// TODO: Use a FAR better collection for this, allowing cheap expansion of the list
	objs: Vec< ObjectSlot >,

	given: Mutex< Vec<(FixedStr6, u16)> >,
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
				//given: Mutex::new( GivenObjects { next: 1, total: 1 } ),
				given: Mutex::new( Vec::new() ),
			};
		// Object 0 is fixed to be "this process" (and is not droppable)
		*ret.objs[0].get_mut() = Some(UserObject::new(crate::threads::CurProcess));
		ret
	}

	fn get(&self, idx: u32) -> Option<&ObjectSlot> {
		self.objs.get(idx as usize)
	}
	fn iter(&self) -> ::core::slice::Iter<ObjectSlot> {
		self.objs.iter()
	}

	fn with_object<O, F>(&self, handle: u32, fcn: F) -> Result< O, super::Error >
	where
		F: FnOnce(&dyn Object)->Result<O,super::Error> 
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
	fn with_object_val<O, F>(&self, handle: u32, fcn: F) -> Result<O, super::Error>
	where
		F: FnOnce(&mut dyn Object) -> Result<O, super::Error>
	{
		if let Some(h) = self.get(handle)
		{
			// Call method
			// NOTE: Move out of the collection before calling, to allow reusing the slot
			let v = h.write().take();
			if let Some(mut obj) = v {
				let name = obj.data.type_name();
				log_debug!("Object by-value #{}: {}", handle, name);
				let rv = fcn(&mut *obj.data);
				::core::mem::forget(obj);
				rv
			}
			else {
				return Err( super::Error::NoSuchObject(handle) )
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
			if let Some(mut lh) = h.try_write()
			{
				if let Some(obj) = lh.take() {
					let name = obj.data.type_name();
					log_debug!("Object removed #{}: {}", handle, name);
					Ok( obj.data )
				}
				else {
					Err( super::Error::NoSuchObject(handle) )
				}
			}
			else
			{
				Err( super::Error::MoveContention )
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
					let name = wh.as_ref().unwrap().data.type_name();
					log_debug!("Object created #{}: {}", i, name);
					return Ok(i as u32);
				}
			}
		}
		log_debug!("No space");
		Err(super::Error::TooManyObjects)
	}

	fn push_given(&self, handle: u32, tag: &str)
	{
		log_debug!("Push {} = {}", tag, handle);
		let mut lh = self.given.lock();
		lh.push( (tag.into(), handle as u16) );
	}

	fn pop_given(&self, tag: &str) -> Option<u32> {
		let mut lh = self.given.lock();
		match lh.iter().position(|e| &e.0[..] == tag)
		{
		Some(i) => Some( lh.remove(i).1 as u32 ),
		None => None,
		}
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
	log_debug!("new_object() - size_of {} = {}", type_name!(T), ::core::mem::size_of::<T>());
	get_process_local::<ProcessObjects>().find_and_fill_slot(|| UserObject::new(val)).unwrap_or(!0)
}

/// Startup: Pushes the specified index as an unclaimed object
pub fn push_as_unclaimed(tag: &str, handle: u32) {
	let objs = get_process_local::<ProcessObjects>();
	objs.push_given(handle, tag);
}

/// Grab an unclaimed object (checking the class)
pub fn get_unclaimed(class: u16, tag: &str) -> u64
{
	super::from_result( get_unclaimed_int(class, tag) )
}
fn get_unclaimed_int(class: u16, tag: &str) -> Result<u32,u32>
{
	let objs = get_process_local::<ProcessObjects>();

	if let Some(id) = objs.pop_given(tag) {
		let slot = match objs.get(id)
			{
			Some(v) => v,
			None => {
				log_notice!("QUIRK - Object index popped ({}) wasn't in list", id);
				return Err(0x1_0000);
				},
			};
		let mut lh = slot.write();
		if lh.is_none() {
			log_notice!("QUIRK - Object index popped ({}) wasn't populated (already freed)", id);
			Err(0x1_0000)
		}
		else if lh.as_ref().map(|x| x.data.class()) == Some(class) {
			log_debug!("get_unclaimed({}) - Returned {}", tag, id);
			Ok(id as u32)
		}
		else {
			let real_class = {
				let o = lh.as_ref().unwrap();
				log_notice!("get_unclaimed({}) - Object popped ({}) was the wrong class (wanted {} [{} ?], but got {} [{} {}])",
					tag, id,
					class, crate::values::get_class_name(class),
					o.data.class(), crate::values::get_class_name(o.data.class()), o.data.type_name()
					);
				o.data.class()
				};
			*lh = None;
			Err(real_class as u32)
		}
	}
	else {
		log_notice!("get_unclaimed({}) - No object in queue", tag);
		Err(0x1_0000)
	}
}

#[inline(never)]
pub fn call_object_ref(handle: u32, call: u16, args: &mut Args) -> Result<u64,super::Error>
{
	// Obtain reference/borrow to object (individually locked), and call the syscall on it
	get_process_local::<ProcessObjects>().with_object(handle, |obj| {
		//log_trace!("#{} {} Call Ref {} - args={:?}", handle, obj.type_name(), call, args);
		obj.handle_syscall_ref(call, args)
		})
}
#[inline(never)]
pub fn call_object_val(handle: u32, call: u16, args: &mut Args) -> Result<u64,super::Error>
{
	// Obtain reference/borrow to object (individually locked), and call the syscall on it
	get_process_local::<ProcessObjects>().with_object_val(handle, |obj| {
		//log_trace!("#{} {} Call Val {} - args={:?}", handle, obj.type_name(), call-0x400, args);
		obj.handle_syscall_val(call, args)
		})
}
#[inline(never)]
pub fn get_class(handle: u32) -> Result<u64, super::Error>
{
	get_process_local::<ProcessObjects>().with_object(handle, |obj| Ok(obj.class() as u64))
}
pub fn clone_object(handle: u32) -> Result<u64, super::Error> {
	get_process_local::<ProcessObjects>().with_object(handle, |obj| {
		match obj.try_clone()
		{
		Some(v) => Ok(v as u64),
		None => Ok(!0),
		}
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

/// Give the target process the object specified by `handle`
pub fn give_object(target: &::kernel::threads::ProcessHandle, tag: &str, handle: u32) -> Result<(),super::Error> {
	log_trace!("give_object(target={:?}, handle={:?})", target, handle);
	let target_list = target.get_process_local_alloc::<ProcessObjects>();
	let obj = get_process_local::<ProcessObjects>().take_object(handle)?;
	let class_id = obj.class();
	let id = target_list.find_and_fill_slot(|| UserObject { data: obj })?;
	
	log_debug!("- Giving object {} ({} {}) as '{}' to {:?} (handle {})",
		handle, class_id, crate::values::get_class_name(class_id),
		tag, target, id
		);
	target_list.push_given( id, tag );

	Ok( () )
}

pub fn take_object<T: Object+'static>(handle: u32) -> Result<T,super::Error> {
	let obj = get_process_local::<ProcessObjects>().take_object(handle)?;
	// SAFE: ptr::read is called on a pointer to a value that is subsequently forgotten
	unsafe {
		let rv = {
			let r = obj.as_any().downcast_ref::<T>().expect("Object was not expected type (TODO: Proper error)");
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
		match get_process_local::<ProcessObjects>().take_object(handle)
		{
		Ok(v) => {
			log_debug!("Object dropped #{}: {}", handle, v.type_name());
			::core::mem::drop( v );
			},
		Err(_) => {}
		}
	}
}




pub fn object_has_no_such_method_val(name: &str, call: u16) -> Result<u64,crate::Error> {
	if call < 0x400 {
		panic!("BUGCHECK: Call ID {:#x} < 0x400 invoked by-value call on {}", call, name);
	}
	else {
		log_notice!("User called non-existent mathod (by-value) {} on {}", call-0x400, name);
	}
	Err( crate::Error::UnknownCall )
}
pub fn object_has_no_such_method_ref(name: &str, call: u16) -> Result<u64,crate::Error> {
	if call >= 0x400 {
		panic!("BUGCHECK: Call ID {:#x} > 0x400 invoked by-ref call on {}", call, name);
	}
	else {
		log_notice!("User called non-existent mathod (by-ref) {} on {}", call, name);
	}
	Err( crate::Error::UnknownCall )
}

