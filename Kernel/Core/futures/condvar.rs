
use ::core::task;
use ::core::sync::atomic::Ordering;
use super::helpers::WakerQueue;

/// An async condvar-alike, used to wait for an external event
#[derive(Default)]
pub struct Condvar
{
	key: ::core::sync::atomic::AtomicUsize,
	waiters: crate::sync::mutex::Mutex< WakerQueue >,
}
pub struct Key(usize);

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
	
	/// Create a waiter for this type
	///
	/// The passed handler is called with None to poll the state.
	pub fn wait(&self, key: Key) -> impl ::core::future::Future<Output=()> + '_ {
		struct Waiter<'a>(&'a Condvar, usize);
		impl<'a> ::core::future::Future for Waiter<'a> {
			type Output = ();
			fn poll(self: ::core::pin::Pin<&mut Self>, cx: &mut task::Context) -> task::Poll<()> {
				let cv = self.0;
				if self.1 != cv.key.load(Ordering::Relaxed) {
					task::Poll::Ready(())
				}
				else {
					cv.waiters.lock().push(cx.waker());
					task::Poll::Pending
				}
			}
		}
		Waiter(self, key.0)
	}

	/// Obtain the current internal "key" (a counter incremented on every wake call)
	pub fn get_key(&self) -> Key {
		Key(self.key.load(Ordering::SeqCst))
	}

	/// Wake a single waiting thread
	pub fn wake_one(&self) -> bool
	{
		self.key.fetch_add(1, Ordering::SeqCst);

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
		self.key.fetch_add(1, Ordering::SeqCst);

		let mut lh = self.waiters.lock();
		while let Some(waiter) = lh.pop() {
			waiter.wake();
		}
	}
}
