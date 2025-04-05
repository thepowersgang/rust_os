
pub trait Args {
	const CALL: u32;
	type Tuple;
	fn from_tuple(t: Self::Tuple) -> Self;
	fn into_tuple(self) -> Self::Tuple;
}
pub trait ToUsizeArray {
	const LEN: usize;
	fn into_array(self) -> [usize; Self::LEN];
}
impl ToUsizeArray for usize { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self] } }
impl ToUsizeArray for u32 { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self as _] } }
impl ToUsizeArray for u16 { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self as _] } }
impl ToUsizeArray for u8  { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self as _] } }
impl ToUsizeArray for bool { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self as _] } }
impl<'a, T: 'static> ToUsizeArray for &'a T     { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self as *const _ as usize] } }
impl<'a, T: 'static> ToUsizeArray for &'a mut T { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self as *const _ as usize] } }
impl<'a, T: 'static> ToUsizeArray for &'a [T]     { const LEN: usize = 2; fn into_array(self) -> [usize; 2] { [self.as_ptr() as usize, self.len()] } }
impl<'a, T: 'static> ToUsizeArray for &'a mut [T] { const LEN: usize = 2; fn into_array(self) -> [usize; 2] { [self.as_ptr() as usize, self.len()] } }
impl<'a> ToUsizeArray for &'a str { const LEN: usize = 2; fn into_array(self) -> [usize; 2] { [self.as_ptr() as usize, self.len()] } }
impl ToUsizeArray for super::FixedStr8 { const LEN: usize = u64::LEN; fn into_array(self) -> [usize; Self::LEN] { <u64 as ToUsizeArray>::into_array(self.into()) } }
#[cfg(target_pointer_width="64")]
impl ToUsizeArray for u64 { const LEN: usize = 1; fn into_array(self) -> [usize; 1] { [self as _] } }
#[cfg(target_pointer_width="32")]
impl ToUsizeArray for u64 {
	const LEN: usize = 2;
	fn into_array(self) -> [usize; 2] {
		[(self & 0xFFFFFFFF) as usize, (self >> 32) as usize ]
	}
}