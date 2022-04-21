// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/heap.rs
//! Dynamic memory manager
use core::ptr::NonNull;

// TODO: Rewrite this to correctly use the size information avaliable

use self::heapdef::HeapDef;

mod heapdef;

// --------------------------------------------------------
// Types
#[derive(Copy,Clone)]
pub enum HeapId
{
	Local,	// Inaccessible outside of process
	Global,	// Global allocations
}

#[derive(Debug)]
pub enum Error
{
	Corrupted,
	OutOfReservation,
	OutOfMemory,
}

//pub struct AnyAlloc
//{
//	ptr: NonNull<()>,
//}
//pub struct TypedAlloc<T>
//{
//	ptr: NonNull<T>,
//}
pub struct ArrayAlloc<T>
{
	ptr: NonNull<T>,
	count: usize,
}
impl<T> !crate::lib::POD for ArrayAlloc<T> {}
unsafe impl<T: Sync> Sync for ArrayAlloc<T> {
}
unsafe impl<T: Send> Send for ArrayAlloc<T> {
}

// --------------------------------------------------------

pub const ZERO_ALLOC: *mut () = 1 as *mut _;

//static S_LOCAL_HEAP: crate::sync::Mutex<HeapDef> = mutex_init!(HeapDef{head:None});
static S_GLOBAL_HEAP: crate::sync::Mutex<HeapDef> = crate::sync::Mutex::new(HeapDef::new());

// --------------------------------------------------------
// Code
pub fn init()
{
}

#[cfg(all(not(test),not(test_shim)))]
mod _allocator {
	#[global_allocator]
	static GLOBAL_ALLOC: Allocator = Allocator;

	struct Allocator;
	unsafe impl ::alloc::alloc::GlobalAlloc for Allocator {
		unsafe fn alloc(&self, layout: ::alloc::alloc::Layout) -> *mut u8 {
			match super::allocate(super::HeapId::Global, layout.size(), layout.align())
			{
			Some(x) => x as *mut u8,
			None => ::core::ptr::null_mut(),
			}
		}
	    unsafe fn dealloc(&self, ptr: *mut u8, layout: ::alloc::alloc::Layout) {
			super::S_GLOBAL_HEAP.lock().deallocate(ptr as *mut (), layout.size(), layout.align());
		}
	}

	#[alloc_error_handler]
	fn error_handler(layout: core::alloc::Layout) -> ! {
		panic!("Alloc error: {:?}", layout);
	}
}

// Used by libgcc and ACPICA
#[cfg(all(not(test),not(test_shim)))]
#[no_mangle] pub unsafe extern "C" fn malloc(size: usize) -> *mut () {
	allocate(HeapId::Global, size, 16).unwrap()
} 
#[cfg(all(not(test),not(test_shim)))]
#[no_mangle] pub unsafe extern "C" fn free(ptr: *mut ()) {
	if !ptr.is_null() {
		deallocate(ptr, 0, 16)
	}
} 

// Used by kernel internals
pub unsafe fn alloc<T>(value: T) -> *mut T
{
	let ret = match allocate(HeapId::Global, ::core::mem::size_of::<T>(), ::core::mem::align_of::<T>())
		{
		Some(v) => v as *mut T,
		None => panic!("Out of memory")
		};
	::core::ptr::write(ret, value);
	ret
}
pub unsafe fn alloc_raw(size: usize, align: usize) -> *mut () {
	match allocate(HeapId::Global, size, align)
	{
	Some(v) => v,
	None => panic!("Out of memory")
	}
}
pub unsafe fn dealloc<T>(value: *mut T)
{
	deallocate(value as *mut (), ::core::mem::size_of::<T>(), ::core::mem::align_of::<T>());
}
pub unsafe fn dealloc_raw(ptr: *mut (), size: usize, align: usize) {
	deallocate(ptr, size, align);
}

impl<T> ArrayAlloc<T>
{
	/// Create a new empty array allocation (const)
	/// - NOTE: Zero count, even when the type is zero-sized
	pub const fn empty() -> ArrayAlloc<T> {
		ArrayAlloc {
			// TODO: When NonNull::empty is const, use that
			// SAFE: Non-zero value
			ptr: unsafe { NonNull::new_unchecked(ZERO_ALLOC as *mut T) },
			count: 0
			}
	}
	
