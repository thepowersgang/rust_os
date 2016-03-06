//
//
//
#![crate_type="rlib"]
#![crate_name="alloc"]
#![feature(lang_items)]	// Allow definition of lang_items
#![feature(const_fn,unsize,coerce_unsized)]
#![feature(unique,nonzero)]
#![feature(box_syntax)]
#![feature(raw)]
#![feature(placement_new_protocol)]
#![feature(optin_builtin_traits)]	// For !Send
#![feature(filling_drop)]	// for RawVec
#![feature(unboxed_closures)]
#![no_std]

#[macro_use]
extern crate syscalls;
#[macro_use]
extern crate macros;

extern crate std_sync as sync;

mod std {
	pub use core::fmt;
	pub use core::iter;
}

pub mod heap;

mod array;
pub mod boxed;

pub mod rc;
pub mod grc;

pub use self::array::ArrayAlloc;


pub fn oom() {
	panic!("Out of memory");
}

pub mod raw_vec {
	pub struct RawVec<T>(::array::ArrayAlloc<T>);
	impl<T> RawVec<T> {
		pub fn new() -> RawVec<T> {
			RawVec( ::array::ArrayAlloc::new(0) )
		}
		pub fn with_capacity(cap: usize) -> RawVec<T> {
			RawVec( ::array::ArrayAlloc::new(cap) )
		}
		pub unsafe fn from_raw_parts(base: *mut T, size: usize) -> RawVec<T> {
			RawVec( ::array::ArrayAlloc::from_raw_parts(base, size) )
		}
		pub fn cap(&self) -> usize {
			self.0.count()
		}
		pub fn ptr(&self) -> *mut T {
			self.0.get_base() as *mut T
		}
		pub fn shrink_to_fit(&mut self, used: usize) {
			self.0.resize(used);
		}
		pub fn reserve(&mut self, cur_used: usize, extra: usize) {
			let newcap = cur_used + extra;
			if newcap < self.cap() {
				
			}
			else {
				self.0.resize(newcap);
			}
		}
		pub fn reserve_exact(&mut self, cur_used: usize, extra: usize) {
			let newcap = cur_used + extra;
			if newcap < self.cap() {
				
			}
			else {
				self.0.resize(newcap);
			}
		}
		pub fn double(&mut self) {
			//kernel_log!("RawVec::<{}>::double()", type_name!(T));
			if self.cap() == 0 {
				self.0.resize(1);
			}
			else {
				let newcap = self.cap() * 2;
				self.0.resize(newcap);
			}
		}
		pub fn into_box(self) -> ::boxed::Box<[T]> {
			todo!("into_box");
		}
		
		pub fn unsafe_no_drop_flag_needs_drop(&self) -> bool {
			self.cap() != ::core::mem::POST_DROP_USIZE
		}
	}
}

