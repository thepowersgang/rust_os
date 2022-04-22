// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/mod.rs
/*! Asynchronous IO and waiting support.

 The Tifflin asynch IO model is based around waiter handlers that contain sufficient information
 to either sleep the thread, or poll for a condition.

 The `wait_on_list` function is the kernel's core implementation of multiple waiters. Userland uses syscalls/threads::wait
*/
use crate::prelude::*;

//pub use self::mutex::Mutex;

//pub mod mutex;
pub mod timer;
pub mod event;
pub mod queue;
pub mod sequential_queue;
pub mod poll;

pub use self::queue::Queue;

/// A boxed ResultWaiter that resturns a Result
pub type BoxAsyncResult<'a,T,E> = Box<dyn ResultWaiter<Result=Result<T,E>>+'a>;

/// Trait for primitive waiters
///
/// Primitive waiters are the lowest level async objects, mostly provided by this module
pub trait PrimitiveWaiter:
	::core::fmt::Debug
{
	/// Return true if the waiter is already complete (and signalled)
	fn is_complete(&self) -> bool;
	
	/// Polls the waiter, returning true if the event has triggered
	fn poll(&self) -> bool;
	/// Runs the completion handler
	fn run_completion(&mut self);
	/// Binds this waiter to signal the provided sleep object
	/// 
	/// Called before the completion handler
	///
	/// Returns true if the sleep object was sucessfully registered, false to force polling (e.g. if already complete)
	fn bind_signal(&mut self, sleeper: &mut crate::threads::SleepObject) -> bool;
	
	/// Unbind waiters from this sleep object
	fn unbind_signal(&mut self);
	
	/// 
	fn is_ready(&mut self) -> bool {
		if self.poll() {
			self.run_completion();
			true
		}
		else {
			false
		}
	}
}

#[derive(Debug)]
pub struct NullWaiter;
impl PrimitiveWaiter for NullWaiter {
	fn is_complete(&self) -> bool { true }
	fn poll(&self) -> bool { true }
	fn run_completion(&mut self) { }
	fn bind_signal(&mut self, _: &mut crate::threads::SleepObject) -> bool { panic!("NullWaiter::bind_signal") }
	fn unbind_signal(&mut self) { panic!("NullWaiter::unbind_signal") }
}

/// A more generic waiter object, that can handle state transitions
pub trait Waiter:
	::core::fmt::Debug
{
	/// Returns true if the waiter is completed (i.e. waiting will do nothing)
	fn is_complete(&self) -> bool;
	
	/// Request a primitive wait object
	fn get_waiter(&mut self) -> &mut dyn PrimitiveWaiter;
	/// Called when the wait returns
	///
	/// Return true to indicate that this waiter is complete
	fn complete(&mut self) -> bool;
}

/// A waiter that exposes access to a value upon completion
pub trait ResultWaiter:
	Waiter
{
	/// Return value once complete
	type Result;
	
	///
	fn get_result(&mut self) -> Option<Self::Result>;
	
	fn as_waiter(&mut self) -> &mut dyn Waiter;// { self }
}

/// A null result waiter, which returns the result of a simple closure when asked
pub struct NullResultWaiter<T, F: Fn()->T>(F,NullWaiter);
impl<T, F: Fn()->T> NullResultWaiter<T,F> {
	pub fn new(f: F) -> Self {
		NullResultWaiter(f, NullWaiter)
	}
}
impl<T, F: Fn()->T> ::core::fmt::Debug for NullResultWaiter<T,F> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "NullResultWaiter")
	}
}
impl<T, F: Fn()->T> Waiter for NullResultWaiter<T,F> {
	fn is_complete(&self) -> bool { true }
	fn get_waiter(&mut self) -> &mut dyn PrimitiveWaiter { &mut self.1 }
	fn complete(&mut self) -> bool { true }
}
impl<T, F: Fn()->T> ResultWaiter for NullResultWaiter<T,F> {
	type Result = F::Output;
	fn get_result(&mut self) -> Option<Self::Result> { Some( self.0() ) }
	fn as_waiter(&mut self) -> &mut dyn Waiter { self }
}

