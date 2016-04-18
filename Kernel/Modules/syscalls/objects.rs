// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/objects.rs
//! Userland "owned" objects
use kernel::prelude::*;

use kernel::sync::{RwLock,Mutex};
use stack_dst::StackDST;
use args::Args;

use kernel::threads::get_process_local;

/// A system-call object
pub trait Object: Send + Sync + ::core::marker::Reflect
{
	/// Object class code (values::CLASS_*)
	const CLASS: u16;
	fn type_name(&self) -> &str { type_name!(Self) }
	fn as_any(&self) -> &Any;
	fn class(&self) -> u16;

	fn try_clone(&self) -> Option<u32>;

	/// Return: Return value or argument error
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64,super::Error>;
	/// Return: Return value or argument error
	//fn handle_syscall_val(self, call: u16, args: &mut Args) -> Result<u64,super::Error>;
	fn handle_syscall_val(&mut self, call: u16, _args: &mut Args) -> Result<u64,super::Error> {
		::objects::object_has_no_such_method_val(self.type_name(), call)
	}

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
	fn try_clone(&self) -> Option<u32> {
		(**self).try_clone()
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64,super::Error> {
		(**self).handle_syscall_ref(call, args)
	}
	fn handle_syscall_val(&mut self, call: u16, args: &mut Args) -> Result<u64,super::Error> {
		(**self).handle_syscall_val(call, args)
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
	data: ObjectAlloc,
}

impl UserObject {
	fn new<T: Object+'static>(v: T) -> Self {
		UserObject {
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
}

type ObjectSlot = RwLock<Option< UserObject >>;

/// Structure used as process-local list of objects
struct ProcessObjects
{
	// TODO: Use a FAR better collection for this, allowing cheap expansion of the list
	objs: Vec< ObjectSlot >,

	// TODO: Something lighter than a mutex? (could be an atomic)
	given: Mutex<GivenObjects>,
}
struct GivenObjects {
	next: u16,
	total: u16,
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
				given: Mutex::new( GivenObjects { next: 1, total: 1 } ),
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

	fn with_object<O, F>(&self, handle: u32, fcn: F) -> Result< O, super::Error >
	where
		F: FnOnce(&Object)->Result<O,super::Error> 
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
		F: FnOnce(&mut Object) -> Result<O, super::Error>
	{
		if let Some(h) = self.get(handle)
		{
			// Call method
			// NOTE: Move out of the collection before calling, to allow reusing the slot
			let v = h.write().take();
			if let Some(mut obj) = v {
				fcn(&mut *obj.data)
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
					log_debug!("Object created #{}: {}", i, wh.as_ref().unwrap().data.type_name());
					return Ok(i as u32);
				}
			}
		}
		log_debug!("No space");
		Err(super::Error::TooManyObjects)
	}

	fn push_given(&self, handle: u32) {
		let mut lh = self.given.lock();
		assert!( lh.next == 1 );
		assert!( lh.total as u32 == handle );
		assert!( handle < 0x10000 );
		lh.total += 1;
	}
	fn pop_given(&self) -> Option<u32> {
		let mut lh = self.given.lock();
		assert!(lh.next <= lh.total);
		if lh.next == lh.total {
			None
		}
		else {
			lh.next += 1;
			Some( (lh.next - 1) as u32 )
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
	//log_debug!("new_object<{}>", type_name!(T));
	get_process_local::<ProcessObjects>().find_and_fill_slot(|| UserObject::new(val)).unwrap_or(!0)
}

/// Grab an unclaimed object (checking the class)
pub fn get_unclaimed(class: u16) -> u64
{
	let objs = get_process_local::<ProcessObjects>();

	let rv = if let Some(id) = objs.pop_given() {
			let slot = match objs.get(id)
				{
				Some(v) => v,
				None => todo!("return error when object in queue doesn't exist (user may have dropped it)"),
				};
			let mut lh = slot.write();
			if lh.is_none() {
				log_notice!("Object didn't exist");
				Err(2)
			}
			else if lh.as_ref().map(|x| x.data.class()) == Some(class) {
				Ok(id as u32)
			}
			else {
				{
					let o = lh.as_ref().unwrap();
					log_notice!("Object was the wrong class (wanted {}, but got {} [{}])",
						class, o.data.class(), o.data.type_name());
				}
				*lh = None;
				Err(1)
			}
		}
		else {
			log_notice!("No object in queue");
			Err(0)
		};
	super::from_result::<u32,u32>( rv )
}

#[inline(never)]
pub fn call_object_ref(handle: u32, call: u16, args: &mut Args) -> Result<u64,super::Error>
{
	// Obtain reference/borrow to object (individually locked), and call the syscall on it
	get_process_local::<ProcessObjects>().with_object(handle, |obj| {
		log_trace!("#{} {} Call Ref {} - args={:?}", handle, obj.type_name(), call, args);
		obj.handle_syscall_ref(call, args)
		})
}
#[inline(never)]
pub fn call_object_val(handle: u32, call: u16, args: &mut Args) -> Result<u64,super::Error>
{
	// Obtain reference/borrow to object (individually locked), and call the syscall on it
	get_process_local::<ProcessObjects>().with_object_val(handle, |obj| {
		log_trace!("#{} {} Call Val {} - args={:?}", handle, obj.type_name(), call-0x400, args);
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
pub fn give_object(target: &::kernel::threads::ProcessHandle, handle: u32) -> Result<(),super::Error> {
	log_debug!("give_object(target={:?}, handle={:?})", target, handle);
	let target_list = target.get_process_local_alloc::<ProcessObjects>();
	let obj = try!(get_process_local::<ProcessObjects>().take_object(handle));
	let id = try!( target_list.find_and_fill_slot(|| UserObject { data: obj }) );
	
	target_list.push_given( id );

	Ok( () )
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




pub fn object_has_no_such_method_val(name: &str, call: u16) -> Result<u64,::Error> {
	if call < 0x400 {
		panic!("BUGCHECK: Call ID {:#x} < 0x400 invoked by-value call on {}", call, name);
	}
	else {
		log_notice!("User called non-existent mathod (by-value) {} on {}", call-0x400, name);
	}
	Err( ::Error::UnknownCall )
}
pub fn object_has_no_such_method_ref(name: &str, call: u16) -> Result<u64,::Error> {
	if call >= 0x400 {
		panic!("BUGCHECK: Call ID {:#x} > 0x400 invoked by-ref call on {}", call, name);
	}
	else {
		log_notice!("User called non-existent mathod (by-ref) {} on {}", call, name);
	}
	Err( ::Error::UnknownCall )
}

