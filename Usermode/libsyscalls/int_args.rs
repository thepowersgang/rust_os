// cspell:ignore SysCall
use crate::values::ToUsizeArray;

/// Trait implemented on tuples that allows calling a syscall using that tuple to provide arguments
pub(crate) trait CallTuple {
	unsafe fn call(self, id: u32) -> u64;
}


fn push<A: ToUsizeArray>(dst: &mut [usize], v: A) -> &mut [usize] {
	v.write_to_array(dst);
	&mut dst[A::LEN..]
}

/// Empty tuple - No arguments
impl CallTuple for () {
	unsafe fn call(self, id: u32) -> u64 {
		super::raw::syscall_0(id)
	}
}

#[cfg(all())]
mod fallback {
	use super::super::raw;
	use super::{CallTuple,ToUsizeArray,push};

	unsafe fn call_array(a: &[usize], id: u32) -> u64 {
		match a {
		&[] => raw::syscall_0(id),
		&[a1] => raw::syscall_1(id,a1),
		&[a1,a2] => raw::syscall_2(id,a1,a2),
		&[a1,a2,a3] => raw::syscall_3(id,a1,a2,a3),
		&[a1,a2,a3,a4] => raw::syscall_4(id,a1,a2,a3,a4),
		&[a1,a2,a3,a4,a5] => raw::syscall_5(id,a1,a2,a3,a4,a5),
		&[a1,a2,a3,a4,a5,a6] => raw::syscall_6(id,a1,a2,a3,a4,a5,a6),
		_ => unimplemented!(),
		}
	}

	impl<A0: ToUsizeArray> CallTuple for (A0,)
	{
		unsafe fn call(self, id: u32) -> u64 {
			let mut a = [0; 8];
			let an = push(&mut a, self.0);
			let _ = an;
			call_array(&a, id)
		}
	}
	impl<A0: ToUsizeArray, A1: ToUsizeArray> CallTuple for (A0,A1,)
	{
		unsafe fn call(self, id: u32) -> u64 {
			let mut a = [0; 4];
			let an = push(&mut a, self.0);
			let an = push(an, self.1);
			let _ = an;
			call_array(&a, id)
		}
	}
	impl<A0: ToUsizeArray, A1: ToUsizeArray, A2: ToUsizeArray> CallTuple for (A0,A1,A2,)
	{
		unsafe fn call(self, id: u32) -> u64 {
			let mut a = [0; 6];
			let an = push(&mut a, self.0);
			let an = push(an, self.1);
			let an = push(an, self.2);
			let _ = an;
			call_array(&a, id)
		}
	}
	impl<A0, A1, A2, A3> CallTuple for (A0,A1,A2,A3,)
	where
		A0: ToUsizeArray,
		A1: ToUsizeArray,
		A2: ToUsizeArray,
		A3: ToUsizeArray,
	{
		unsafe fn call(self, id: u32) -> u64 {
			let mut a = [0; 8];
			let an = push(&mut a, self.0);
			let an = push(an, self.1);
			let an = push(an, self.2);
			let an = push(an, self.3);
			let _ = an;
			call_array(&a, id)
		}
	}
	impl<A0, A1, A2, A3, A4> CallTuple for (A0,A1,A2,A3,A4,)
	where
		A0: ToUsizeArray,
		A1: ToUsizeArray,
		A2: ToUsizeArray,
		A3: ToUsizeArray,
		A4: ToUsizeArray,
	{
		unsafe fn call(self, id: u32) -> u64 {
			let mut a = [0; 8];
			let an = push(&mut a, self.0);
			let an = push(an, self.1);
			let an = push(an, self.2);
			let an = push(an, self.3);
			let an = push(an, self.4);
			let _ = an;
			call_array(&a, id)
		}
	}
}

