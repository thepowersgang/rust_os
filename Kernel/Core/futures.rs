// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/futures.rs
//! Helpers for standard library futures/async
use core::task;
//use core::pin::Pin;

pub mod flag;
pub mod condvar;
pub mod mutex;
/// Helper to wait on multiple futures at once
pub mod join;

mod simple_waiter;
use self::simple_waiter::SimpleWaiter;

pub use self::condvar::Condvar;
pub use self::mutex::Mutex;

mod helpers {
	mod waker_queue;
	pub use self::waker_queue::WakerQueue;
}

/// Wait on two futures, returning only one result
pub fn join_one<F1, F2>(a: F1, b: F2) -> join::JoinOne<F1,F2> {
	join::JoinOne::new(a, b)
}

/// Block on a single future
pub fn block_on<F: ::core::future::Future>(mut f: F) -> F::Output {
	// SAFE: The memory doesn't move after this pin.
	let mut f = unsafe { ::core::pin::Pin::new_unchecked(&mut f) };
	runner(|c| {
		match f.as_mut().poll(c)
		{
		task::Poll::Ready(v) => Some(v),
		task::Poll::Pending => None,
		}
	})
}

static TIME_WAKEUPS: crate::sync::Mutex<helpers::WakerQueue> = crate::sync::Mutex::new( helpers::WakerQueue::new() );

pub(super) fn time_tick() {
	TIME_WAKEUPS.lock().wake_all();
}

/// Sleep as a future for a given number of milisecond
pub fn msleep(ms: usize) -> impl core::future::Future<Output=()> {
	struct Sleep(u64);
	impl core::future::Future for Sleep {
		type Output = ();
		fn poll(self: core::pin::Pin<&mut Self>, cx: &mut task::Context) -> task::Poll<Self::Output> {
			if self.0 < crate::time::ticks() {
				task::Poll::Ready( () )
			}
			else {
				// Set the next wakeup
				TIME_WAKEUPS.lock().push(cx.waker());
				crate::time::request_interrupt(self.0);
				task::Poll::Pending
			}
		}
	}
	Sleep(crate::time::ticks() + ms as u64)
}

/// Create a waker handle that does nothing
pub fn null_waker() -> task::Waker
{
	fn rw_clone(_: *const ()) -> task::RawWaker {
		task::RawWaker::new(1 as *const (), &VTABLE)
	}
	fn rw_wake(_: *const ()) {
	}
	fn rw_wake_by_ref(_: *const ()) {
	}
	fn rw_drop(_: *const ()) {
	}
	static VTABLE: task::RawWakerVTable = task::RawWakerVTable::new(
		rw_clone, rw_wake, rw_wake_by_ref, rw_drop,
		);
	// SAFE: This waker does nothing
	unsafe {
		task::Waker::from_raw(rw_clone(1 as *const ()))
	}
}

/// Simple async task executor
pub fn runner<T>(mut f: impl FnMut(&mut task::Context)->Option<T>) -> T
{
	let waiter = SimpleWaiter::new();

	// SAFE: The inner waker above won't move
	let waker = unsafe { task::Waker::from_raw(waiter.raw_waker()) };
	let mut context = task::Context::from_waker(&waker);

	loop
	{
		if let Some(rv) = f(&mut context) {
			return rv;
		}
		waiter.sleep();
	}
}
