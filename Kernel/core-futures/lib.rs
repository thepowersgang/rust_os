#![no_std]
#![feature(generator_trait)]
#![feature(optin_builtin_traits)]

pub use ::core::*;



pub mod future {
	pub use core::future::*;
	use core::ops::{Generator,GeneratorState};
	use core::task::{Context,Poll};
	use core::pin::Pin;

	// NOTE: A chunk of this code is adapted from https://github.com/rust-lang/rust/blob/717702dffdf9ddb84e1fd35f189511a307e350e1/src/libstd/future.rs

	struct GenFuture<T>(T);

	impl<T: Generator<Yield = ()>> !Unpin for GenFuture<T> {}

	#[doc(hidden)]
	impl<T: Generator<Yield = ()>> Future for GenFuture<T>
	{
		type Output = T::Return;
		fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>
		{
			// Safe because we're !Unpin + !Drop mapping to a ?Unpin value
			let gen = unsafe { Pin::map_unchecked_mut(self, |s| &mut s.0) };
			let _guard = unsafe { set_task_context(cx) };
			match gen.resume( () )
			{
			GeneratorState::Yielded(()) => Poll::Pending,
			GeneratorState::Complete(x) => Poll::Ready(x),
			}
		}
	}

	extern "C" {
		fn set_tls_futures_context(p: *mut u8) -> *mut u8;
	}
	unsafe fn set_task_context<'a>(cx: &'a mut Context<'_>) -> RestoreOnDrop<'a> {
		RestoreOnDrop( set_tls_futures_context(cx as *mut _ as *mut u8),  ::core::marker::PhantomData )
	}
	struct RestoreOnDrop<'a>(*mut u8, ::core::marker::PhantomData<&'a mut Context<'a>>);
	impl<'a> ::core::ops::Drop for RestoreOnDrop<'a> {
		fn drop(&mut self) {
			// SAFE: Safe FFI
			unsafe {
				set_tls_futures_context(self.0);
			}
		}
	}
	

	/// Create a future from a generator
	pub fn from_generator<T>(x: T) -> impl Future<Output = T::Return>
	where
		T: ::core::ops::Generator<Yield = ()>
	{
		GenFuture(x)
	}


	/// Poll a future, using the TLS stored context
	pub fn poll_with_tls_context<F>(f: Pin<&mut F>) -> Poll<F::Output>
	where
	    F: Future,
	{
		// Get the context from the TLS, then poll with it
		// SAFE: Safe FFI, checking for NUL and the pointer is unique
		let (_h, cx) = unsafe {
			let ptr = set_tls_futures_context(::core::ptr::null_mut());
			assert!( !ptr.is_null(), "rustc bug: futures pointer NULL when polled" );
			// Ensure that the pointer is returned to TLS once we're done
			let h = RestoreOnDrop(ptr,  ::core::marker::PhantomData);
			(h, &mut *(ptr as *mut _))
			};
		f.poll(cx)
	}
}
