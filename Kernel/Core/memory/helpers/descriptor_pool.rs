/*
 */
///! A pool of descriptors in DMA-able memory
use prelude::*;
use memory::virt::ArrayHandle;

pub struct DescriptorPool<T>
{
	items_handle: ArrayHandle<T>,
	used_state: Vec<bool>,
}

pub struct LentDescriptor<T>
{
	pool: *const DescriptorPool<T>,
	ptr: *mut T,
}

impl<T> DescriptorPool<T>
{
	pub fn try_pop(&mut self) -> Option<LentDescriptor<T>>
	{
		if let Some(i) = self.used_state.iter().position(|&a| a == false)
		{
			self.used_state[i] = true;
			Some(LentDescriptor {
					pool: self,
					ptr: &mut self.items_handle[i],
				})
		}
		else {
			None
		}
	}
}

impl<T> LentDescriptor<T>
{
	pub fn phys(&self) -> ::arch::memory::PAddr {
		::memory::virt::get_phys( &*self )
	}
}

impl<T> ::core::ops::Drop for LentDescriptor<T>
{
	fn drop(&mut self)
	{
		todo!("LentDescriptor::drop - pool={:p}", self.pool)
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

