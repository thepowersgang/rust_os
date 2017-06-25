//! General numeric helpers
use core::ops;

pub trait Int
where
	Self: ops::Add<Output=Self>,
	Self: ops::Sub<Output=Self>,
	Self: ops::Mul<Output=Self>,
	Self: ops::Div<Output=Self>,
	Self: ops::Rem<Output=Self>,
	Self: Sized
{
	fn one() -> Self;
}
impl Int for u64 {
	fn one() -> Self { 1 }
}
impl Int for u32 {
	fn one() -> Self { 1 }
}
impl Int for usize {
	fn one() -> Self { 1 }
}

/// Round the passed value up to a multiple of the target value
pub fn round_up<T: Int+Copy>(val: T, target: T) -> T
{
	return (val + target - Int::one()) / target * target;
}
/// Divide `num` by `den`, rounding up
pub fn div_up<T: Int+Copy>(num: T, den: T) -> T
{
	return (num + den - Int::one()) / den;
}
/// Divide+Remainder `num` by `den`
pub fn div_rem<T: Int+Copy>(num: T, den: T) -> (T,T)
{
	return (num / den, num % den);
}

/// Absolute difference between two numbers
pub fn abs_diff<T: PartialOrd + ops::Sub>(a: T, b: T) -> T::Output {
	return if a > b { a - b } else { b - a };
}

