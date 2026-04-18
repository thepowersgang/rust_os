
pub trait Args {
	const CALL: u32;
	type Tuple;
	fn from_tuple(t: Self::Tuple) -> Self;
	fn into_tuple(self) -> Self::Tuple;
}
pub trait ToUsizeArray {
	//const fn len() -> usize;
	const LEN: usize;
	fn write_to_array(self, _: &mut [usize]);
}
pub macro to_usize_array {
	([$($g:tt)*] $s:ident: $t:ty => [$v:expr] ) => {
		impl<$($g)*> ToUsizeArray for $t {
			//const fn len() -> usize { 1 }
			const LEN: usize = 1;
			fn write_to_array($s, dst: &mut [usize]) {
				dst[0] = $v;
			}
		}
	},
	([$($g:tt)*] $s:ident: $t:ty => [$v:expr,$v2:expr] ) => {
		impl<$($g)*> ToUsizeArray for $t {
			//const fn len() -> usize { 2 }
			const LEN: usize = 2;
			fn write_to_array($s, dst: &mut [usize]) {
				dst[0] = $v;
				dst[1] = $v2;
			}
		}
	}
}
to_usize_array! { [] self: usize => [self] }
to_usize_array! { [] self: u32   => [self as _] }
to_usize_array! { [] self: u16   => [self as _] }
to_usize_array! { [] self: u8    => [self as _] }
to_usize_array! { [] self: bool  => [self as _] }
to_usize_array! { ['a, T: 'static] self: &'a     T => [self as *const _ as usize] }
to_usize_array! { ['a, T: 'static] self: &'a mut T => [self as *const _ as usize] }
to_usize_array! { ['a, T: 'static] self: &'a     [T] => [self.as_ptr() as usize, self.len()] }
to_usize_array! { ['a, T: 'static] self: &'a mut [T] => [self.as_ptr() as usize, self.len()] }
to_usize_array! { ['a] self: &'a str => [self.as_ptr() as usize, self.len()] }
#[cfg(target_pointer_width="64")]
to_usize_array! { [] self: u64   => [self as _] }
#[cfg(target_pointer_width="32")]
to_usize_array! { [] self: u64   => [(self & 0xFFFFFFFF) as usize, (self >> 32) as usize] }
impl ToUsizeArray for super::FixedStr8 {
	//const fn len() -> usize { u64::len() }
	const LEN: usize = u64::LEN;
	fn write_to_array(self, dst: &mut [usize]) {
		<u64 as ToUsizeArray>::write_to_array(self.into(), dst)
	}
}

