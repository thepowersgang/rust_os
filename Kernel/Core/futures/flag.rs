//!
//! 
//! 
use ::core::task;
use ::core::pin::Pin;
use ::core::future::Future;
use ::core::sync::atomic::{AtomicBool,Ordering};

/// An async boolean that is only waitable by a single task
pub struct SingleFlag
{   
	flag: AtomicBool,
    // TODO: Use a wait queue structure instead? Or a general async condvar
	waiter: crate::sync::mutex::Mutex<Option< task::Waker >>
}

impl SingleFlag
{
	/// Create a new event source
	pub const fn new() -> SingleFlag
	{
		SingleFlag {
			flag: AtomicBool::new(false),
			waiter: crate::sync::mutex::Mutex::new(None),
		}
	}
	/// Return a wait handle for this event source
	pub fn wait(&self) -> impl Future<Output=()> + '_
	{
        struct Waiter<'a> {
            flag: &'a SingleFlag,
        }
        impl<'a> Future for Waiter<'a> {
            type Output = ();
            fn poll(self: Pin<&mut Self>, cx: &mut task::Context) -> task::Poll<()> {
                let flag = self.into_ref().get_ref().flag;
                if flag.flag.swap(false, Ordering::Relaxed) {
                    task::Poll::Ready( () )
                }
                else {
                    let mut lh = flag.waiter.lock();
                    match *lh
                    {
                    None => { *lh = Some(cx.waker().clone()); }
                    Some(ref w) if cx.waker().will_wake(w) => {},
                    Some(_) => todo!("Multiple tasks waiting on the same SingleWaiter"),
                    }
                    task::Poll::Pending
                }
            }
        }
        Waiter {
            flag: self,
            }
	}
	/// Raise the event (waking any attached waiter)
	pub fn trigger(&self)
	{
		self.flag.store(true, Ordering::SeqCst);	// prevents reodering around this
		self.waiter.lock().take().map(|r| r.wake());
	}
}
