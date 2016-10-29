// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/irqs.rs
//! Core IRQ Abstraction
use prelude::*;
use core::sync::atomic::AtomicBool;
use arch::sync::Spinlock;
use arch::interrupts;
use lib::{VecMap};
use lib::mem::Arc;

/// A handle for an IRQ binding that pokes an async event when the IRQ fires
pub struct EventHandle
{
	_binding: BindingHandle,
	event: Arc<::async::event::Source>,
}
pub struct ObjectHandle( BindingHandle );

struct BindingHandle(u32, u32);

#[derive(Default)]
struct IRQBinding
{
	arch_handle: interrupts::IRQHandle,
	has_fired: AtomicBool,	// Set to true if the IRQ fires while the lock is held by this CPU
	//handlers: Spinlock<Queue<Handler>>,
	handlers: Spinlock<Vec<Box<FnMut()->bool + Send + 'static>>>,
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

static S_IRQ_WORKER_SIGNAL: ::lib::LazyStatic<::threads::SleepObject<'static>> = lazystatic_init!();
static S_IRQ_WORKER: ::lib::LazyStatic<::threads::WorkerThread> = lazystatic_init!();

pub fn init() {
	// SAFE: Called in a single-threaded context
	unsafe {
		S_IRQ_WORKER_SIGNAL.prep(|| ::threads::SleepObject::new("IRQ Worker"));
		S_IRQ_WORKER.prep(|| ::threads::WorkerThread::new("IRQ Worker", irq_worker));
	}
}

fn bind(num: u32, obj: Box<FnMut()->bool + Send>) -> BindingHandle
{	
	log_trace!("bind(num={}, obj={:?})", num, "TODO"/*obj*/);
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
	
	BindingHandle( num, index as u32 )
}
impl Drop for BindingHandle
{
	fn drop(&mut self)
	{
		todo!("Drop IRQ binding handle: IRQ {} idx {}", self.0, self.1);
	}
}

fn irq_worker()
{
	loop {
		S_IRQ_WORKER_SIGNAL.wait();
		for (irqnum,b) in S_IRQ_BINDINGS.lock().mapping.iter()
		{
			if b.has_fired.swap(false, ::core::sync::atomic::Ordering::Relaxed)
			{
				if let Some(mut lh) = b.handlers.try_lock_cpu() {
					for handler in &mut *lh {
						handler();
					}
				}
				log_trace!("irq_worker: IRQ{} fired", irqnum);
			}
		}
	}
}

/// Bind an event waiter to an interrupt
pub fn bind_event(num: u32) -> EventHandle
{
	let ev = Arc::new( ::async::event::Source::new() );
	EventHandle {
		event: ev.clone(),
		_binding: bind(num, Box::new(move || { ev.trigger(); true })),
		//_binding: bind(num, Box::new(HandlerEvent { event: ev })),
		}
}

pub fn bind_object(num: u32, obj: Box<FnMut()->bool + Send + 'static>) -> ObjectHandle
{
	ObjectHandle( bind(num, obj) )
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
		let context = &*rv as *const IRQBinding as *const ();
		rv.arch_handle = match interrupts::bind_gsi(num as usize, IRQBinding::handler_raw, context)
			{
			Ok(v) => v,
			Err(e) => panic!("Unable to bind handler to GSI {}: {:?}", num, e),
			};
		rv
	}
	
	fn handler_raw(info: *const ())
	{
		// SAFE: 'info' pointer should be an IRQBinding instance
		unsafe {
			let binding_ref = &*(info as *const IRQBinding);
			binding_ref.handle();
		}
	}
	#[tag_safe(irq)]
	fn handle(&self)
	{
		//log_trace!("handle() num={}", self.arch_handle.num());
		// The CPU owns the lock, so we don't care about ordering
		self.has_fired.store(true, ::core::sync::atomic::Ordering::Relaxed);
		
		S_IRQ_WORKER_SIGNAL.signal();
	}
}

impl EventHandle
{
	pub fn get_event(&self) -> &::async::event::Source
	{
		&*self.event
	}
}

