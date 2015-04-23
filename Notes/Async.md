
Asynchronous IO (and other waiting)
===

Model 0: No IO-level async support
---

The simplest model would be to just not have low-level async IO support. Instead using worker threads and other event channels for when async IO is desired.

This basically means using the select/poll model of non-blocking IO (not strictly async)

Model 1: Waiter objects with boxed functions
---

Currently implemented model (at time of writing). Uses boxed closures contained within an enum to handle arbitary waits, but appears to suffer from lifetime interaction issues when starting a new wait from a wait callback. Also has problems with heavy use of allocations (on each transition, there is usually a free+allloc).

Model 2: Boxed state objects
---

A new model which will hopefully not suffer from lifetime issues, and won't churn allocations on transitions.

Each IO device creates a structure that implements 'async::WaiterState' to handle state transitions used for asynchronious IO.
Reading from a storage device (for example) will return a boxed trait object, allowing the user of the code to wait on the object.

Async's wait method will handle asking each "channel" to wait, and sleep on the returned primitive waiter reference.

Example: Within the ATA driver.
```rust
use kernel::async;

enum WaitState<'dev>
{
	Acquire(async::mutex::Waiter),
	IoActive(async::mutex::HeldMutex<'dev,AtaRegs>, async::event::Waiter),
}
struct AtaWaiter<'dev>
{
	dev: &'dev mut AtaController,
	state: WaitState<'dev>,
}

impl<'a> async::WaiterState for AtaWaiter<'a>
{
	// 'wait' - Should return a waiter reference, and often does the required work to start the wait
	fn wait(&mut self) -> &mut async::Waiter
	{
		match self.state
		{
		// Initial state: Acquire the register lock
		WaitState::Acquire(ref mut waiter) => {
			// TODO: Should this short-circuit if the lock can be acquired now?
			*waiter = self.dev.regs.async_lock();
			waiter
			},
		// Final state: Start IO and wait for it to complete
		WaitState::IoActive(ref mut lh, ref mut waiter) => {
			lh.start_dma( disk, blockidx, &dma_buffer, is_write, dma_regs );
			*waiter = self.dev.interrupt.event.wait();
			},
		}
	}
	
	// 'wait_complete' - Called when the waiter returned from wait is complete
	// - Returns true if this waiter has completed its wait
	fn wait_complete(&mut self) -> bool
	{
		// Update state if the match returns
		self.state = match self.state
			{
			// If the Acquire wait completed, switch to IoActive state
			WaitState::Acquire(ref mut waiter) => WaitState::IoActive(waiter.take_lock(), async::Waiter::new_none()),
			// And if IoActive completes, we're complete
			WaitState::IoActive(ref _lh, ref _waiter) => return true,
			};
		return false;
	}
}
```
