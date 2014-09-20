//
//
//

pub trait Clone
{
	fn clone(&self) -> Self;
}

macro_rules! impl_clone { ($t:ty, $val:expr) => (impl Clone for $t { fn clone(&self) -> $t { $val(self) }}) }
macro_rules! impl_clone_copy { ($t:ty) => (impl_clone!($t, |s:&_| *s))}

impl_clone_copy!(int)  impl_clone_copy!(i8) impl_clone_copy!(i16) impl_clone_copy!(i32) impl_clone_copy!(i64)
impl_clone_copy!(uint) impl_clone_copy!(u8) impl_clone_copy!(u16) impl_clone_copy!(u32) impl_clone_copy!(u64)
impl_clone_copy!(bool) impl_clone_copy!(char)
impl_clone!( (), |_| () )

// vim: ft=rust

