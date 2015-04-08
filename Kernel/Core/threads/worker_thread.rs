// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads/worker_thread.rs
//! Management of kernel worker threads (short or long)
use _common::*;


pub struct WorkerThread(super::thread::ThreadHandle);

impl WorkerThread
{
	pub fn new<F: FnOnce()+Send>(fcn: F) -> WorkerThread
	{
		let handle = super::thread::ThreadHandle::new("worker", fcn);
		WorkerThread(handle)
	}
}


