// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads/worker_thread.rs
//! Management of kernel worker threads (short or long)
use prelude::*;


pub struct WorkerThread(super::thread::ThreadHandle);

impl WorkerThread
{
	#[allow(dead_code)]
	/// Construct a new worker thread
	pub fn new<F: FnOnce()+Send>(name: &str, fcn: F) -> WorkerThread
	{
		let handle = super::thread::ThreadHandle::new(name, fcn);
		WorkerThread(handle)
	}
}


