//! Collection traits

pub mod vec_deque;
pub mod vec_map;

pub use self::vec_deque::VecDeque;
pub use self::vec_map::VecMap;
	
/// A mutable sequence
pub trait MutableSeq<T>
{
	fn push(&mut self, t: T);
	fn pop(&mut self) -> ::core::option::Option<T>;
}

