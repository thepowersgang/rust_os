

/// A wrapper around a future that adds a callback that is called if the future is dropped before completion
pub struct DropWrapper<F, D>
where
	D: FnOnce(),
{
	future: F,
	dropstate: Option<D>,
}
impl<F, D> ::core::future::Future for DropWrapper<F,D>
where
	F: ::core::future::Future,
	D: FnOnce(),
{
	type Output = F::Output;
	fn poll(mut self: ::core::pin::Pin<&mut Self>, c: &mut ::core::task::Context) -> ::core::task::Poll<Self::Output> {
		// SAFE: Correct usage of `Pin`
		let (inner, dropstate) = unsafe {
			let s = self.as_mut().get_unchecked_mut();
			(::core::pin::Pin::new_unchecked(&mut s.future), &mut s.dropstate)
			};
		match inner.poll(c)
		{
		v @ ::core::task::Poll::Ready(_) => { *dropstate = None; v },
		v @ ::core::task::Poll::Pending => v,
		}
	}
}
impl<F, D> ::core::ops::Drop for DropWrapper<F,D>
where
	D: FnOnce(),
{
	fn drop(&mut self) {
		if let Some(cb) = self.dropstate.take() {
			cb();
		}
	}
}

/// A wrapper around a future that adds a callback that is called if the future is dropped before completion
pub fn drop_wrapper<F, D>(future: F, cb: D) -> DropWrapper<F, D>
where
	F: ::core::future::Future,
	D: FnOnce(),
{
	DropWrapper { future, dropstate: Some(cb), }
}
