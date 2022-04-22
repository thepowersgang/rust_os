
use ::core::task;
use ::core::sync::atomic::Ordering;
use super::helpers::WakerQueue;

/// An async wait queue
///
/// Allows a list of threads to wait on a single object (e.g. a Mutex)
#[derive(Default)]
pub struct Condvar
{
	key: ::core::sync::atomic::AtomicUsize,
	waiters: crate::sync::mutex::Mutex< WakerQueue >,
}

impl Condvar
{
	/// Create a new queue source
	pub const fn new() -> Condvar
	{
		Condvar {
			key: ::core::sync::atomic::AtomicUsize::new(0),
			waiters: crate::sync::mutex::Mutex::new(WakerQueue::new()),
		}
	}
	
	/// Create a waiter for this queue
	///
	/// The passed handler is called with None to poll the state.
	// TODO: Race conditions between 'Source::wait_on' and 'wait_on_list'.
	pub fn wait(&self) -> impl ::core::future::Future<Output=()> + '_ {
        struct Waiter<'a>(&'a Condvar, usize);
        impl<'a> ::core::future::Future for Waiter<'a> {
            type Output = ();
            fn poll(self: ::core::pin::Pin<&mut Self>, cx: &mut task::Context) -> task::Poll<()> {
				let s = self.into_ref().get_ref();
                let cv = s.0;
				if s.1 != cv.key.load(Ordering::Relaxed) {
					task::Poll::Ready(())
				}
				else {
					cv.waiters.lock().push(cx.waker());
					task::Poll::Pending
				}
            }
        }
        Waiter(self, self.key.load(Ordering::SeqCst))
	}

	/// Wake a single waiting thread
	pub fn wake_one(&self) -> bool
	{
		let mut lh = self.waiters.lock();
		if let Some(waiter) = lh.pop() {
			waiter.wake();
			true
		}
		else {
			false
		}
	}

	/// Wake all waiting threads
	pub fn wake_all(&self)
	{
		let mut lh = self.waiters.lock();
		while let Some(waiter) = lh.pop() {
			waiter.wake();
		}
	}
}