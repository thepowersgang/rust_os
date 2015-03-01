// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/mod.rs
///! Asynchronous IO and waiting support
use _common::*;
use core::cell::RefCell;
use core::atomic::{AtomicBool,ATOMIC_BOOL_INIT};
use lib::Queue;

pub use self::mutex::Mutex;
pub use self::timer::Timer;

pub mod mutex;
pub mod timer;

pub mod events
{
	pub type EventMask = u32;
}

/// A general-purpose wait event (when flag is set, waiters will be informed)
pub struct EventSource
{
	flag: AtomicBool,
	waiter: ::sync::mutex::Mutex<Option<::threads::SleepObjectRef>>
}

/// A wait queue
pub struct QueueSource
{
	waiters: ::sync::mutex::Mutex<Queue<::threads::SleepObjectRef>>,
}

pub enum Waiter<'a>
{
	None,
	Event(EventWait<'a>),
	Poll(Option< PollCb<'a> >),
}

pub type PollCb<'a> = RefCell<Box<for<'r> FnMut(Option<&'r mut Waiter<'a>>) -> bool + Send + 'a>>;

type EventCb<'a> = Box<for<'r> ::lib::thunk::Invoke<(&'r mut Waiter<'a>),()> + Send + 'a>;

pub struct EventWait<'a>
{
	source: Option<&'a EventSource>,
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

pub enum WaitError
{
	Timeout,
}

static s_event_none: EventSource = EventSource { flag: ATOMIC_BOOL_INIT, waiter: mutex_init!(None) };

impl EventSource
{
	pub fn new() -> EventSource
	{
		EventSource {
			flag: ATOMIC_BOOL_INIT,
			waiter: ::sync::mutex::Mutex::new(None),
		}
	}
	pub fn wait_on<'a, F: FnOnce(&mut Waiter) + Send + 'a>(&'a self, f: F) -> Waiter<'a>
	{
		Waiter::new_event(self, f)
	}
	pub fn trigger(&self)
	{
		self.flag.store(true, ::core::atomic::Ordering::Relaxed);
		self.waiter.lock().as_mut().map(|r| r.signal());
	}
}

impl QueueSource
{
	pub fn new() -> QueueSource
	{
		QueueSource {
			waiters: ::sync::mutex::Mutex::new(Queue::new()),
		}
	}
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
	pub fn new_none() -> Waiter<'a>
	{
		Waiter::None
	}
	pub fn new_event<'b, F: for<'r> FnOnce(&'r mut Waiter<'b>) + Send + 'b>(src: &'b EventSource, f: F) -> Waiter<'b>
	{
		Waiter::Event( EventWait {
			source: Some(src),
			callback: Some(box f as EventCb),
			} )
	}
	/// Create a new polling waiter
	///
	/// The passed closure is called in two different modes.
	/// 1. If the argument is `None`, it should return true iff the wait should terminate
	/// 1. If the argument is `Some(e)`, the wait was terminated and completion handlers should fire (optionally assigning a new
	///    waiter to the passed handle).
	pub fn new_poll<F: FnMut(Option<&mut Waiter<'a>>)->bool + Send + 'a>(f: F) -> Waiter<'a>
	{
		Waiter::Poll( Some(RefCell::new(box f)) )
	}

	pub fn is_valid(&self) -> bool
	{
		match *self
		{
		Waiter::None => true,
		Waiter::Event(ref i) => i.callback.is_some(),
		Waiter::Poll(ref c) => c.is_some(),
		}
	}
	
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
	
	fn poll(&self) -> bool
	{
		match *self
		{
		Waiter::None => true,
		Waiter::Event(ref i) => match i.source
			{
			Some(r) => r.flag.load(::core::atomic::Ordering::Relaxed),
			None => true
			},
		Waiter::Poll(ref c) => match *c
			{
			Some(ref cb) => {
				let mut b = cb.borrow_mut();
				let rb = &mut **b;
				rb(None)
				},
			None => true,
			},
		}
	}
	
	/// Returns false if binding was impossible
	fn bind_signal(&mut self, sleeper: &mut ::threads::SleepObject) -> bool
	{
		match *self
		{
		Waiter::None => true,
		Waiter::Event(ref i) => {
			match i.source
			{
			Some(r) => { *r.waiter.lock() = Some(sleeper.get_ref()) },
			None => {},
			}
			true
			},
		Waiter::Poll(ref c) => false,
		}
	}
	
	fn run_completion(&mut self)
	{
		match ::core::mem::replace(self, Waiter::None)
		{
		Waiter::None => {
			},
		Waiter::Event(mut i) => {
			let cb = i.callback.take().expect("EventWait::run_completion with callback None");
			cb.invoke(self);
			},
		Waiter::Poll(mut callback) => {
			let mut cb = callback.take().expect("Wait::run_completion with Poll callback None");
			cb.into_inner()(Some(self));
			}
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
		match self
		{
		&Waiter::None => write!(f, "Waiter::None"),
		&Waiter::Event(ref e) => match e.source
			{
			Some(ref s) => write!(f, "Waiter::Event({:p}))", s),
			None => write!(f, "Waiter::Event(None)"),
			},
		&Waiter::Poll(_) => write!(f, "Waiter::Poll(..)"),
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
pub fn wait_on_list(waiters: &mut [&mut Waiter])
{
	log_trace!("wait_on_list(waiters = {:?})", waiters);
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


