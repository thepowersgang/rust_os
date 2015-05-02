// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/irqs.rs
//! Core IRQ Abstraction
use _common::*;
use core::atomic::AtomicBool;
use arch::sync::Spinlock;
use arch::interrupts;
use lib::{VecMap};
use lib::mem::Arc;

/// A handle for an IRQ binding that pokes an async event when the IRQ fires
pub struct EventHandle
{
	num: u32,
	index: usize,
	event: Arc<::async::event::Source>,
}
pub struct ObjectHandle<T: Handler>
{
	num: u32,
	index: usize,
	_pd: ::core::marker::PhantomData<T>,
}

struct HandlerEvent
{
	//index: usize,
	event: Arc<::async::event::Source>
}
pub trait Handler: Send + 'static
{
	//fn get_idx(&self) -> usize;
	fn handle(&mut self) -> bool;
}

#[derive(Default)]
struct IRQBinding
{
	arch_handle: interrupts::IRQHandle,
	has_fired: AtomicBool,	// Set to true if the IRQ fires while the lock is held by this CPU
	handlers: Spinlock<Vec<Box<Handler>>>,	// TODO: When DST functions are avaliable, change to Queue<Handler>
}

struct Bindings
{
	mapping: VecMap<u32, Box<IRQBinding>>,
	next_index: usize,
}

// Notes:
// - Store a map of interrupt IDs against 
// - Hand out 'Handle' structures containing a pointer to the handler on that queue?
// - Per IRQ queue of
/// Map of IRQ numbers to core's dispatcher bindings. Bindings are boxed so the address is known in the constructor
static S_IRQ_BINDINGS: ::sync::mutex::LazyMutex<Bindings> = lazymutex_init!();

fn bind(num: u32, obj: Box<Handler>) -> usize
{
	// 1. (if not already) bind a handler on the architecture's handlers
	let mut map_lh = S_IRQ_BINDINGS.lock_init(|| Bindings { mapping: VecMap::new(), next_index: 0 });
	let index = map_lh.next_index;
	map_lh.next_index += 1;
	let binding = match map_lh.mapping.entry(num)
		{
		::lib::vec_map::Entry::Occupied(e) => e.into_mut(),
		// - Vacant, create new binding (pokes arch IRQ clode)
		::lib::vec_map::Entry::Vacant(e) => e.insert( IRQBinding::new_boxed(num) ),
		};
	// 2. Add this handler to the meta-handler
	binding.handlers.lock().push( obj );
	
	index
}

/// Bind an event waiter to an interrupt
pub fn bind_event(num: u32) -> EventHandle
{
	let ev = Arc::new( ::async::event::Source::new() );
	EventHandle {
		num: num,
		index: bind(num, Box::new(HandlerEvent { event: ev.clone() }) as Box<Handler>),
		event: ev,
		}
}

pub fn bind_object<T: Handler>(num: u32, obj: Box<T>) -> ObjectHandle<T>
{
	todo!("bind_object<{}>(num={})", type_name!(T), num);
}

impl IRQBinding
{
	fn new_boxed(num: u32) -> Box<IRQBinding>
	{
		let mut rv = Box::new( IRQBinding::default());
		assert!(num < 256, "{} < 256 failed", num);
		// TODO: Use a better function, needs to handle IRQ routing etc.
		// - In theory, the IRQ num shouldn't be a u32, instead be an opaque IRQ index
		//   that the arch code understands (e.g. value for PciLineA that gets translated into an IOAPIC line)
		rv.arch_handle = interrupts::bind_gsi(
			num as usize, IRQBinding::handler_raw, &*rv as *const IRQBinding as *const ()
			).unwrap();
		rv
	}
	
	fn handler_raw(info: *const ())
	{
		unsafe {
			let binding_ref = &*(info as *const IRQBinding);
			binding_ref.handle();
		}
	}
	fn handle(&self)
	{
		// If the current CPU owns the queue lock, don't do processing here
		if let Some(mut lh) = self.handlers.try_lock_cpu()
		{
			log_debug!("Firing IRQ handlers");
			// Otherwise, lock the handlers list and run them
			// - Should not cause a race condition, as the current CPU shouldn't be doing funny stuff
			// - POSSIBLE : Reach here, other IRQ which causes changes to this IRQ's data?
			for handler in &mut *lh
			{
				// TODO: Call handlers
				handler.handle();
			}
		}
		// Instead, mark the interrupt as having fired and let it call handlers later
		else
		{
			log_debug!("Deferring IRQ handler fire");
			// The CPU owns the lock, so we don't care about ordering
			self.has_fired.store(true, ::core::atomic::Ordering::Relaxed);
		}
			
	}
}

impl Handler for HandlerEvent
{
	//fn get_idx(&self) -> usize { self.index }
	fn handle(&mut self) -> bool {
		self.event.trigger();
		true
	}
}

impl EventHandle
{
	pub fn get_event(&self) -> &::async::event::Source
	{
		&*self.event
	}
}

impl ::core::ops::Drop for EventHandle
{
	fn drop(&mut self)
	{
		panic!("TODO: EventHandle::drop() num={}, idx={}", self.num, self.index);
		// - Locate interrupt handler block
		// - Locate this index within that list
		// - Remove from list
	}
}

