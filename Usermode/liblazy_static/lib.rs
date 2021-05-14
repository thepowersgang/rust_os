#![no_std]

extern crate std_sync;

pub struct LazyStatic<T>
{
	data: ::std_sync::RwLock< Option<T> >,
}

impl<T> LazyStatic<T>
{
	pub const fn new() -> LazyStatic<T> {
		LazyStatic {
			data: ::std_sync::RwLock::new( None ),
			}
	}

	pub fn init<F>(&self, f: F)
	where
		F: FnOnce() -> T
	{
		let mut lh = self.data.write();
		assert!(lh.is_none(), "LazyStatic initialised multiple times");
		*lh = Some(f());
	}
}

impl<T> ::core::ops::Deref for LazyStatic<T> {
	type Target = T;
	fn deref(&self) -> &T {
		let ptr: *const T =
			match *self.data.read() {
			Some(ref v) => v,
			None => panic!("LazyStatic used before initialisation"),
			};
		// SAFE: This pointer is valid, and read-only
		unsafe { &*ptr }
	}
}