#[cfg(any())]
mod const_generic_hacks {

use super::CallTuple;

trait CallArray {
	unsafe fn call(self, id: u32) -> u64;
}
impl CallArray for [usize; 0] {
	unsafe fn call(self, id: u32) -> u64 {
		super::raw::syscall_0(id)
	}
}
impl CallArray for [usize; 1] {
	unsafe fn call(self, id: u32) -> u64 {
		let [a1] = self;
		super::raw::syscall_1(id, a1)
	}
}
impl CallArray for [usize; 2] {
	unsafe fn call(self, id: u32) -> u64 {
		let [a1, a2] = self;
		super::raw::syscall_2(id, a1, a2)
	}
}
impl CallArray for [usize; 3] {
	unsafe fn call(self, id: u32) -> u64 {
		let [a1, a2, a3] = self;
		super::raw::syscall_3(id, a1, a2, a3)
	}
}
impl CallArray for [usize; 4] {
	unsafe fn call(self, id: u32) -> u64 {
		let [a1, a2, a3, a4] = self;
		super::raw::syscall_4(id, a1, a2, a3, a4)
	}
}
impl CallArray for [usize; 5] {
	unsafe fn call(self, id: u32) -> u64 {
		let [a1, a2, a3, a4, a5] = self;
		super::raw::syscall_5(id, a1, a2, a3, a4, a5)
	}
}
impl CallArray for [usize; 6] {
	unsafe fn call(self, id: u32) -> u64 {
		let [a1, a2, a3, a4, a5, a6] = self;
		super::raw::syscall_6(id, a1, a2, a3, a4, a5, a6)
	}
}

impl<A0: ToUsizeArray> CallTuple for (A0,)
where
	[usize; A0::LEN]: CallArray
{
	unsafe fn call(self, id: u32) -> u64 {
		let mut a = [0; A0::LEN];
		let an = push(&mut a, self.0);
		assert!(an.len() == 0);
		CallArray::call(a, id)
	}
}
impl<A0: ToUsizeArray, A1: ToUsizeArray> CallTuple for (A0,A1,)
where
	[usize; A0::LEN + A1::LEN]: CallArray
{
	unsafe fn call(self, id: u32) -> u64 {
		let mut a = [0; A0::LEN + A1::LEN];
		let an = push(&mut a, self.0);
		let an = push(an, self.1);
		assert!(an.len() == 0);
		CallArray::call(a, id)
	}
}
impl<A0: ToUsizeArray, A1: ToUsizeArray, A2: ToUsizeArray> CallTuple for (A0,A1,A2,)
where
	[usize; A0::LEN + A1::LEN + A2::LEN]: CallArray
{
	unsafe fn call(self, id: u32) -> u64 {
		let mut a = [0; A0::LEN + A1::LEN + A2::LEN];
		let an = push(&mut a, self.0);
		let an = push(an, self.1);
		let an = push(an, self.2);
		assert!(an.len() == 0);
		CallArray::call(a, id)
	}
}
impl<A0, A1, A2, A3> CallTuple for (A0,A1,A2,A3,)
where
	A0: ToUsizeArray,
	A1: ToUsizeArray,
	A2: ToUsizeArray,
	A3: ToUsizeArray,
	[usize; A0::LEN + A1::LEN + A2::LEN + A3::LEN]: CallArray
{
	unsafe fn call(self, id: u32) -> u64 {
		let mut a = [0; A0::LEN + A1::LEN + A2::LEN + A3::LEN];
		let an = push(&mut a, self.0);
		let an = push(an, self.1);
		let an = push(an, self.2);
		let an = push(an, self.3);
		assert!(an.len() == 0);
		CallArray::call(a, id)
	}
}
impl<A0, A1, A2, A3, A4> CallTuple for (A0,A1,A2,A3,A4,)
where
	A0: ToUsizeArray,
	A1: ToUsizeArray,
	A2: ToUsizeArray,
	A3: ToUsizeArray,
	A4: ToUsizeArray,
	[usize; A0::LEN + A1::LEN + A2::LEN + A3::LEN + A4::LEN]: CallArray
{
	unsafe fn call(self, id: u32) -> u64 {
		let mut a = [0; A0::LEN + A1::LEN + A2::LEN + A3::LEN + A4::LEN];
		let an = push(&mut a, self.0);
		let an = push(an, self.1);
		let an = push(an, self.2);
		let an = push(an, self.3);
		let an = push(an, self.4);
		assert!(an.len() == 0);
		CallArray::call(a, id)
	}
}

}