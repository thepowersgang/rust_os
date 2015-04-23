
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
Reading from a storage device (for example) will return a boxed one of these structures, which can be handed to the async wait method.

Async's wait method will handle asking each "channel" to wait, and sleep on the returned primitive waiter reference.

```rust
enum WaitState<'dev>
{
	Acquire(async::Waiter),
	IoActive(async::HeldMutex<'dev,AtaRegs>, async::Waiter),
}
struct AtaWaiter<'dev>
{
	dev: &'dev mut AtaController,
	state: WaitState<'dev>,
}

impl<'a> WaiterState for AtaWaiter<'a>
{
	// 'wait' - Should return a waiter reference, and often does the required work to start the wait
	fn wait(&mut self) -> &mut Waiter
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
	fn wait_complete(&mut self) -> bool
	{
		self.state = {
			match self.state
			{
			WaitState::Acquire(ref mut waiter) => WaitState::IoActive(waiter.take_lock(), async::Waiter::new_none()),
			WaitState::IoActive(ref _lh, ref _waiter) => return true,
			}
			};
		return false;
	}
}
```
