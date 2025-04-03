
/// Trait implemented on tuples that allows calling a syscall using that tuple to provide arguments
pub(crate) trait CallTuple {
	unsafe fn call(self, id: u32) -> u64;
}

trait ToUsizeArray {
	const LEN: usize;
	fn into_array(self) -> [usize; Self::LEN];
}
//pub trait EnumTrait {
//	fn into_usize(self) -> usize;
//}
impl ToUsizeArray for usize { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self] } }
impl ToUsizeArray for u32 { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self as _] } }
impl ToUsizeArray for u16 { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self as _] } }
impl ToUsizeArray for u8  { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self as _] } }
impl<'a, T: 'static> ToUsizeArray for &'a T     { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self as *const _ as usize] } }
impl<'a, T: 'static> ToUsizeArray for &'a mut T { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self as *const _ as usize] } }
impl<'a, T: 'static> ToUsizeArray for &'a [T]     { const LEN: usize = 2; fn into_array(self) -> [usize; 2] { [self.as_ptr() as usize, self.len()] } }
impl<'a, T: 'static> ToUsizeArray for &'a mut [T] { const LEN: usize = 2; fn into_array(self) -> [usize; 2] { [self.as_ptr() as usize, self.len()] } }
impl<'a> ToUsizeArray for &'a str { const LEN: usize = 2; fn into_array(self) -> [usize; 2] { [self.as_ptr() as usize, self.len()] } }
impl ToUsizeArray for ::values::FixedStr8 { const LEN: usize = u64::LEN; fn into_array(self) -> [usize; Self::LEN] { <u64 as ToUsizeArray>::into_array(self.into()) } }
//impl<T: EnumTrait+'static> ToUsizeArray for T { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self.into_usize()] } }

#[cfg(target_pointer_width="64")]
impl ToUsizeArray for u64 { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self as _] } }
#[cfg(target_pointer_width="32")]
impl ToUsizeArray for u64 {
	const LEN: usize = 2;
	fn into_array(self) -> [usize; 2] {
		[(self & 0xFFFFFFFF) as usize, (self >> 32) as usize ]
	}
}

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

fn concat_array<const N1: usize, const N2: usize>(a1: [usize; N1], a2: [usize; N2]) -> [usize; N1 + N2] {
	let mut rv = [0; N1+N2];
	rv[..N1].copy_from_slice(&a1);
	rv[N1..].copy_from_slice(&a2);
	rv
}

/// Empty tuple - No arguments
impl CallTuple for () {
	unsafe fn call(self, id: u32) -> u64 {
		super::raw::syscall_0(id)
	}
}
impl<A0: ToUsizeArray> CallTuple for (A0,)
where
	[usize; A0::LEN]: CallArray
{
	unsafe fn call(self, id: u32) -> u64 {
		let a1 = self.0.into_array();
		CallArray::call(a1, id)
	}
}
impl<A0: ToUsizeArray, A1: ToUsizeArray> CallTuple for (A0,A1,)
where
	[usize; A0::LEN + A1::LEN]: CallArray
{
	unsafe fn call(self, id: u32) -> u64 {
		let a = concat_array(self.0.into_array(), self.1.into_array());
		CallArray::call(a, id)
	}
}
impl<A0: ToUsizeArray, A1: ToUsizeArray, A2: ToUsizeArray> CallTuple for (A0,A1,A2,)
where
	[usize; A0::LEN + A1::LEN + A2::LEN]: CallArray
{
	unsafe fn call(self, id: u32) -> u64 {
		let a = concat_array(self.0.into_array(), self.1.into_array());
		let a = concat_array(a, self.2.into_array());
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
		let a = concat_array(self.0.into_array(), self.1.into_array());
		let a = concat_array(a, self.2.into_array());
		let a = concat_array(a, self.3.into_array());
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
		let a = concat_array(self.0.into_array(), self.1.into_array());
		let a = concat_array(a, self.2.into_array());
		let a = concat_array(a, self.3.into_array());
		let a = concat_array(a, self.4.into_array());
		CallArray::call(a, id)
	}
}
