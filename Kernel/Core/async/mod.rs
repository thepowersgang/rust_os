// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/mod.rs
/*! Asynchronous IO and waiting support.

 The Tifflin asynch IO model is based around waiter handlers that contain sufficient information
 to either sleep the thread, or poll for a condition.
*/
use _common::*;
use core::cell::RefCell;
use core::atomic::{AtomicBool,ATOMIC_BOOL_INIT};
use lib::Queue;

pub use self::mutex::Mutex;
pub use self::timer::Timer;

pub mod mutex;
pub mod timer;

/// A general-purpose wait event (when flag is set, waiters will be informed)
///
/// Only a single object can wait on this event at one time
///
/// TODO: Determine the set/reset conditions on the wait flag.
pub struct EventSource
{
	flag: AtomicBool,
	waiter: ::sync::mutex::Mutex<Option<::threads::SleepObjectRef>>
}

/// A wait queue
///
/// Allows a list of threads to wait on a single object (e.g. a Mutex)
pub struct QueueSource
{
	waiters: ::sync::mutex::Mutex<Queue<::threads::SleepObjectRef>>,
}

/// A wait handle for any type of possible wait
pub struct Waiter<'a>(WaiterInt<'a>);

/// Internal waiter enum
enum WaiterInt<'a>
{
	None,
	Event(EventWait<'a>),
	Poll(Option< PollCb<'a> >),
}

/// Callback for a poll waiter
type PollCb<'a> = RefCell<Box<for<'r> FnMut(Option<&'r mut Waiter<'a>>) -> bool + Send + 'a>>;

/// Callback for an event waiter
type EventCb<'a> = Box<for<'r> ::lib::thunk::Invoke<(&'r mut Waiter<'a>),()> + Send + 'a>;

/// Event waiter
struct EventWait<'a>
{
	/// Event source
	source: Option<&'a EventSource>,
	/// Callback to call once the event becomes true
	callback: Option<EventCb<'a>>,
}


/// A handle returned by a read operation (re-borrows the target buffer)
pub struct ReadHandle<'buf,'src>
{
	buffer: &'buf [u8],
	waiter: Waiter<'src>,
}

/// A handle returned by a read operation (re-borrows the target buffer)
pub struct WriteHandle<'buf,'src>
{
	buffer: &'buf [u8],
	waiter: Waiter<'src>,
}

/// Error type from wait_on_list
pub enum WaitError
{
	Timeout,
}

static s_event_none: EventSource = EventSource { flag: ATOMIC_BOOL_INIT, waiter: mutex_init!(None) };

impl EventSource
{
	/// Create a new event source
	pub fn new() -> EventSource
	{
		EventSource {
			flag: ATOMIC_BOOL_INIT,
			waiter: ::sync::mutex::Mutex::new(None),
		}
	}
	/// Return a wait handle for this event source
	pub fn wait_on<'a, F: FnOnce(&mut Waiter) + Send + 'a>(&'a self, f: F) -> Waiter<'a>
	{
		Waiter(WaiterInt::Event( EventWait {
			source: Some(self),
			callback: Some(box f as EventCb),
			} ))
	}
	/// Raise the event (waking any attached waiter)
	pub fn trigger(&self)
	{
		self.flag.store(true, ::core::atomic::Ordering::Relaxed);
		self.waiter.lock().as_mut().map(|r| r.signal());
	}
}

impl QueueSource
{
	/// Create a new queue source
	pub fn new() -> QueueSource
	{
		QueueSource {
			waiters: ::sync::mutex::Mutex::new(Queue::new()),
		}
	}
	/// Create a waiter for this queue
	///
	/// The passed handler is called with None to poll the state.
	// TODO: Race conditions between 'QueueSource::wait_on' and 'wait_on_list'.
	pub fn wait_on<'a, F: FnMut(Option<&mut Waiter>) + Send + 'a>(&'a self, f: F) -> Waiter<'a>
	{
		// TODO: Requires a queue wait variant
		unimplemented!();
	}
	/// Wake a single waiting thread
	pub fn wake_one(&self) -> bool
	{
		let mut lh = self.waiters.lock();
		if let Some(waiter) = lh.pop()
		{
			waiter.signal();
			true
		}
		else
		{
			false
		}
	}
}

impl<'a> Waiter<'a>
{
	/// Create a new null waiter (that is always ready)
	pub fn new_none() -> Waiter<'a>
	{
		Waiter(WaiterInt::None)
	}
	/// Create a new polling waiter. (DISCOURAGED)
 	///
	/// This waiter is provided ONLY for low-level hardware which does not provide sufficient IRQ support, and needs to be polled.
	///
	/// The passed closure is called in two different modes.
	/// 1. If the argument is `None`, it should return true iff the wait should terminate
	/// 1. If the argument is `Some(e)`, the wait was terminated and completion handlers should fire (optionally assigning a new
	///    waiter to the passed handle).
	pub fn new_poll<F>(f: F) -> Waiter<'a>
	where
		F: FnMut(Option<&mut Waiter<'a>>)->bool + Send + 'a
	{
		Waiter( WaiterInt::Poll( Some(RefCell::new(box f)) ) )
	}
	
	/// Returns the ready status of the waiter, running the completion handle if ready
	pub fn is_ready(&mut self) -> bool
	{
		if self.poll()
		{
			self.run_completion();
			true
		}
		else
		{
			false
		}
	}
	
	/// Returns the ready status (doing no other processing)
	fn poll(&self) -> bool
	{
		match self.0
		{
		WaiterInt::None => true,
		WaiterInt::Event(ref i) => match i.source
			{
			Some(r) => r.flag.load(::core::atomic::Ordering::Relaxed),
			None => true
			},
		WaiterInt::Poll(ref c) => match *c
			{
			Some(ref cb) => {
				let mut b = cb.borrow_mut();
				let rb = &mut **b;
				// Call poll hander with 'None' to ask it to poll
				rb(None)
				},
			None => true,
			},
		}
	}
	
	/// Run completion handler, replacing the waiter with a null waiter
	///
	/// "self: &mut Self" is passed to the handler, allowing it to create a new event.
	fn run_completion(&mut self)
	{
		match ::core::mem::replace(&mut self.0, WaiterInt::None)
		{
		WaiterInt::None => {
			// Do nothing
			},
		WaiterInt::Event(mut i) => {
			let cb = i.callback.take().expect("EventWait::run_completion with callback None");
			cb.invoke(self);
			},
		WaiterInt::Poll(mut callback) => {
			let mut cb = callback.take().expect("Wait::run_completion with Poll callback None");
			// Pass 'Some(self)' to indicate completion 
			cb.into_inner()(Some(self));
			}
		}
	}
	
	/// Bind the waiter's source to the passed sleeper
	///
	/// Returns false if binding was impossible
	///
	/// TODO: Safety issues, what happens when an event doesn't fire and the sleeper is destroyed. No unbind as yet
	fn bind_signal(&mut self, sleeper: &mut ::threads::SleepObject) -> bool
	{
		match self.0
		{
		WaiterInt::None => true,
		WaiterInt::Event(ref i) => {
			match i.source
			{
			Some(r) => { *r.waiter.lock() = Some(sleeper.get_ref()) },
			None => {},
			}
			true
			},
		WaiterInt::Poll(ref c) => false,
		}
	}
	
	//// Call the provided function after the original callback
	//pub fn chain<F: FnOnce(&mut EventWait) + Send + 'a>(mut self, f: F) -> EventWait<'a>
	//{
	//	let cb = self.callback.take().unwrap();
	//	let newcb = box move |e: &mut EventWait<'a>| { cb.invoke(e); f(e); };
	//	EventWait {
	//		callback: Some(newcb),
	//		source: self.source,
	//	}
	//}
}

impl<'a> ::core::fmt::Debug for Waiter<'a>
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		match self.0
		{
		WaiterInt::None => write!(f, "WaiterInt::None"),
		WaiterInt::Event(ref e) => match e.source
			{
			Some(ref s) => write!(f, "WaiterInt::Event({:p}))", s),
			None => write!(f, "WaiterInt::Event(None)"),
			},
		WaiterInt::Poll(_) => write!(f, "WaiterInt::Poll(..)"),
		}
	}
}

impl<'o_b,'o_e> ReadHandle<'o_b, 'o_e>
{
	pub fn new<'b,'e>(dst: &'b [u8], w: Waiter<'e>) -> ReadHandle<'b,'e>
	{
		ReadHandle {
			buffer: dst,
			waiter: w,
		}
	}
}

impl<'o_b,'o_e> WriteHandle<'o_b, 'o_e>
{
	pub fn new<'b,'e>(dst: &'b [u8], w: Waiter<'e>) -> WriteHandle<'b,'e>
	{
		WriteHandle {
			buffer: dst,
			waiter: w,
		}
	}
}

// Note - List itself isn't modified, but needs to be &mut to get &mut to inners
/**
 * Wait on a set of Waiter objects. Returns when at least one of the waiters completes, or the timeout elapses
 *
 * If the timeout is None, this function can wait forever. If the timeout is Some(0), no wait occurs (but completion
 * handlers may fire).
 */
pub fn wait_on_list(waiters: &mut [&mut Waiter], timeout: Option<u64>)
{
	log_trace!("wait_on_list(waiters = {:?}, timeout = {:?})", waiters, timeout);
	if waiters.len() == 0
	{
		panic!("wait_on_list - Nothing to wait on");
	}
	//else if waiters.len() == 1
	//{
	//	// Only one item to wait on, explicitly wait
	//	waiters[0].wait()
	//}
	else
	{
		// Multiple waiters
		// - Create an object for them to signal
		let mut obj = ::threads::SleepObject::new("wait_on_list");
		let mut force_poll = false;
		for ent in waiters.iter_mut()
		{
			force_poll |= !ent.bind_signal( &mut obj );
		}
		
		if force_poll
		{
			log_trace!("- Polling");
			let mut n_passes = 0;
			'outer: loop
			{
				for ent in waiters.iter()
				{
					if ent.poll() { break 'outer; }
				}
				n_passes += 1;
				// TODO: Take a short nap
			}
			log_trace!("- Fire ({} passes)", n_passes);
		}
		else
		{
			// - Wait the current thread on that object
			log_trace!(" Sleeping");
			obj.wait();
		}
		
		// - When woken, run completion handlers on all completed waiters
		let mut n_complete = 0;
		for ent in waiters.iter_mut()
		{
			if ent.is_ready()
			{
				n_complete += 1;
			}
		}
	}
}


