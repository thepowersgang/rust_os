//! libuefi clone of the ::std::borrow module

pub enum Cow<'heap, 'd, T: ?Sized>
where
	T: 'd + ToOwned<'heap>
{
	Owned(T::Owned),
	Borrowed(&'d T)
}
impl<'bs, 'd, T: ?Sized> From<&'d T> for Cow<'bs, 'd, T>
where
	T: 'd + ToOwned<'bs>
{
	fn from(v: &'d T) -> Self {
		Cow::Borrowed(v)
	}
}
impl<'bs, 'd, T: ?Sized> ::core::ops::Deref for Cow<'bs, 'd, T>
where
	T: 'd + ToOwned<'bs>
{
	type Target = T;
	fn deref(&self) -> &T {
		match self
		{
		&Cow::Owned(ref v) => v.borrow(),
		&Cow::Borrowed(v) => v,
		}
	}
}
impl<'bs, 'd, T: ?Sized> ::core::fmt::Display for Cow<'bs, 'd, T>
where
	T: 'd + ToOwned<'bs> + ::core::fmt::Display
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		(**self).fmt(f)
	}
}

pub trait Borrow<T: ?Sized>
{
	fn borrow(&self) -> &T;
}

pub trait ToOwned<'heap>
{
	type Owned: 'heap + Borrow<Self>;
	fn to_owned(&self, &'heap ::boot_services::BootServices) -> Self::Owned;
}
