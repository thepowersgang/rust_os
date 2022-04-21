/*
 */
///! A pool of descriptors in DMA-able memory
use crate::prelude::*;
use crate::memory::virt::ArrayHandle;
use crate::lib::POD;

pub struct DescriptorPool<T: POD>
{
	items_handle: ArrayHandle<T>,
	used_state: Vec<bool>,
}

pub struct LentDescriptor<T: POD>
{
	pool: *const DescriptorPool<T>,
	ptr: *mut T,
}

impl<T: POD> DescriptorPool<T>
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

impl<T: POD> LentDescriptor<T>
{
	pub fn phys(&self) -> crate::arch::memory::PAddr {
		crate::memory::virt::get_phys( &*self )
	}
}

impl<T: POD> ::core::ops::Drop for LentDescriptor<T>
{
	fn drop(&mut self)
	{
		todo!("LentDescriptor::drop - pool={:p}", self.pool)
	}
}

impl<T: POD> ::core::ops::Deref for LentDescriptor<T>
{
	type Target = T;
	
	fn deref(&self) -> &T
	{
		// SAFE: We "own" that pointer (... hardware might dick with it though?)
		unsafe { &*self.ptr }
	}
}

impl<T: POD> ::core::ops::DerefMut for LentDescriptor<T>
{
	fn deref_mut(&mut self) -> &mut T
	{
		// SAFE: We "own" that pointer (... hardware might dick with it though?)
		unsafe { &mut *self.ptr }
	}
}

