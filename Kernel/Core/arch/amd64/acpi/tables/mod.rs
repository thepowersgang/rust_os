
pub mod madt;

pub use self::madt::Madt;

pub trait Table: crate::lib::POD
{
	type Iter<'a>: Iterator where Self: 'a;
	fn iterate_subitems<'s>(&'s self, trailing_data: &'s [u8]) -> Self::Iter<'s>;
}