	/// Create a new array allocation with `count` items
	pub fn new(count: usize) -> ArrayAlloc<T>
	{
		// SAFE: Correctly constructs 'NonNull' instances
		unsafe {
			if ::core::mem::size_of::<T>() == 0 {
				ArrayAlloc { ptr: NonNull::new_unchecked(ZERO_ALLOC as *mut T), count: !0 }
			}
			else if count == 0 {
				ArrayAlloc { ptr: NonNull::new_unchecked(ZERO_ALLOC as *mut T), count: 0 }
			}
			else
			{
				let ptr = match allocate(HeapId::Global, ::core::mem::size_of::<T>() * count, ::core::mem::align_of::<T>())
					{
					Some(v) => v as *mut T,
					None => panic!("Out of memory when allocating array of {} elements", count)
					};
				assert!(!ptr.is_null());
				ArrayAlloc { ptr: NonNull::new_unchecked(ptr), count: count }
			}
		}
	}
	pub unsafe fn from_raw(ptr: *mut T, count: usize) -> ArrayAlloc<T> {
		ArrayAlloc { ptr: NonNull::new_unchecked(ptr), count: count }
	}
	pub fn into_raw(self) -> *mut [T] {
		let ptr = self.ptr.as_ptr();
		let count = self.count;
		::core::mem::forget(self);
		// SAFE: Takes ownership
		unsafe {
			::core::slice::from_raw_parts_mut(ptr, count)
		}
	}
	
	pub fn count(&self) -> usize { self.count }
	
	pub fn get_base(&self) -> *const T { self.ptr.as_ptr() }
	pub fn get_base_mut(&mut self) -> *mut T { self.ptr.as_ptr() }
	
	//#[is_safe(irq)]
	pub fn get_ptr_mut(&mut self, idx: usize) -> *mut T {
		// SAFE: Index asserted to be valid, have &mut
		unsafe {
			assert!(idx < self.count, "ArrayAlloc<{}>::get_mut({}) OOB {}", type_name!(T), idx, self.count);
			self.ptr.as_ptr().offset(idx as isize)
		}
	}
	//#[is_safe(irq)]
	pub fn get_ptr(&self, idx: usize) -> *const T {
		// SAFE: Index asserted to be valid
		unsafe {
			assert!(idx < self.count, "ArrayAlloc<{}>::get_ptr({}) OOB {}", type_name!(T), idx, self.count);
			self.ptr.as_ptr().offset(idx as isize)
		}
	}

	/// Attempt to expand this array without reallocating	
	pub fn expand(&mut self, new_count: usize) -> bool
	{
		if new_count > self.count
		{
			let newsize = ::core::mem::size_of::<T>() * new_count;
			// SAFE: Pointer is valid
			if unsafe { expand( self.ptr.as_ptr() as *mut (), newsize ) }
			{
				self.count = new_count;
				true
			}
			else
			{
				false
			}
		}
		else
		{
			log_warning!("ArrayAlloc<{}>::expand: Called with <= count", type_name!(T));
			true
		}
	}
	
	pub fn shrink(&mut self, new_count: usize)
	{
		if new_count == self.count
		{
			// Nothing to do
		}
		else if new_count > self.count
		{
			log_warning!("ArrayAlloc::<{}>::shrink - Called with > count", type_name!(T));
		}
		else
		{
			let newsize = ::core::mem::size_of::<T>() * new_count;
			// SAFE: Pointer is valid, and raw pointer is being manipulated (lifetimes up to the caller)
			unsafe { shrink(self.ptr.as_ptr() as *mut (), newsize) };
			self.count = new_count;
		}
	}
}
impl_fmt!{
	<T> Debug(self,f) for ArrayAlloc<T> {
		write!(f, "ArrayAlloc {{ {:p} + {} }}", self.ptr.as_ptr(), self.count)
	}
}
impl<T> ::core::ops::Drop for ArrayAlloc<T>
{
	fn drop(&mut self)
	{
		if self.count > 0 {
			// SAFE: Pointer is valid
			unsafe { deallocate(self.ptr.as_ptr() as *mut (), ::core::mem::size_of::<T>() * self.count, ::core::mem::align_of::<T>()) };
		}
	}
}

// Main entrypoints
/// Allocate memory from the specified heap
unsafe fn allocate(heap: HeapId, size: usize, align: usize) -> Option<*mut ()>
{
	match heap
	{
	HeapId::Global => match S_GLOBAL_HEAP.lock().allocate(size, align)
		{
		Ok(v) => Some(v),
		Err(e) => {
			log_error!("Unable to allocate: {:?}", e);
			None
			},
		},
	_ => panic!("TODO: Non-global heaps"),
	}
}

/// Attempt to expand in-place
unsafe fn expand(pointer: *mut (), newsize: usize) -> bool
{
	S_GLOBAL_HEAP.lock().expand_alloc(pointer, newsize)
}
unsafe fn shrink(pointer: *mut (), newsize: usize)
{
	S_GLOBAL_HEAP.lock().shrink_alloc(pointer, newsize)
}

unsafe fn deallocate(pointer: *mut (), size: usize, align: usize)
{
	S_GLOBAL_HEAP.lock().deallocate(pointer as *mut (), size, align);
}


// vim: ft=rust