impl<T: PrimitiveWaiter> Waiter for T {
	fn is_complete(&self) -> bool {
		self.is_complete()
	}
	fn get_waiter(&mut self) -> &mut dyn PrimitiveWaiter {
		self
	}
	fn complete(&mut self) -> bool {
		true
	}
}

impl<'a> dyn Waiter+'a
{
	/// Wait on a single wait object
	pub fn wait(&mut self)
	{
		log_debug!("Waiting on {:?}", self);
		while !self.is_complete()
		{
			let completed = {
				let prim = self.get_waiter();
				crate::threads::SleepObject::with_new("wait_on_list", |obj_ref| {
					log_trace!("- bind");
					if prim.bind_signal( obj_ref ) {
						obj_ref.wait();
					}
					else {
						while !prim.poll() {
							// TODO: Take a nap
						}
					}
					prim.unbind_signal();
					log_trace!("- sleep over");
					});
				prim.is_ready()
				};
			// completed = This cycle is done, not everything?
			if completed {
				self.complete();
			}
		}
	}
}

impl<'a,T> dyn ResultWaiter<Result=T>+'a
{
	/// Wait for the waiter to complete, then return the result
	pub fn wait(&mut self) -> T
	{
		<dyn Waiter>::wait(self.as_waiter());
		self.get_result().expect("Waiter complete, but no result")
	}
}

/// Error type from wait_on_list
pub enum WaitError
{
	Timeout,
}

