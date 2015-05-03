/*
 */
///! A pool of descriptors in DMA-able memory
use prelude::*;
use memory::virt::AllocHandle;

pub struct DescriptorPool<T>
{
	items_handle: AllocHandle,
	used_state: Vec<bool>,
	_type: ::core::marker::PhantomData<T>,
}

pub struct LentDescriptor<T>
{
	pool: *const DescriptorPool<T>,
	ptr: *mut T,
}

impl<T> DescriptorPool<T>
{
	pub fn try_pop(&self) -> Option<LentDescriptor<T>>
	{
		unimplemented!();
	}
}

impl<T> LentDescriptor<T>
{
	pub fn phys(&self) -> ::arch::memory::PAddr {
		::memory::virt::get_phys( &*self )
	}
}

impl<T> ::core::ops::Deref for LentDescriptor<T>
{
	type Target = T;
	
	fn deref(&self) -> &T
	{
		// Safe, we "own" that pointer (... hardware might dick with it though?)
		unsafe { &*self.ptr }
	}
}

impl<T> ::core::ops::DerefMut for LentDescriptor<T>
{
	fn deref_mut(&mut self) -> &mut T
	{
		// Safe, we "own" that pointer (... hardware might dick with it though?)
		unsafe { &mut *self.ptr }
	}
}

