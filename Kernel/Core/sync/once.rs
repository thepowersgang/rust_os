
use ::core::sync::atomic::Ordering;

/// A thread-safe OnceCell that does not allow contented access (but only has one byte of overhead)
pub struct OnceCell<T> {
	flag: ::core::sync::atomic::AtomicU8,
	value: ::core::cell::UnsafeCell<::core::mem::MaybeUninit<T>>,
}
unsafe impl<T: Sync> Sync for OnceCell<T> { }
unsafe impl<T: Send> Send for OnceCell<T> { }

const FLAG_UNINIT: u8 = 0;
const FLAG_EDITING: u8 = 1;
const FLAG_INIT: u8 = 2;
impl<T> OnceCell<T> {
	pub const fn new() -> Self {
		OnceCell {
			flag: ::core::sync::atomic::AtomicU8::new(FLAG_UNINIT),
			value: ::core::cell::UnsafeCell::new(::core::mem::MaybeUninit::uninit()),
		}
	}

	pub fn get_init<F>(&self, cb: F) -> &T
	where
		F: FnOnce()->T
	{
		match self.flag.compare_exchange(FLAG_UNINIT, FLAG_EDITING, Ordering::SeqCst, Ordering::SeqCst) {
		Ok(_) => {
			// SAFE: Just transitioned from FLAG_UNINIT to FLAG_EDITING, so can mutate
			let rv: &T = unsafe { (*self.value.get()).write(cb()) };
			self.flag.store(FLAG_INIT, Ordering::SeqCst);
			rv
			},
		// SAFE: Previous flag was `FLAG_INIT`, so the value is initialised
		Err(FLAG_INIT) => unsafe { (*self.value.get()).assume_init_ref() },
		Err(_) => panic!("Contented init of `OnceLock`"),
		}
	}

	pub fn get(&self) -> &T {
		assert!(self.flag.load(Ordering::Acquire) == FLAG_INIT, "Reading from contended or uninitialisee OnceCell");
		// SAFE: The flag is known to be FLAG_INIT
		unsafe {
			(*self.value.get()).assume_init_ref()
		}
	}
}
