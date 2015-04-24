// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/mod.rs
/*! Asynchronous IO and waiting support.

 The Tifflin asynch IO model is based around waiter handlers that contain sufficient information
 to either sleep the thread, or poll for a condition.
*/
use _common::*;
use lib::Queue;

pub use self::mutex::Mutex;
pub use self::timer::Timer;

pub mod mutex;
pub mod timer;
pub mod event;
pub mod queue;
pub mod poll;

/// Trait for primitive waiters
///
/// Primitive waiters are the lowest level async objects, mostly provided by this module
pub trait PrimitiveWaiter:
	::core::fmt::Debug
{
	fn is_ready(&mut self) -> bool {
		if self.poll() {
			self.run_completion();
			true
		}
		else {
			false
		}
	}
	fn poll(&self) -> bool;
	fn run_completion(&mut self);
	fn bind_signal(&mut self, sleeper: &mut ::threads::SleepObject) -> bool;
}

/// A more generic waiter object, that can handle state transitions
pub trait Waiter:
	::core::fmt::Debug
{
	fn is_complete(&self) -> bool;
	
	/// Request a primitive wait object
	fn get_waiter(&mut self) -> &mut PrimitiveWaiter;
	/// Called when the wait returns
	///
	/// Return true to indicate that this waiter is complete
	fn complete(&mut self) -> bool;
}


impl<T: PrimitiveWaiter> Waiter for T {
	fn is_complete(&self) -> bool {
		self.poll()
	}
	fn get_waiter(&mut self) -> &mut PrimitiveWaiter {
		self
	}
	fn complete(&mut self) -> bool {
		true
	}
}


impl<'a> Waiter+'a
{
	/// Wait on a single wait object
	pub fn wait(&mut self)
	{
		while !self.is_complete()
		{
			let completed = {
				let prim = self.get_waiter();
				let mut obj = ::threads::SleepObject::new("wait_on_list");
				if prim.bind_signal( &mut obj ) {
					obj.wait();
				}
				else {
					todo!("Poll in Waiter::wait()");
				}
				prim.is_ready()
				};
			if completed {
				self.complete();
			}
		}
	}
}

/// Error type from wait_on_list
pub enum WaitError
{
	Timeout,
}


pub fn wait_on_list(waiters: &mut [&mut Waiter], timeout: Option<u64>) -> Option<usize>
{
	log_trace!("wait_on_list(waiters = {:?}, timeout = {:?})", waiters, timeout);
	if waiters.len() == 0
	{
		panic!("wait_on_list - Nothing to wait on");
	}
	
	// Wait on primitives from the waiters, returning the indexes of those that need a state advance
	let new_completions: Vec<usize> = {
		// 
		let mut prim_waiters: Vec<_> = waiters.iter_mut()
			.enumerate()	// Tag with index
			.filter( |&(_,ref x)| !x.is_complete() )	// Eliminate complete
			.map( |(i,x)| (i, x.get_waiter()) )	// Obtain waiter
			.collect();
		
		// - If there are no incomplete waiters, return None
		if prim_waiters.len() == 0 {
			return None;
		}
		
		// - Create an object for them to signal
		let mut obj = ::threads::SleepObject::new("wait_on_list");
		let mut force_poll = false;
		for &mut (_,ref mut ent) in prim_waiters.iter_mut()
		{
			force_poll |= !ent.bind_signal( &mut obj );
		}
		
		if force_poll
		{
			log_trace!("- Polling");
			let mut n_passes = 0;
			'outer: loop
			{
				for &(_, ref ent) in prim_waiters.iter()
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
		prim_waiters.into_iter().filter_map( |(i,x)| if x.is_ready() { Some(i) } else { None }).collect()
		};
	
	let n_complete = new_completions.iter().filter( |&&i| waiters[i].complete() ).count();
	Some( n_complete )
}

// Note - List itself isn't modified, but needs to be &mut to get &mut to inners
/**
 * Wait on a set of Waiter objects. Returns when at least one of the waiters completes, or the timeout elapses
 *
 * If the timeout is None, this function can wait forever. If the timeout is Some(0), no wait occurs (but completion
 * handlers may fire).
 */
pub fn wait_on_primitives(waiters: &mut [&mut PrimitiveWaiter], timeout: Option<u64>)
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


