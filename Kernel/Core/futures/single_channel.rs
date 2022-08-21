//!
//! A "channel" that stores a single item, and can be waited upon by one waiter
//! 
use ::core::task;
use ::core::future::Future;
use ::core::pin::Pin;

/// A slot that can store on value and be waited upon on by one waiter
#[derive(Default)]
pub struct SingleChannel<T>
{
    inner: crate::sync::Spinlock<Inner<T>>,
}
impl<T> SingleChannel<T>
{
    pub const fn new() -> Self {
        SingleChannel { inner: crate::sync::Spinlock::new(Inner { data: None, waiter: None }) }
    }
    /// Clear the contained data
    #[cfg(false_)]
    pub fn clear(&self) -> Option<T> {
        let mut lh = self.inner.lock();
        lh.data = None;
    }
    /// 
	pub fn wait(&self) -> impl Future<Output=T> + '_ {
		struct Waiter<'a, T: 'a> {
			flag: &'a SingleChannel<T>,
		}
		impl<'a, T> Future for Waiter<'a, T> {
			type Output = T;
			fn poll(self: Pin<&mut Self>, cx: &mut task::Context) -> task::Poll<T> {
                let mut lh = self.flag.inner.lock();
                if let Some(v) = lh.data.take() {
					task::Poll::Ready( v )
                }
                else {
					match lh.waiter
					{
					None => { lh.waiter = Some(cx.waker().clone()); }
					Some(ref w) if cx.waker().will_wake(w) => {},
					Some(_) => todo!("Multiple tasks waiting on the same SingleFlag"),
					}
					task::Poll::Pending
				}
			}
		}
		Waiter {
			flag: self,
        }
    }
	/// Store an item
	pub fn store(&self, v: T)
	{
        let mut lh = self.inner.lock();
        lh.data = Some(v);
		lh.waiter.take().map(|r| r.wake());
    }
}
struct Inner<T>
{
    data: Option<T>,
    waiter: Option<task::Waker>,
}
impl<T> Default for Inner<T> {
    fn default() -> Self {
        Inner { data: None, waiter: None }
    }
}