/// Wait on the provided list of Waiter trait objects
///
pub fn wait_on_list(waiters: &mut [&mut dyn Waiter], timeout: Option<u64>) -> Option<usize>
{
	log_trace!("wait_on_list(waiters = {:?}, timeout = {:?})", waiters, timeout);
	if waiters.len() == 0
	{
		panic!("wait_on_list - Nothing to wait on");
	}
	
	if timeout.is_some() {
		todo!("Support timeouts in wait_on_list");
	}
	
	// Wait on primitives from the waiters, returning the indexes of those that need a state advance
	
	// - If there are no incomplete waiters, return None
	if waiters.iter().filter(|x| !x.is_complete()).count() == 0 {
		return None;
	}
	
	// - Create an object for them to signal
	crate::threads::SleepObject::with_new("wait_on_list", |obj| {
		let force_poll = waiters.iter_mut()
			.filter( |x| !x.is_complete() )
			.fold(false, |v,x| v | !x.get_waiter().bind_signal(obj) )
			// ^ doesn't use .any() becuase of unbind_signal below
			;
		
		if force_poll
		{
			log_trace!("- Polling");
			let mut n_passes = 0;
			// While none of the active waiters returns true from poll()
			'outer: loop
			{
				for w in waiters.iter_mut()
				{
					if w.is_complete() {
					}
					else if w.get_waiter().poll() {
						break 'outer;
					}
					else {
					}
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
		
		for ent in waiters.iter_mut().filter(|x| !x.is_complete()) {
			ent.get_waiter().unbind_signal();
		}
		});
	
	// Run completion handlers (via .is_ready and .complete), counting the number of changed waiters
	let mut n_complete = 0;
	for ent in waiters.iter_mut().filter(|x| !x.is_complete())
	{
		if ent.get_waiter().is_ready() && ent.complete()
		{
			n_complete += 1;
		}
	}
	Some( n_complete )
}

pub struct FutureWrapper<F>
where
	F: ::core::future::Future
{
	inner: FutureWrapperInner<F>,
}
enum FutureWrapperInner<F>
where
	F: ::core::future::Future
{
	Running(::core::pin::Pin<Box<F>>),
	Complete(F::Output),
	Consumed,
}
impl<F> FutureWrapper<F>
where
	F: ::core::future::Future
{
	pub fn new(fut: F) -> Self
	{
		FutureWrapper {
			inner: FutureWrapperInner::Running(Box::pin(fut)),
			}
	}

	fn poll_with_waker(&mut self, waker: ::core::task::Waker)
	{
		match self.inner
		{
		FutureWrapperInner::Running(ref mut fut) => {
			let mut context = ::core::task::Context::from_waker(&waker);
			match fut.as_mut().poll(&mut context)
			{
			::core::task::Poll::Ready(rv) => {
				self.inner = FutureWrapperInner::Complete(rv);
				},
			::core::task::Poll::Pending => {},
			}
			},
		_ => {},
		}
	}
}
impl<F> ::core::fmt::Debug for FutureWrapper<F>
where
	F: ::core::future::Future
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		match self.inner
		{
		FutureWrapperInner::Running(_) => f.write_str("Running"),
		FutureWrapperInner::Complete(_) => f.write_str("Complete"),
		FutureWrapperInner::Consumed => f.write_str("Consumed"),
		}
	}
}
impl<F> PrimitiveWaiter for FutureWrapper<F>
where
	F: ::core::future::Future
{
	fn is_complete(&self) -> bool {
		match self.inner
		{
		FutureWrapperInner::Running(_) => false,
		_ => true,
		}
	}
	fn poll(&self) -> bool {
		PrimitiveWaiter::is_complete(self)
	}
	fn run_completion(&mut self) {
	}
	fn bind_signal(&mut self, sleeper: &mut crate::threads::SleepObject) -> bool {

		// NOTE: This function converts a SleepObjectRef into a RawWaker (they both have a single pointer of data)

		fn sleep_object_raw_waker(so: &crate::threads::SleepObject) -> ::core::task::RawWaker
		{
			unsafe fn rw_clone(raw_self: *const ()) -> ::core::task::RawWaker
			{
				// Cast pointer to the input pointer into a pointer to a SleepObjectRef
				let sor = &*(&raw_self as *const _ as *const crate::threads::SleepObjectRef);
				sleep_object_raw_waker(sor)
			}
			unsafe fn rw_wake(raw_self: *const ())
			{
				let sor: crate::threads::SleepObjectRef = ::core::mem::transmute(raw_self);
				sor.signal();
			}
			unsafe fn rw_wake_by_ref(raw_self: *const ())
			{
				let sor = &*(&raw_self as *const _ as *const crate::threads::SleepObjectRef);
				sor.signal();
			}
			unsafe fn rw_drop(raw_self: *const ())
			{
				let sor: crate::threads::SleepObjectRef = ::core::mem::transmute(raw_self);
				::core::mem::drop(sor);
			}
			static VTABLE: ::core::task::RawWakerVTable = ::core::task::RawWakerVTable::new(
				/*clone:*/ rw_clone,
				/*wake:*/ rw_wake,
				/*wake_by_ref:*/ rw_wake_by_ref,
				/*drop:*/ rw_drop,
				);
			// SAFE: Storing the single-pointer SleepObjectRef in a pointer
			unsafe {
				::core::task::RawWaker::new(::core::mem::transmute(so.get_ref()), &VTABLE)
			}
		}

		// Call future poll with the provided sleep object
		// - Expose a SleepObjectRef from Context
		// SAFE: Doesn't outlive the sleeper
		let waker = unsafe { ::core::task::Waker::from_raw(sleep_object_raw_waker(sleeper)) };
		self.poll_with_waker(waker);
		true	// binding always suceeds
	}
	fn unbind_signal(&mut self) {
		// Poll again (with a null waker)
		self.poll_with_waker( crate::futures::null_waker() );
	}
}
impl<F> ResultWaiter for FutureWrapper<F>
where
	F: ::core::future::Future
{
	type Result = F::Output;

	fn get_result(&mut self) -> Option<Self::Result> {
		match self.inner
		{
		FutureWrapperInner::Running(_) => None,
		FutureWrapperInner::Complete(_) => {
			let v = match ::core::mem::replace(&mut self.inner, FutureWrapperInner::Consumed)
				{
				FutureWrapperInner::Complete(v) => v,
				_ => unreachable!(),
				};
			Some(v)
			},
		FutureWrapperInner::Consumed => None,
		}
	}
	
	fn as_waiter(&mut self) -> &mut dyn Waiter { self }
}

