// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/lazy_static.rs
//! 
use ::core::sync::atomic;

/// A lazily initialised value (for `static`s)
pub struct LazyStatic<T>(atomic::AtomicU8, ::core::cell::UnsafeCell<::core::mem::MaybeUninit<T>>);
unsafe impl<T: Send+Sync> Sync for LazyStatic<T> {}	// Barring the unsafe "prep" call, is Sync
unsafe impl<T: Send+Sync> Send for LazyStatic<T> {}	// Sendable because inner is sendable

#[repr(u8)]
enum State {
	Uninit,
	Initialising,
	Init,
}

#[macro_export]
macro_rules! lazystatic_init {
	() => ( $crate::lib::LazyStatic::new() );
}

impl<T> LazyStatic<T>
{
	pub const fn new() -> Self {
		LazyStatic(atomic::AtomicU8::new(State::Uninit as u8), ::core::cell::UnsafeCell::new(::core::mem::MaybeUninit::uninit()) )
	}
}
	
impl<T: Send+Sync> LazyStatic<T>
{
	/// Prepare the value using the passed function, panics if a race occurs and returns without doing anything if the value is already initialised
	pub fn prep<Fcn: FnOnce()->T>(&self, fcn: Fcn) -> &T
	{
		match self.0.compare_exchange(State::Uninit as u8, State::Initialising as u8, atomic::Ordering::SeqCst, atomic::Ordering::SeqCst)
		{
		Ok(_) => {
			// SAFE: Protected by atomic wrapper flag
			unsafe {
				::core::ptr::write( (*self.1.get()).as_mut_ptr(), fcn() );
			}
			self.0.compare_exchange(State::Initialising as u8, State::Init as u8, atomic::Ordering::SeqCst, atomic::Ordering::SeqCst).expect("BUG: LazyStatic state error");
			}
		Err(s) if s == State::Init as u8 => {},
		Err(_) => panic!("Racy initialisation of LazyStatic<{}>", type_name!(T)),
		}
		// SAFE: Reports as initailised
		unsafe { &*self.get() }
	}
	/// Returns true if the static has been initialised
	pub fn ls_is_valid(&self) -> bool {
		self.0.load(atomic::Ordering::SeqCst) == State::Init as u8
	}
	/// (unsafe) Obtain a mutable reference to the interior
	pub unsafe fn ls_unsafe_mut(&self) -> &mut T {
		&mut *self.get()
	}

	fn get(&self) -> *mut T {
		assert_eq!(self.0.load(atomic::Ordering::SeqCst), State::Init as u8, "Dereferencing LazyStatic<{}> without initialising", type_name!(T));
		// SAFE: Usage of UnsafeCell protected by atomic
		unsafe { (*self.1.get()).as_mut_ptr() }
	}
}
impl<T: Send+Sync> ::core::ops::Deref for LazyStatic<T>
{
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: No aliasing possible without calling an `unsafe` function incorrectly
		unsafe { & *self.get() }
	}
}

