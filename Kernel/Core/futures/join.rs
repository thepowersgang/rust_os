///! Future wrappers that wait on multiple futures at the same time

use ::core::task;
use ::core::pin::Pin;
use ::core::future::Future;

/// A future wrapper that waits on both futures and returns the first future to complete
/// 
/// Once complete, the other future will be dropped.
pub struct JoinOne<F1, F2> {
	f1: Option<F1>,
	f2: Option<F2>,
}
impl<F1, F2> JoinOne<F1, F2> {
	pub fn new(f1: F1, f2: F2) -> Self {
		JoinOne {
			f1: Some(f1),
			f2: Some(f2),
		}
	}

	/// Get the non-completed future (still pinned)
	pub fn get_unfinished(self: Pin<&mut Self>) -> Option<JoinOneRes<Pin<&mut F1>, Pin<&mut F2>>> {
		// SAFE: Pinning is maintained
		unsafe {
			if self.f1.is_some() && self.f1.is_some() {
				None
			}
			else if self.f1.is_some() {
				Some(JoinOneRes::One( self.map_unchecked_mut(|v| v.f1.as_mut().unwrap()) ) )
			}
			else {
				Some(JoinOneRes::Two( self.map_unchecked_mut(|v| v.f2.as_mut().unwrap()) ))
			}
		}
	}
}
impl<F1,F2> Future for JoinOne<F1,F2>
where
	F1: Future,
	F2: Future,
{
	type Output = JoinOneRes<F1::Output,F2::Output>;
	fn poll(self: Pin<&mut Self>, cx: &mut task::Context) -> task::Poll<Self::Output> {
		// SAFE: No item will be moved (just either replaced or re-pinned)
		let s = unsafe { self.get_unchecked_mut() };
		match (&s.f1, &s.f2) {
		(Some(_), Some(_)) => {},
		_ => panic!("Being polled after completion"),
		}
		// SAFE: Item won't be moved
		match unsafe { Pin::new_unchecked(s.f1.as_mut().unwrap()) }.poll(cx)
		{
		task::Poll::Ready(rv) => { s.f1 = None; return task::Poll::Ready(JoinOneRes::One(rv)) },
		task::Poll::Pending => {},
		}
		// SAFE: Item won't be moved
		match unsafe { Pin::new_unchecked(s.f2.as_mut().unwrap()) }.poll(cx)
		{
		task::Poll::Ready(rv) => { s.f2 = None; return task::Poll::Ready(JoinOneRes::Two(rv)) },
		task::Poll::Pending => {},
		}
		task::Poll::Pending
	}
}

/// Result type for a [JoinOne] future wrapper
pub enum JoinOneRes<F1,F2> {
	/// The first future completed
	One(F1),
	/// The second future completed
	Two(F2),
}


pub struct JoinBoth<F1, F2>
where
	F1: Future,
	F2: Future,
{
	f1: JoinBothInner<F1>,
	f2: JoinBothInner<F2>,
}

enum JoinBothInner<F: Future> {
	Active(F),
	Done(Option<F::Output>),
}
impl<F1, F2> JoinBoth<F1, F2>
where
	F1: Future,
	F2: Future,
{
	pub fn new(f1: F1, f2: F2) -> Self {
		JoinBoth {
			f1: JoinBothInner::Active(f1),
			f2: JoinBothInner::Active(f2,)
		}
	}
}
impl<F1,F2> Future for JoinBoth<F1,F2>
where
	F1: Future,
	F2: Future,
{
	type Output = (F1::Output,F2::Output);
	fn poll(self: Pin<&mut Self>, cx: &mut task::Context) -> task::Poll<Self::Output> {
		// SAFE: No item will be moved (just either replaced or re-pinned)
		let s = unsafe { self.get_unchecked_mut() };
		match s.f1
		{
		JoinBothInner::Active(ref mut f) =>
			// SAFE: Pointer is from a pinned location
			match unsafe { Pin::new_unchecked(f) }.poll(cx)
			{
			task::Poll::Ready(rv) => { s.f1 = JoinBothInner::Done(Some(rv)); },
			task::Poll::Pending => {},
			},
		JoinBothInner::Done(_) => {},
		}
		
		match s.f2
		{
		JoinBothInner::Active(ref mut f) =>
			// SAFE: Pointer is from a pinned location
			match unsafe { Pin::new_unchecked(f) }.poll(cx)
			{
			task::Poll::Ready(rv) => { s.f2 = JoinBothInner::Done(Some(rv)); },
			task::Poll::Pending => {},
			},
		JoinBothInner::Done(_) => {},
		}

		match (&mut s.f1, &mut s.f2)
		{
		( JoinBothInner::Done(v1), JoinBothInner::Done(v2) ) =>
			match (v1.take(), v2.take())
			{
			(Some(v1), Some(v2)) => task::Poll::Ready( (v1, v2) ),
			(None, None) => panic!("Polled after completion"),
			_ => panic!("Inconsistent"),
			}
		_ => task::Poll::Pending,
		}
	}
}
