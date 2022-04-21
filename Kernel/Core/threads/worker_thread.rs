// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads/worker_thread.rs
//! Management of kernel worker threads (short or long)
#[allow(unused_imports)]
use crate::prelude::*;


pub struct WorkerThread(super::thread::ThreadHandle);

impl WorkerThread
{
	#[allow(dead_code)]
	/// Construct a new worker thread
	pub fn new<F: FnOnce()+Send+'static>(name: &str, fcn: F) -> WorkerThread
	{
		let handle = super::thread::ThreadHandle::new(name, fcn, super::S_PID0.clone());
		WorkerThread(handle)
	}

	// TODO: Allow the worker to return a value?
	pub fn wait(&self) -> Result<(),()>
	{
		if ! cfg!(feature="test") {
			todo!("Wait for worker thread to terminate");
		}
		else {
			Ok( () )
		}
	}
}


